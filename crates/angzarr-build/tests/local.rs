//! End to end on angzarr-local: `tests/local/buyin-app`, an application
//! whose domain layer and `register()` are generated from the buy-in
//! fixture protos and whose components are the scaffold with its bodies
//! written, is built against angzarr-embedded and angzarr-router and run: one
//! command, a policy step each way, and a refusal translated by a policy.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use angzarr_build::{Existing, Scaffolded, Translate, scaffold_into};

const BUYIN: &[&str] = &[
    "buyin/v1/wallet.proto",
    "buyin/v1/buy_in.proto",
    "buyin/v1/policies.proto",
];
const COMPONENTS: &[&str] = &[
    "buy_in.rs",
    "buy_in_wallet.rs",
    "wallet.rs",
    "wallet_buy_in.rs",
];

fn app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/local/buyin-app")
}

fn fixtures() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../angzarr-codegen/tests/fixtures")
        .display()
        .to_string()
}

/// The app's refusal switch: BuyInWallet translates Hold into HoldLost.
fn translate() -> Vec<Translate> {
    vec![Translate {
        policy: "buyin.v1.BuyInWalletPolicy".to_string(),
        command: "buyin.v1.Hold".to_string(),
        fact: "buyin.v1.HoldLost".to_string(),
    }]
}

/// The app's components are the scaffold of today's protos: completing a
/// copy of them adds nothing. (So the app below runs the scaffold's shape,
/// not a hand-made one.)
#[test]
fn test_the_apps_components_are_the_complete_scaffold() {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let src = std::env::temp_dir().join(format!(
        "angzarr-build-local-test-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&src).unwrap();
    for file in COMPONENTS {
        std::fs::copy(app().join("src").join(file), src.join(file)).unwrap();
    }
    let mut done = scaffold_into(
        &src,
        BUYIN,
        &[&fixtures()],
        &translate(),
        Existing::Complete,
    )
    .unwrap();
    done.sort_by_key(|d| format!("{d:?}"));
    let kept: Vec<Scaffolded> = COMPONENTS
        .iter()
        .map(|file| Scaffolded::Kept(src.join(file)))
        .collect();
    assert_eq!(done, kept);
    std::fs::remove_dir_all(&src).unwrap();
}

#[test]
#[ignore = "runs once angzarr-embedded step 2 (angzarr-ports) and angzarr-embedded's dispatch through \
            angzarr-router land: re-pin tests/local/buyin-app/Cargo.toml then"]
fn test_the_generated_register_runs_a_command_a_policy_step_and_a_refusal() {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/buyin-app");
    let out = Command::new(env!("CARGO"))
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(app().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("cargo runs");
    assert!(
        out.status.success(),
        "the app builds and runs:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        [
            // One command.
            "wallet balance=100 held=0",
            // BuyInStarted -> Hold (BuyInWallet); Held -> RecordHeld (WalletBuyIn).
            "wallet balance=100 held=30",
            "buyin held=30 failed=false",
            // Hold refused, recording nothing; translated into HoldLost.
            "wallet balance=100 held=30",
            "buyin held=0 failed=true",
            "",
        ]
        .join("\n")
    );
}
