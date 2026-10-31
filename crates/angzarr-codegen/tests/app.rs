//! An angzarr-embedded application's startup, generated (`angzarr
//! scaffold app`): the host config and the driver the user's, written
//! once; build.rs and main.rs the CLI's, derived from the protos and the
//! host config on disk. Pinned by golden files (`tests/golden/app`), and
//! the host config's lints against the protos.
//!
//! The host config is angzarr-embedded-host's strict `HostConfig`
//! (angzarr-embedded 5265e5c): `kind` required on every component, every
//! component listed by its name from the protos, unknown keys refused.

mod common;

use angzarr_codegen::{
    AppLint, AppSeed, CodegenError, DomainCrate, Exposure, Owner, RetrySeed, ScaffoldFile, app,
    child_components, lint_app, remote_domains,
};
use common::*;

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/app");

/// The blackjack example: the table aggregate, the player aggregate, and
/// the policies between them (and one to the wallet, outside the app).
const BLACKJACK: &[&str] = &[
    "io/angzarr/examples/v1/table.proto",
    "io/angzarr/examples/v1/player.proto",
    "io/angzarr/examples/v1/policies.proto",
];
const SHOP: &[&str] = &["shop/v1/shop.proto"];

fn seed(name: &str) -> AppSeed {
    AppSeed {
        name: name.to_string(),
        includes: vec!["proto".to_string()],
        languages: Vec::new(),
        expose: Vec::new(),
        retry: RetrySeed::default(),
        in_memory: false,
        angzarr_build: None,
        domain: None,
    }
}

/// What blackjack's e2e and examples-rust's `remote_player` hand-write:
/// the table and its policies in process, the player a Python child, the
/// store in memory.
fn blackjack() -> AppSeed {
    AppSeed {
        languages: vec![("player".to_string(), "python".to_string())],
        in_memory: true,
        ..seed("blackjack")
    }
}

fn file<'a>(files: &'a [ScaffoldFile], path: &str) -> &'a ScaffoldFile {
    files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no {path} among {:?}", paths(files)))
}

fn paths(files: &[ScaffoldFile]) -> Vec<&str> {
    let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    paths.sort_unstable();
    paths
}

fn golden(app: &str, path: &str) -> String {
    std::fs::read_to_string(format!("{GOLDEN}/{app}/{path}")).unwrap()
}

/// Every file of `files` is `app`'s golden (rewritten under
/// `ANGZARR_BLESS`), and owned as the split says: the host config, the
/// manifest and the driver the user's, build.rs and main.rs the CLI's.
fn assert_golden(app: &str, files: &[ScaffoldFile]) {
    assert_eq!(
        paths(files),
        [
            "Cargo.toml",
            "angzarr.toml",
            "build.rs",
            "src/driver.rs",
            "src/main.rs"
        ]
    );
    for f in files {
        let owner = match f.path.as_str() {
            "angzarr.toml" | "Cargo.toml" | "src/driver.rs" => Owner::User,
            _ => Owner::Generated,
        };
        assert_eq!(f.owner, owner, "{}", f.path);
        if std::env::var_os("ANGZARR_BLESS").is_some() {
            let path = format!("{GOLDEN}/{app}/{}", f.path);
            std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
            std::fs::write(&path, &f.source).unwrap();
            continue;
        }
        assert_eq!(f.source, golden(app, &f.path), "{}", f.path);
    }
}

#[test]
fn a_single_aggregate_rust_app_is_its_golden_config_build_script_main_and_driver() {
    let files = app(&descriptors(SHOP), SHOP, &seed("shop"), None, None).unwrap();
    assert_golden("shop", &files);
}

#[test]
fn the_mixed_blackjack_app_is_its_golden_rust_table_python_player_and_policies() {
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &blackjack(), None, None).unwrap();
    assert_golden("blackjack", &files);
}

/// examples-rust's layout: the domain layer is a library crate's
/// (examples-proto runs `wire` and `markup_domain`), so the app generates
/// none of its own.
fn examples_proto() -> DomainCrate {
    DomainCrate {
        path: "examples_proto".to_string(),
        wire: Some("examples_proto::v1".to_string()),
    }
}

#[test]
fn an_app_over_a_domain_crate_reuses_its_domain_layer_and_generates_none() {
    let seeded = AppSeed {
        domain: Some(examples_proto()),
        ..blackjack()
    };
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &seeded, None, None).unwrap();
    assert_golden("blackjack-domain-crate", &files);
    let main = &file(&files, "src/main.rs").source;
    assert!(
        main.contains("\npub use examples_proto::domain;\n"),
        "{main}"
    );
    assert!(
        main.contains("\npub use examples_proto::v1 as wire;\n"),
        "{main}"
    );
    assert!(
        !main.contains(".domain.rs") && !main.contains("examples.v1.rs"),
        "no generated wire types or domain layer: {main}"
    );
    let build = &file(&files, "build.rs").source;
    assert!(
        build.contains("angzarr_build::app_without_domain(\"angzarr.toml\", PROTOS, INCLUDES)"),
        "{build}"
    );
    let manifest = &file(&files, "Cargo.toml").source;
    assert!(
        manifest.contains(
            "\n[package.metadata.angzarr.domain]\ncrate = \"examples_proto\"\nwire = \"examples_proto::v1\"\n"
        ),
        "{manifest}"
    );
}

#[test]
fn a_domain_crate_with_no_wire_path_has_no_wire_module() {
    let seeded = AppSeed {
        domain: Some(DomainCrate {
            wire: None,
            ..examples_proto()
        }),
        ..blackjack()
    };
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &seeded, None, None).unwrap();
    let main = &file(&files, "src/main.rs").source;
    assert!(
        main.contains("\npub use examples_proto::domain;\n"),
        "{main}"
    );
    assert!(!main.contains("wire"), "{main}");
    let manifest = &file(&files, "Cargo.toml").source;
    assert!(
        manifest.contains("\n[package.metadata.angzarr.domain]\ncrate = \"examples_proto\"\n\n"),
        "{manifest}"
    );
}

#[test]
fn the_manifest_on_disk_names_the_domain_crate_not_the_seed() {
    let set = descriptors(BLACKJACK);
    let seeded = AppSeed {
        domain: Some(examples_proto()),
        ..blackjack()
    };
    let over = app(&set, BLACKJACK, &seeded, None, None).unwrap();
    let manifest = file(&over, "Cargo.toml").source.clone();
    // The user's manifest names the crate: the seed (none) is unused.
    let files = app(&set, BLACKJACK, &blackjack(), None, Some(&manifest)).unwrap();
    assert_eq!(file(&files, "Cargo.toml").source, manifest);
    assert_eq!(
        file(&files, "src/main.rs").source,
        file(&over, "src/main.rs").source
    );
    assert_eq!(
        file(&files, "build.rs").source,
        file(&over, "build.rs").source
    );
    // And a manifest naming none generates the layer, whatever the seed.
    let plain = app(&set, BLACKJACK, &blackjack(), None, None).unwrap();
    let mine = file(&plain, "Cargo.toml").source.clone();
    let files = app(&set, BLACKJACK, &seeded, None, Some(&mine)).unwrap();
    assert_eq!(
        file(&files, "src/main.rs").source,
        file(&plain, "src/main.rs").source
    );
}

#[test]
fn a_domain_crate_that_is_no_rust_path_or_a_manifest_that_does_not_read_is_refused() {
    let set = descriptors(BLACKJACK);
    for (path, wire) in [("examples-proto", None), ("examples_proto", Some("v1 x"))] {
        let seeded = AppSeed {
            domain: Some(DomainCrate {
                path: path.to_string(),
                wire: wire.map(str::to_string),
            }),
            ..blackjack()
        };
        let bad = wire.unwrap_or(path).to_string();
        assert_eq!(
            app(&set, BLACKJACK, &seeded, None, None),
            Err(CodegenError::AppDomainPath { path: bad })
        );
    }
    for manifest in [
        "[package\n",
        "[package]\nname = \"x\"\n[package.metadata.angzarr.domain]\nwire = \"x\"\n",
        "[package]\nname = \"x\"\n[package.metadata.angzarr.domain]\ncrate = \"x\"\nwires = \"x\"\n",
    ] {
        assert!(
            matches!(
                app(&set, BLACKJACK, &blackjack(), None, Some(manifest)),
                Err(CodegenError::AppManifest { .. })
            ),
            "{manifest}"
        );
    }
}

/// The config, its comments and blank lines dropped.
fn settings(toml: &str) -> Vec<&str> {
    toml.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

#[test]
fn the_generated_blackjack_config_is_the_one_its_e2e_hand_writes() {
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &blackjack(), None, None).unwrap();
    // What the e2e hand-wrote, now with every component listed, each with
    // its kind (the owner's config rulings): the table, the player a
    // Python child, the two policies between them in process; the wallet
    // policy's target is not in the application, so it is not listed.
    let hand = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\
                dir = \"player\"\nentry = [\"angzarr-player\"]\n\n\
                [[component]]\nname = \"policy-table-player\"\nkind = \"policy\"\nlanguage = \"rust\"\n\n\
                [[component]]\nname = \"policy-player-table\"\nkind = \"policy\"\nlanguage = \"rust\"\n\n\
                [store]\nin_memory = true\n";
    assert_eq!(
        settings(&file(&files, "angzarr.toml").source),
        settings(hand)
    );
    // examples-rust's remote_player: the same components, its `dir` an
    // environment variable's and its handshake a slow first `uv` run's.
    let remote_player = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                         [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\
                         entry = [\"angzarr-player\"]\n";
    let generated = settings(&file(&files, "angzarr.toml").source);
    for line in settings(remote_player) {
        assert!(generated.contains(&line), "{line}: {generated:?}");
    }
}

#[test]
fn the_generated_main_registers_only_the_in_process_components_and_their_policies() {
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &blackjack(), None, None).unwrap();
    let main = &file(&files, "src/main.rs").source;
    for module in [
        "pub mod table_aggregate;",
        "pub mod table_player_policy;",
        "pub mod player_table_policy;",
    ] {
        assert!(main.contains(module), "{module}:\n{main}");
    }
    // The player is the Python child's; the wallet policy's target is
    // outside the application.
    for absent in ["pub mod player_aggregate", "pub mod table_wallet_policy"] {
        assert!(!main.contains(absent), "{absent}:\n{main}");
    }
}

#[test]
fn expose_and_the_retry_settings_seeded_are_emitted_and_the_unseeded_are_not() {
    let seeded = AppSeed {
        expose: vec![("table".to_string(), Exposure::Loopback)],
        retry: RetrySeed {
            max_retries: Some(5),
            max_race_runs: Some(0),
        },
        ..blackjack()
    };
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &seeded, None, None).unwrap();
    let config = &file(&files, "angzarr.toml").source;
    assert!(
        config.contains(
            "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\nexpose = \"loopback\"\n"
        ),
        "{config}"
    );
    assert!(
        config.contains("\n[retry]\nmax_retries = 5\nmax_race_runs = 0\n"),
        "{config}"
    );
    let ip = AppSeed {
        expose: vec![("table".to_string(), Exposure::Ip)],
        ..blackjack()
    };
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &ip, None, None).unwrap();
    let config = &file(&files, "angzarr.toml").source;
    assert!(config.contains("expose = \"ip\"\n"), "{config}");
    assert!(
        !config.contains("[retry]"),
        "no retry key seeded, no table: the host's defaults: {config}"
    );

    let plain = app(&descriptors(BLACKJACK), BLACKJACK, &blackjack(), None, None).unwrap();
    let config = &file(&plain, "angzarr.toml").source;
    assert!(
        !config.contains("expose") && !config.contains("[retry]"),
        "{config}"
    );
}

#[test]
fn an_existing_config_is_the_input_the_generated_files_follow_not_the_seed() {
    // The user moved the table to a Go child of their own and kept the
    // player in Rust; the seed still says the player is Python.
    let mine = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"go\"\n\
                launch = [\"./bin/table\"]\n\n\
                [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"rust\"\n";
    let files = app(
        &descriptors(BLACKJACK),
        BLACKJACK,
        &blackjack(),
        Some(mine),
        None,
    )
    .unwrap();
    let config = file(&files, "angzarr.toml");
    assert_eq!(config.owner, Owner::User);
    assert_eq!(config.source, mine, "the config on disk is passed through");
    let main = &file(&files, "src/main.rs").source;
    assert!(main.contains("pub mod player_aggregate;"), "{main}");
    assert!(!main.contains("pub mod table_aggregate;"), "{main}");
}

#[test]
fn generating_twice_from_the_same_inputs_is_byte_identical() {
    let set = descriptors(BLACKJACK);
    let first = app(&set, BLACKJACK, &blackjack(), None, None).unwrap();
    let config = file(&first, "angzarr.toml").source.clone();
    let second = app(&set, BLACKJACK, &blackjack(), Some(&config), None).unwrap();
    assert!(!first.is_empty());
    assert_eq!(first, second);
}

#[test]
fn a_seed_naming_a_component_the_protos_lack_is_refused() {
    let seeded = AppSeed {
        languages: vec![("dealer".to_string(), "python".to_string())],
        ..blackjack()
    };
    assert_eq!(
        app(&descriptors(BLACKJACK), BLACKJACK, &seeded, None, None),
        Err(CodegenError::AppRefused {
            lints: vec![AppLint::UnknownComponent {
                name: "dealer".to_string(),
                kind: "domain".to_string(),
            }]
        })
    );
}

#[test]
fn a_seed_exposing_a_process_manager_is_refused() {
    // A process manager's name is pmg- and its domain (decision 14.50).
    let buyin = AppSeed {
        expose: vec![("pmg-buyin".to_string(), Exposure::Loopback)],
        ..seed("buyin")
    };
    assert_eq!(
        app(&descriptors(BUYIN), BUYIN, &buyin, None, None),
        Err(CodegenError::AppRefused {
            lints: vec![AppLint::NotExposable {
                name: "pmg-buyin".to_string(),
                kind: "process_manager".to_string(),
            }]
        })
    );
}

#[test]
fn an_existing_config_with_lints_is_refused() {
    let mine = "[[component]]\nname = \"dealer\"\nkind = \"domain\"\nlanguage = \"rust\"\n";
    let refused = app(
        &descriptors(BLACKJACK),
        BLACKJACK,
        &blackjack(),
        Some(mine),
        None,
    );
    assert!(
        matches!(&refused, Err(CodegenError::AppRefused { lints }) if !lints.is_empty()),
        "{refused:?}"
    );
}

fn lints(config: &str, files: &[&str]) -> Vec<AppLint> {
    lint_app(config, &descriptors(BLACKJACK), files).unwrap()
}

#[test]
fn the_generated_configs_have_no_lints() {
    assert_eq!(
        lint_app(&golden("shop", "angzarr.toml"), &descriptors(SHOP), SHOP).unwrap(),
        []
    );
    assert_eq!(lints(&golden("blackjack", "angzarr.toml"), BLACKJACK), []);
}

#[test]
fn lint_app_flags_an_unknown_component_a_duplicate_and_an_unlisted_domain() {
    let config = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"dealer\"\nkind = \"domain\"\nlanguage = \"rust\"\n";
    assert_eq!(
        lints(config, BLACKJACK),
        [
            AppLint::DuplicateComponent {
                name: "table".to_string()
            },
            AppLint::UnknownComponent {
                name: "dealer".to_string(),
                kind: "domain".to_string(),
            },
            AppLint::UnlistedDomain {
                domain: "player".to_string()
            },
        ]
    );
}

#[test]
fn lint_app_flags_a_policy_component_the_protos_do_not_declare() {
    let config = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"dealer-table-policy\"\nkind = \"policy\"\n\
                  language = \"python\"\n";
    assert_eq!(
        lints(config, BLACKJACK),
        [AppLint::UnknownComponent {
            name: "dealer-table-policy".to_string(),
            kind: "policy".to_string(),
        }]
    );
}

#[test]
fn lint_app_flags_a_policy_whose_source_domain_is_not_in_the_app() {
    // The table alone, and the player-to-table policy listed: it reads the
    // player, which nothing of the application owns. A policy of the
    // protos the config does not list is not registered, and no lint.
    let config = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"policy-player-table\"\nkind = \"policy\"\n\
                  language = \"rust\"\n";
    let files = [BLACKJACK[0], BLACKJACK[2]];
    assert_eq!(
        lints(config, &files),
        [AppLint::PolicySourceAbsent {
            policy: "policy-player-table".to_string(),
            domain: "player".to_string(),
        }]
    );
    let table = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n";
    assert_eq!(lints(table, &files), []);
}

#[test]
fn lint_app_flags_a_remote_policy_whose_source_domain_is_not_in_the_app() {
    // A Python child serving the player-to-table policy, by its name from
    // the protos, beside the table alone.
    let config = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"policy-player-table\"\nkind = \"policy\"\n\
                  language = \"python\"\nentry = [\"-m\", \"player_table.serve\"]\n";
    let files = [BLACKJACK[0], BLACKJACK[2]];
    assert_eq!(
        lints(config, &files),
        [AppLint::PolicySourceAbsent {
            policy: "policy-player-table".to_string(),
            domain: "player".to_string(),
        }]
    );
}

#[test]
fn lint_app_flags_a_policy_whose_target_is_not_in_the_app_unless_declared_remote() {
    const WALLET: &str = "io/angzarr/examples/v1/wallet.proto";
    let files = [BLACKJACK[0], BLACKJACK[1], BLACKJACK[2], WALLET];
    let set = descriptors(&files);
    let base = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\n\
                [[component]]\nname = \"policy-table-wallet\"\nkind = \"policy\"\nlanguage = \"rust\"\n";
    assert_eq!(
        lint_app(base, &set, &files).unwrap(),
        [
            AppLint::PolicyTargetAbsent {
                policy: "policy-table-wallet".to_string(),
                domain: "wallet".to_string(),
            },
            AppLint::UnlistedDomain {
                domain: "wallet".to_string()
            },
        ]
    );
    // The wallet declared remote, a child of its own: no lint.
    let remote = format!(
        "{base}\n[[component]]\nname = \"wallet\"\nkind = \"domain\"\nlanguage = \"go\"\n\
         launch = [\"./bin/wallet\"]\n"
    );
    assert_eq!(lint_app(&remote, &set, &files).unwrap(), []);
}

#[test]
fn lint_app_flags_exposing_anything_but_an_aggregate() {
    let config = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\nexpose = \"ip\"\n\n\
                  [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"policy-table-player\"\nkind = \"policy\"\n\
                  language = \"python\"\nexpose = \"loopback\"\n\n\
                  [[component]]\nname = \"table-view\"\nkind = \"projector\"\n\
                  language = \"python\"\nexpose = \"ip\"\n";
    assert_eq!(
        lints(config, BLACKJACK),
        [
            AppLint::NotExposable {
                name: "policy-table-player".to_string(),
                kind: "policy".to_string(),
            },
            AppLint::NotExposable {
                name: "table-view".to_string(),
                kind: "projector".to_string(),
            },
        ],
        "an exposed aggregate is fine; a projector is not checked against the protos"
    );
    let buyin = "[[component]]\nname = \"wallet\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                 [[component]]\nname = \"pmg-buyin\"\nkind = \"domain\"\nlanguage = \"rust\"\nexpose = \"loopback\"\n";
    assert_eq!(
        lint_app(buyin, &descriptors(BUYIN), BUYIN).unwrap(),
        [AppLint::NotExposable {
            name: "pmg-buyin".to_string(),
            kind: "process_manager".to_string(),
        }]
    );
}

#[test]
fn lint_app_flags_a_language_with_no_launch_and_an_empty_launch() {
    let config = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"cobol\"\n";
    assert_eq!(
        lints(config, BLACKJACK),
        [AppLint::UnknownLanguage {
            name: "player".to_string(),
            language: "cobol".to_string(),
        }]
    );
    let empty = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                 [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\nlaunch = []\n";
    assert_eq!(
        lints(empty, BLACKJACK),
        [AppLint::EmptyLaunch {
            name: "player".to_string()
        }]
    );
}

#[test]
fn lint_app_refuses_what_is_not_the_hosts_config() {
    for config in [
        // An unknown key, of a component and of a table.
        "[[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\
         lanuch = [\"x\"]\n",
        "[retry]\nmax_race_runz = 0\n",
        // A value outside a closed set.
        "[[component]]\nname = \"table\"\nkind = \"saga\"\nlanguage = \"rust\"\n",
        "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\
         expose = \"world\"\n",
        "[policy]\non_bug = \"ignore\"\n",
        // A component with no kind (the owner's ruling: kind is required).
        "[[component]]\nname = \"table\"\nlanguage = \"rust\"\n",
        // A key of the wrong type.
        "[store]\nin_memory = \"yes\"\n",
        "[supervision]\ncall_timeout_ms = -1\n",
    ] {
        assert!(
            matches!(
                lint_app(config, &descriptors(BLACKJACK), BLACKJACK),
                Err(CodegenError::AppConfig { .. })
            ),
            "{config}"
        );
    }
}

#[test]
fn remote_domains_are_the_domains_not_in_rust_or_given_a_launch() {
    let config = "[[component]]\nname = \"table\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"dealer\"\nkind = \"domain\"\nlanguage = \"rust\"\n\
                  launch = [\"./dealer\"]\n\n\
                  [[component]]\nname = \"table-view\"\nkind = \"projector\"\nlanguage = \"python\"\n";
    assert_eq!(
        remote_domains(config).unwrap(),
        ["player".to_string(), "dealer".to_string()]
    );
}

/// What a child process serves besides a domain: every policy, projector
/// or upcaster not in Rust, or given a `launch`, by name.
#[test]
fn child_components_are_the_others_not_in_rust_or_given_a_launch() {
    let config = "[[component]]\nname = \"player\"\nkind = \"domain\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"policy-table-player\"\nkind = \"policy\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"policy-player-table\"\nkind = \"policy\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"table-view\"\nkind = \"projector\"\nlanguage = \"rust\"\n\
                  launch = [\"./view\"]\n";
    assert_eq!(
        child_components(config).unwrap(),
        ["policy-table-player".to_string(), "table-view".to_string()]
    );
}

#[test]
fn a_projector_is_listed_when_its_sources_are_all_the_applications() {
    const WITH_LEDGER: &[&str] = &[
        "io/angzarr/examples/v1/table.proto",
        "io/angzarr/examples/v1/wallet.proto",
        "io/angzarr/examples/v1/ledger.proto",
        "io/angzarr/examples/v1/player.proto",
    ];
    let set = descriptors(WITH_LEDGER);
    let listed = |seed: &AppSeed| {
        let files = app(&set, WITH_LEDGER, seed, None, None).unwrap();
        file(&files, "angzarr.toml")
            .source
            .contains("name = \"projector-wallet-table-ledger\"\nkind = \"projector\"")
    };
    assert!(listed(&seed("ledger")));
    // A Python domain that is not one of its sources leaves it listed.
    let player = AppSeed {
        languages: vec![("player".to_string(), "python".to_string())],
        ..seed("ledger")
    };
    assert!(listed(&player));
    // The wallet a Python child: the host records its events, so the
    // projector of it runs in process all the same.
    let python = AppSeed {
        languages: vec![("wallet".to_string(), "python".to_string())],
        ..seed("ledger")
    };
    assert!(listed(&python));
    // A source no component of the application owns (the wallet's protos
    // not generated) leaves it unlisted.
    let alone = [WITH_LEDGER[0], WITH_LEDGER[2]];
    let files = app(&set, &alone, &seed("ledger"), None, None).unwrap();
    assert!(
        !file(&files, "angzarr.toml")
            .source
            .contains("kind = \"projector\""),
        "{}",
        file(&files, "angzarr.toml").source
    );
    // A projector of no source is not listed.
    const LONE: &[&str] = &["lone/v1/lone.proto"];
    let files = app(&descriptors(LONE), LONE, &seed("lone"), None, None).unwrap();
    assert!(
        !file(&files, "angzarr.toml").source.contains("projector"),
        "{}",
        file(&files, "angzarr.toml").source
    );
}

#[test]
fn an_exposure_lands_on_its_component_alone_and_one_naming_none_is_a_lint() {
    let seeded = AppSeed {
        expose: vec![("table".to_string(), Exposure::Ip)],
        ..blackjack()
    };
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &seeded, None, None).unwrap();
    let config = &file(&files, "angzarr.toml").source;
    assert_eq!(config.matches("expose = ").count(), 1, "{config}");
    let nowhere = AppSeed {
        expose: vec![("nowhere".to_string(), Exposure::Ip)],
        ..blackjack()
    };
    assert_eq!(
        app(&descriptors(BLACKJACK), BLACKJACK, &nowhere, None, None),
        Err(CodegenError::AppRefused {
            lints: vec![AppLint::UnknownComponent {
                name: "nowhere".to_string(),
                kind: "domain".to_string(),
            }]
        })
    );
}

#[test]
fn the_manifests_package_is_the_apps_name_as_a_crate_name() {
    for (name, package) in [
        ("shop", "shop"),
        ("  My Shop!!  App ", "my-shop-app"),
        ("-x", "x"),
    ] {
        let files = app(&descriptors(SHOP), SHOP, &seed(name), None, None).unwrap();
        let manifest = &file(&files, "Cargo.toml").source;
        assert!(
            manifest.contains(&format!("\nname = \"{package}\"\n")),
            "{name}: {manifest}"
        );
    }
}

fn language(name: &str, language: &str) -> (String, String) {
    (name.to_string(), language.to_string())
}

/// `config`'s entry for `name`, from its `[[component]]` to the next.
fn entry<'a>(config: &'a str, name: &str) -> &'a str {
    let at = config
        .find(&format!("name = \"{name}\"\n"))
        .unwrap_or_else(|| panic!("no {name}:\n{config}"));
    let rest = &config[at..];
    &rest[..rest.find("\n\n").unwrap_or(rest.len())]
}

/// Every kind takes its language from the seed, by its name in the
/// application: a Python one is a child, its `dir` and entry point the
/// project `angzarr scaffold python` writes for that name.
#[test]
fn a_seed_puts_each_component_it_names_in_its_language() {
    let seeded = AppSeed {
        languages: vec![
            language("player", "python"),
            language("policy-table-player", "python"),
        ],
        ..blackjack()
    };
    let files = app(&descriptors(BLACKJACK), BLACKJACK, &seeded, None, None).unwrap();
    let config = &file(&files, "angzarr.toml").source;
    assert_eq!(
        entry(config, "policy-table-player"),
        "name = \"policy-table-player\"\nkind = \"policy\"\nlanguage = \"python\"\n\
         dir = \"policy-table-player\"\nentry = [\"angzarr-policy-table-player\"]"
    );
    assert_eq!(
        entry(config, "policy-player-table"),
        "name = \"policy-player-table\"\nkind = \"policy\"\nlanguage = \"rust\""
    );
    // The Python policy is the child's: no module of it in main.rs.
    let main = &file(&files, "src/main.rs").source;
    assert!(!main.contains("pub mod table_player_policy"), "{main}");
    assert!(main.contains("pub mod player_table_policy;"), "{main}");
}

#[test]
fn a_seed_puts_a_projector_in_its_language() {
    const WITH_LEDGER: &[&str] = &[
        "io/angzarr/examples/v1/table.proto",
        "io/angzarr/examples/v1/wallet.proto",
        "io/angzarr/examples/v1/ledger.proto",
    ];
    let seeded = AppSeed {
        languages: vec![language("projector-wallet-table-ledger", "python")],
        ..seed("ledger")
    };
    let files = app(&descriptors(WITH_LEDGER), WITH_LEDGER, &seeded, None, None).unwrap();
    assert_eq!(
        entry(
            &file(&files, "angzarr.toml").source,
            "projector-wallet-table-ledger"
        ),
        "name = \"projector-wallet-table-ledger\"\nkind = \"projector\"\nlanguage = \"python\"\n\
         dir = \"projector-wallet-table-ledger\"\nentry = [\"angzarr-projector-wallet-table-ledger\"]"
    );
}

/// A domain is named by its name in the application or by its domain; its
/// Python project is its domain's, as `scaffold python --domain` names it.
#[test]
fn a_seed_names_a_domain_by_its_component_name_or_its_domain() {
    const BUYIN: &[&str] = &[
        "buyin/v1/wallet.proto",
        "buyin/v1/buy_in.proto",
        "buyin/v1/policies.proto",
    ];
    for name in ["pmg-buyin", "buyin"] {
        let seeded = AppSeed {
            languages: vec![language(name, "python")],
            ..seed("buyin")
        };
        let files = app(&descriptors(BUYIN), BUYIN, &seeded, None, None).unwrap();
        assert_eq!(
            entry(&file(&files, "angzarr.toml").source, "pmg-buyin"),
            "name = \"pmg-buyin\"\nkind = \"domain\"\nlanguage = \"python\"\n\
             dir = \"buyin\"\nentry = [\"angzarr-buyin\"]",
            "{name}"
        );
    }
}

/// A language the host has no launch for is written as given, and the
/// lint refuses it: a `launch` is the user's to add.
#[test]
fn a_seed_in_a_language_with_no_launch_is_refused() {
    let seeded = AppSeed {
        languages: vec![language("policy-table-player", "go")],
        ..blackjack()
    };
    assert_eq!(
        app(&descriptors(BLACKJACK), BLACKJACK, &seeded, None, None),
        Err(CodegenError::AppRefused {
            lints: vec![AppLint::UnknownLanguage {
                name: "policy-table-player".to_string(),
                language: "go".to_string(),
            }]
        })
    );
}
