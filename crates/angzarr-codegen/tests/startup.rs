//! The startup of an angzarr-embedded application: the host config (data:
//! each `[[component]]`, its kind and language, a remote one's launch), a
//! thin main.rs and
//! the build script, pinned by golden files (`tests/golden/startup`), and
//! the lints of a host config against the protos.

mod common;

use angzarr_codegen::{
    CodegenError, Owner, StartupApp, StartupLint, component_packages, lint_startup, remote_domains,
    startup,
};
use common::*;

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/golden/startup/blackjack"
);
const FILES: &[&str] = &[
    "io/angzarr/examples/v1/table.proto",
    "io/angzarr/examples/v1/player.proto",
    "io/angzarr/examples/v1/policies.proto",
];

fn blackjack() -> StartupApp {
    StartupApp {
        name: "blackjack".to_string(),
        python: vec!["player".to_string()],
        includes: vec!["proto".to_string()],
        angzarr_build: r#"{ git = "https://github.com/angzarr-io/angzarr-cli", rev = "c1898ae" }"#
            .to_string(),
    }
}

#[test]
fn test_startup_matches_the_golden_files_all_the_users() {
    let files = startup(&descriptors(BLACKJACK), FILES, &blackjack()).unwrap();
    let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            "Cargo.toml",
            "angzarr.toml",
            "build.rs",
            "src/driver.rs",
            "src/main.rs"
        ]
    );
    for f in &files {
        assert_eq!(f.owner, Owner::User, "{}", f.path);
        let golden = std::fs::read_to_string(format!("{GOLDEN}/{}", f.path)).unwrap();
        assert_eq!(f.source, golden, "{}", f.path);
    }
}

#[test]
fn test_startup_refuses_a_python_domain_the_protos_do_not_declare() {
    let app = StartupApp {
        python: vec!["dealer".to_string()],
        ..blackjack()
    };
    assert_eq!(
        startup(&descriptors(BLACKJACK), FILES, &app),
        Err(CodegenError::PythonNoDomain {
            domain: "dealer".to_string()
        })
    );
}

fn lints(config: &str) -> Vec<StartupLint> {
    lint_startup(config, &descriptors(BLACKJACK), FILES).unwrap()
}

#[test]
fn test_the_generated_config_has_no_lints() {
    let config = std::fs::read_to_string(format!("{GOLDEN}/angzarr.toml")).unwrap();
    assert_eq!(lints(&config), []);
}

#[test]
fn test_lint_startup_flags_a_language_with_no_launch() {
    let config = "[[component]]\nname = \"table\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"player\"\nlanguage = \"cobol\"\n";
    assert_eq!(
        lints(config),
        [StartupLint::UnknownLanguage {
            domain: "player".to_string(),
            language: "cobol".to_string(),
        }]
    );
    let launched = format!("{config}launch = [\"./player\"]\n");
    assert_eq!(lints(&launched), [], "a launch makes any language runnable");
}

#[test]
fn test_lint_startup_flags_an_empty_launch() {
    for launch in ["[]", "[\"\"]", "[\" \", \"x\"]"] {
        let config = format!(
            "[[component]]\nname = \"table\"\nlanguage = \"rust\"\n\n\
             [[component]]\nname = \"player\"\nlanguage = \"python\"\nlaunch = {launch}\n"
        );
        assert_eq!(
            lints(&config),
            [StartupLint::EmptyLaunch {
                domain: "player".to_string()
            }],
            "{launch}"
        );
    }
}

#[test]
fn test_lint_startup_flags_unknown_duplicate_and_unlisted_domains() {
    let config = "[[component]]\nname = \"table\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"table\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"dealer\"\nlanguage = \"rust\"\n";
    assert_eq!(
        lints(config),
        [
            StartupLint::DuplicateDomain {
                domain: "table".to_string()
            },
            StartupLint::UnknownDomain {
                domain: "dealer".to_string()
            },
            StartupLint::UnlistedDomain {
                domain: "player".to_string()
            },
        ]
    );
}

#[test]
fn test_lint_startup_refuses_a_config_that_is_not_a_host_config() {
    let misspelt = "[[component]]\nname = \"player\"\nlanguage = \"python\"\nlanuch = [\"x\"]\n";
    assert!(matches!(
        lint_startup(misspelt, &descriptors(BLACKJACK), FILES),
        Err(CodegenError::StartupConfig { .. })
    ));
}

#[test]
fn test_startup_pins_angzarr_embedded_by_the_one_constant() {
    let files = startup(&descriptors(BLACKJACK), FILES, &blackjack()).unwrap();
    let manifest = &files
        .iter()
        .find(|f| f.path == "Cargo.toml")
        .unwrap()
        .source;
    for krate in [
        "angzarr-embedded",
        "angzarr-embedded-diagnostic",
        "angzarr-embedded-host",
        "angzarr-embedded-macros",
    ] {
        assert!(
            manifest.contains(&format!(
                "{krate} = {{ git = \"https://github.com/angzarr-io/angzarr-embedded\", rev = \"{}\" }}",
                angzarr_codegen::ANGZARR_EMBEDDED_REV
            )),
            "{krate}:\n{manifest}"
        );
    }
}

#[test]
fn test_remote_domains_are_those_not_in_rust_or_given_a_launch() {
    let config = "[[component]]\nname = \"table\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"player\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"dealer\"\nlanguage = \"rust\"\nlaunch = [\"./dealer\"]\n";
    assert_eq!(
        remote_domains(config).unwrap(),
        ["player".to_string(), "dealer".to_string()]
    );
}

#[test]
fn test_component_packages_names_each_package_once_in_order() {
    let mut files = BUYIN.to_vec();
    files.extend(FILES);
    let mut every = BUYIN.to_vec();
    every.extend(BLACKJACK);
    let set = descriptors(&every);
    assert_eq!(
        component_packages(&set, &files).unwrap(),
        ["buyin.v1".to_string(), "io.angzarr.examples.v1".to_string()]
    );
}

#[test]
fn test_lint_startup_checks_only_domains_against_the_protos() {
    let config = "[[component]]\nname = \"table\"\nlanguage = \"rust\"\n\n\
                  [[component]]\nname = \"player\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"table-view\"\nkind = \"projector\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"table-player\"\nkind = \"policy\"\nlanguage = \"cobol\"\n";
    assert_eq!(
        lints(config),
        [StartupLint::UnknownLanguage {
            domain: "table-player".to_string(),
            language: "cobol".to_string(),
        }],
        "a policy or projector is no domain of the protos, but needs a launch as any child does"
    );
    let named_twice = format!(
        "{config}\n[[component]]\nname = \"player\"\nkind = \"policy\"\nlanguage = \"rust\"\n"
    );
    assert!(
        lints(&named_twice).contains(&StartupLint::DuplicateDomain {
            domain: "player".to_string()
        }),
        "one name per component, whatever its kind"
    );
}

#[test]
fn test_lint_startup_refuses_an_unknown_kind_and_the_old_domain_tables() {
    for config in [
        "[[component]]\nname = \"table\"\nkind = \"saga\"\nlanguage = \"rust\"\n",
        "[[domain]]\nname = \"table\"\nlanguage = \"rust\"\n",
    ] {
        assert!(
            matches!(
                lint_startup(config, &descriptors(BLACKJACK), FILES),
                Err(CodegenError::StartupConfig { .. })
            ),
            "{config}"
        );
    }
}

#[test]
fn test_remote_domains_are_domains_only() {
    let config = "[[component]]\nname = \"player\"\nlanguage = \"python\"\n\n\
                  [[component]]\nname = \"table-view\"\nkind = \"projector\"\nlanguage = \"python\"\n";
    assert_eq!(remote_domains(config).unwrap(), ["player".to_string()]);
}
