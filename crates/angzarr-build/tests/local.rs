//! End to end on angzarr-embedded: `tests/local/buyin-app`, an application
//! whose domain layer, adapters and `register()` are generated from the
//! buy-in fixture protos and whose components are the scaffold's interfaces
//! and implementations with their bodies written, is type-checked against
//! angzarr-embedded, and run: one command, a policy step each way, and a
//! refusal translated by a policy.

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
    "buy_in_impl.rs",
    "buy_in_wallet.rs",
    "buy_in_wallet_impl.rs",
    "wallet.rs",
    "wallet_buy_in.rs",
    "wallet_buy_in_impl.rs",
    "wallet_impl.rs",
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
/// copy of them adds nothing. (So the app below is the scaffold's shape,
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

/// The generated adapters and register() compile against angzarr-embedded
/// with the app's interfaces and implementations.
#[test]
#[ignore = "builds angzarr-embedded and its dependencies: slow; run with --ignored"]
fn test_the_app_type_checks_against_angzarr_embedded() {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/buyin-app");
    let out = Command::new(env!("CARGO"))
        .args(["check", "--quiet", "--lib", "--manifest-path"])
        .arg(app().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("cargo runs");
    assert!(
        out.status.success(),
        "the app type-checks:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The generated register() is what angzarr_embedded_host::run takes: the
/// app, run from its angzarr.toml, handles one command, a policy step each
/// way, and a refusal its policy translates.
#[test]
#[ignore = "builds angzarr-embedded, angzarr-embedded-host and their dependencies, fetched from \
            the network on a cold cache: slow; run with --ignored"]
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

/// `tests/local/kinds-app`: the generated domain layer of a typed refusal
/// of each field kind and a command refusing several, inputs of each emits
/// cardinality and a projector of two domains, with their adapters, built
/// against angzarr-embedded and run: each refusal's code and args, then
/// commands whose events the generated projector and one written by hand
/// (registered through angzarr-embedded's public API beside the generated
/// register()) project and a generated policy reacts to, translating a
/// refusal of a command refusing two types as their choice; then the policy
/// swapped for one written by hand under the generated one's name
/// (`angzarr_build::component_names`): the same derived command id.
#[test]
#[ignore = "builds angzarr-embedded and its dependencies: slow; run with --ignored"]
fn test_the_kinds_app_refuses_records_and_projects_as_generated() {
    let app = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/local/kinds-app");
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/kinds-app");
    let out = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--manifest-path"])
        .arg(app.join("Cargo.toml"))
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
            "kinds-v1-postrefused-over-limit reason=OVER_LIMIT small=-1 big=2 count=3 total=4 \
             text=t data=ab01 flag=true ratio=0.5 share=1.5 tone=LOUD limit=9",
            "kinds-v1-postrefused-over-limit",
            "kinds-v1-unreasoned reason=LOUD",
            "kinds-v1-listed ",
            "kinds-v1-elsewhere why=AWAY",
            "kinds-v1-mislabeled reason=LATE",
            // Open: Posted and Noted; the view projects Posted, and Noted
            // sets the policy off, whose Audit the view projects too.
            "view: Posted from ledger (1)",
            "view: Audited from audit (2)",
            "tally: 2 events from ledger",
            "generated policy: 49bd00eef2815f39bc2a91179c4619f9",
            // Batch: Noted then Posted, one step each. The audit refuses
            // root 2 with one of Audit's two refusal types, and the
            // policy's callback is handed it as their choice, typed.
            "audit refused: Some(Unreasoned(Unreasoned { reason: Loud }))",
            "view: Posted from ledger (3)",
            "tally: 2 events from ledger",
            // Audit, the view's other source.
            "view: Audited from audit (4)",
            // The policy swapped for one written by hand under the
            // generated one's name: the same command id.
            "swapped",
            "view: Posted from ledger (1)",
            "view: Audited from audit (2)",
            "tally: 2 events from ledger",
            "hand-written policy: 49bd00eef2815f39bc2a91179c4619f9",
            "",
        ]
        .join("\n")
    );
}
