//! The file side of `angzarr scaffold app`: the host config and the driver
//! written once and never overwritten, build.rs and main.rs rewritten every
//! run from the protos and the host config on disk, a refused config
//! writing nothing; and `app`, the one call the generated build.rs makes.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use angzarr_build::{AppSeed, BuildError, RetrySeed, Scaffolded, app_into, app_scaffold_into};
use angzarr_codegen::CodegenError;

const BUYIN: &[&str] = &[
    "buyin/v1/wallet.proto",
    "buyin/v1/buy_in.proto",
    "buyin/v1/policies.proto",
];
const SHOP: &[&str] = &["shop/v1/shop.proto"];
const APP_FILES: &[&str] = &[
    "Cargo.toml",
    "angzarr.toml",
    "build.rs",
    "src/driver.rs",
    "src/main.rs",
];

fn fixtures() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../angzarr-codegen/tests/fixtures")
        .display()
        .to_string()
}

/// A fresh, empty directory.
fn empty_dir() -> PathBuf {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "angzarr-build-app-test-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed() -> AppSeed {
    AppSeed {
        name: "buyin".to_string(),
        includes: vec!["proto".to_string()],
        python: Vec::new(),
        expose: Vec::new(),
        retry: RetrySeed::default(),
        in_memory: true,
        angzarr_build: None,
    }
}

fn scaffold(root: &Path, protos: &[&str], seed: &AppSeed) -> Vec<Scaffolded> {
    app_scaffold_into(root, protos, &[&fixtures()], seed).unwrap()
}

/// Each file's action, by its path under `root`, sorted by path.
fn actions(root: &Path, done: &[Scaffolded]) -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = done
        .iter()
        .map(|d| {
            let (path, action) = match d {
                Scaffolded::Created(p) => (p, "created"),
                Scaffolded::Completed(p) => (p, "completed"),
                Scaffolded::Regenerated(p) => (p, "regenerated"),
                Scaffolded::Kept(p) => (p, "kept"),
            };
            let rel = path.strip_prefix(root).unwrap_or(path);
            (rel.display().to_string(), action)
        })
        .collect();
    out.sort();
    out
}

fn all(action: &'static str) -> Vec<(String, &'static str)> {
    APP_FILES.iter().map(|p| (p.to_string(), action)).collect()
}

fn read(root: &Path, path: &str) -> String {
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn scaffolding_an_app_writes_its_config_build_script_main_and_driver() {
    let root = empty_dir();
    assert_eq!(
        actions(&root, &scaffold(&root, SHOP, &seed())),
        all("created")
    );
    for path in APP_FILES {
        assert!(root.join(path).exists(), "{path}");
    }
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn rescaffolding_an_app_unchanged_keeps_every_file_byte_for_byte() {
    let root = empty_dir();
    scaffold(&root, SHOP, &seed());
    let before: Vec<String> = APP_FILES.iter().map(|p| read(&root, p)).collect();
    assert_eq!(actions(&root, &scaffold(&root, SHOP, &seed())), all("kept"));
    let after: Vec<String> = APP_FILES.iter().map(|p| read(&root, p)).collect();
    assert_eq!(before, after);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn rescaffolding_never_overwrites_the_users_config_or_driver_and_rewrites_the_clis_files() {
    let root = empty_dir();
    scaffold(&root, SHOP, &seed());
    let config = format!(
        "{}\n[retry]\nmax_retries = 9\n",
        read(&root, "angzarr.toml")
    );
    let driver = "// mine\npub async fn drive(_: &angzarr_embedded_host::App) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }\n";
    std::fs::write(root.join("angzarr.toml"), &config).unwrap();
    std::fs::write(root.join("src/driver.rs"), driver).unwrap();
    let main = read(&root, "src/main.rs");
    let build = read(&root, "build.rs");
    std::fs::write(root.join("src/main.rs"), "// edited by hand\n").unwrap();
    std::fs::write(root.join("build.rs"), "// edited by hand\n").unwrap();

    // A different seed changes nothing of a config that exists.
    let other = AppSeed {
        in_memory: false,
        retry: RetrySeed {
            max_retries: Some(1),
            max_race_runs: Some(1),
        },
        ..seed()
    };
    assert_eq!(
        actions(&root, &scaffold(&root, SHOP, &other)),
        [
            ("Cargo.toml".to_string(), "kept"),
            ("angzarr.toml".to_string(), "kept"),
            ("build.rs".to_string(), "regenerated"),
            ("src/driver.rs".to_string(), "kept"),
            ("src/main.rs".to_string(), "regenerated"),
        ]
    );
    assert_eq!(read(&root, "angzarr.toml"), config);
    assert_eq!(read(&root, "src/driver.rs"), driver);
    assert_eq!(read(&root, "src/main.rs"), main);
    assert_eq!(read(&root, "build.rs"), build);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_edit_to_the_config_is_followed_by_the_next_main() {
    let root = empty_dir();
    scaffold(&root, BUYIN, &seed());
    assert!(read(&root, "src/main.rs").contains("pub mod buy_in;"));
    // The user moves the buy-in (its process manager, pmg-buyin) to a
    // Python child.
    let config = read(&root, "angzarr.toml").replace(
        "name = \"pmg-buyin\"\nkind = \"domain\"\nlanguage = \"rust\"\n",
        "name = \"pmg-buyin\"\nkind = \"domain\"\nlanguage = \"python\"\n\
         entry = [\"angzarr-buyin\"]\n",
    );
    assert_ne!(config, read(&root, "angzarr.toml"));
    std::fs::write(root.join("angzarr.toml"), &config).unwrap();
    scaffold(&root, BUYIN, &seed());
    let main = read(&root, "src/main.rs");
    assert!(!main.contains("pub mod buy_in;"), "{main}");
    assert!(main.contains("pub mod wallet;"), "{main}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_config_with_lints_is_refused_and_nothing_is_written() {
    let root = empty_dir();
    let dealer = AppSeed {
        python: vec!["dealer".to_string()],
        ..seed()
    };
    let refused = app_scaffold_into(&root, BUYIN, &[&fixtures()], &dealer);
    assert!(
        matches!(
            &refused,
            Err(BuildError::Codegen(CodegenError::AppRefused { lints })) if lints.len() == 1
        ),
        "{refused:?}"
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn app_into_writes_the_whole_layer_the_configs_remote_domains_as_contracts() {
    let out = empty_dir();
    let config = out.join("angzarr.toml");
    std::fs::write(
        &config,
        "[[component]]\nname = \"wallet\"\nkind = \"domain\"\nlanguage = \"rust\"\n\n\
         [[component]]\nname = \"pmg-buyin\"\nkind = \"domain\"\nlanguage = \"python\"\n",
    )
    .unwrap();
    app_into(&out, &config, BUYIN, &[&fixtures()]).unwrap();
    for file in [
        "buyin.v1.rs",
        "buyin.v1.domain.rs",
        "angzarr-adapters.rs",
        "angzarr-descriptors.rs",
        "angzarr-register.rs",
    ] {
        assert!(out.join(file).exists(), "{file}");
    }
    let register = std::fs::read_to_string(out.join("angzarr-register.rs")).unwrap();
    assert!(register.contains("adapters::Wallet"), "{register}");
    // The Python child's process manager, listed by its name, is its
    // domain's contract.
    let adapters = std::fs::read_to_string(out.join("angzarr-adapters.rs")).unwrap();
    assert!(
        adapters.contains("`buyin.v1.BuyInState`, served by a child process"),
        "{adapters}"
    );
    assert!(
        adapters.contains("`buyin.v1.WalletState`, delegating to"),
        "{adapters}"
    );
    assert!(
        !register.contains("adapters::BuyIn>"),
        "the buy-in is the Python child's: {register}"
    );
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn a_config_that_cannot_be_read_is_an_error_not_a_new_config() {
    let root = empty_dir();
    std::fs::create_dir_all(root.join("angzarr.toml")).unwrap();
    let refused = app_scaffold_into(&root, SHOP, &[&fixtures()], &seed());
    assert!(matches!(refused, Err(BuildError::Io { .. })), "{refused:?}");
    std::fs::remove_dir_all(&root).unwrap();
}
