//! The file side of the scaffold and of the generated layer: the scaffold
//! (each component's interface and implementation) goes into the user's
//! `src` once and is never overwritten, `Existing::Complete` (`--missing`)
//! adds only absent methods, and the adapters and `register` go into the
//! output directory, regenerated every build.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use angzarr_build::{
    BuildError, BuildWarning, Existing, Scaffolded, Translate, adapters_from, adapters_into,
    markup_domain_into, register_into, scaffold_into,
};
use angzarr_codegen::CodegenError;

const BUYIN: &[&str] = &[
    "buyin/v1/wallet.proto",
    "buyin/v1/buy_in.proto",
    "buyin/v1/policies.proto",
];
const FILES: &[&str] = &[
    "buy_in.rs",
    "buy_in_impl.rs",
    "buy_in_wallet.rs",
    "buy_in_wallet_impl.rs",
    "wallet.rs",
    "wallet_buy_in.rs",
    "wallet_buy_in_impl.rs",
    "wallet_impl.rs",
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
        "angzarr-build-scaffold-test-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn scaffold(src: &Path, existing: Existing) -> Vec<Scaffolded> {
    let mut done = scaffold_into(src, BUYIN, &[&fixtures()], &[], existing).unwrap();
    done.sort_by_key(|d| format!("{d:?}"));
    done
}

#[test]
fn test_scaffold_into_writes_each_absent_file() {
    let src = empty_dir();
    let created: Vec<Scaffolded> = FILES
        .iter()
        .map(|f| Scaffolded::Created(src.join(f)))
        .collect();
    assert_eq!(scaffold(&src, Existing::Keep), created);
    for file in FILES {
        let source = std::fs::read_to_string(src.join(file)).expect(file);
        syn::parse_file(&source).expect("the scaffold parses");
    }
    std::fs::remove_dir_all(&src).unwrap();
}

#[test]
fn test_scaffold_into_never_overwrites_a_users_file() {
    let src = empty_dir();
    scaffold(&src, Existing::Keep);
    let mine = "//! The user's wallet, nothing like the scaffold.\n";
    std::fs::write(src.join("wallet.rs"), mine).unwrap();
    let done = scaffold(&src, Existing::Keep);
    assert!(
        done.contains(&Scaffolded::Kept(src.join("wallet.rs"))),
        "{done:?}"
    );
    assert_eq!(
        std::fs::read_to_string(src.join("wallet.rs")).unwrap(),
        mine
    );
    std::fs::remove_dir_all(&src).unwrap();
}

#[test]
fn test_scaffold_into_with_complete_adds_only_absent_stubs() {
    let src = empty_dir();
    scaffold(&src, Existing::Keep);
    let path = src.join("buy_in_wallet.rs");
    let before = std::fs::read_to_string(&path).unwrap();
    // The switch now translates Hold for BuyInWallet: its refused method is
    // the one method absent, from the interface and the implementation.
    let translate = [Translate {
        policy: "buyin.v1.BuyInWalletPolicy".to_string(),
        command: "buyin.v1.Hold".to_string(),
        fact: "buyin.v1.HoldLost".to_string(),
    }];
    let mut done =
        scaffold_into(&src, BUYIN, &[&fixtures()], &translate, Existing::Complete).unwrap();
    done.sort_by_key(|d| format!("{d:?}"));
    let mut expected: Vec<Scaffolded> = FILES
        .iter()
        .map(|f| match *f {
            "buy_in_wallet.rs" | "buy_in_wallet_impl.rs" => Scaffolded::Completed(src.join(f)),
            _ => Scaffolded::Kept(src.join(f)),
        })
        .collect();
    expected.sort_by_key(|d| format!("{d:?}"));
    assert_eq!(done, expected);
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("fn hold_refused("), "{after}");
    assert!(after.len() > before.len());
    std::fs::remove_dir_all(&src).unwrap();
}

#[test]
fn test_scaffold_into_with_complete_refuses_an_input_the_proto_contradicts() {
    let src = empty_dir();
    scaffold(&src, Existing::Keep);
    let path = src.join("buy_in.rs");
    // The user took HoldLost, a fact, as a command.
    let mine = std::fs::read_to_string(&path)
        .unwrap()
        .replace("fact: &HoldLost", "cmd: &HoldLost");
    std::fs::write(&path, &mine).unwrap();
    let error = scaffold_into(&src, BUYIN, &[&fixtures()], &[], Existing::Complete)
        .expect_err("a mismatched marker");
    assert!(
        matches!(
            &error,
            BuildError::Codegen(CodegenError::InputMismatch { message, .. })
                if message == "buyin.v1.HoldLost"
        ),
        "{error:?}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), mine);
    std::fs::remove_dir_all(&src).unwrap();
}

#[test]
fn test_register_into_writes_the_generated_register() {
    let out = empty_dir();
    register_into(&out, BUYIN, &[&fixtures()], &[]).unwrap();
    let source = std::fs::read_to_string(out.join("angzarr-register.rs")).expect("register");
    assert_eq!(
        source,
        include_str!("../../angzarr-codegen/tests/golden/buyin.register.rs")
    );
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_markup_domain_into_writes_the_markup_domain_layer_by_package() {
    let out = empty_dir();
    markup_domain_into(
        &out,
        BUYIN,
        &[&fixtures()],
        &[("buyin.v1", "::buyin_proto::v1")],
    )
    .unwrap();
    let source = std::fs::read_to_string(out.join("buyin.v1.domain.rs")).expect("domain file");
    assert!(
        source.contains("::angzarr_embedded::next::markup::DomainMessage"),
        "{source}"
    );
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_adapters_into_writes_the_generated_adapters() {
    let out = empty_dir();
    adapters_into(&out, BUYIN, &[&fixtures()], &[]).unwrap();
    let source = std::fs::read_to_string(out.join("angzarr-adapters.rs")).expect("adapters");
    // Under the licence header's line (owner ruling 2026-10-31).
    let (header, source) = source.split_once('\n').unwrap();
    assert!(
        header.starts_with("// Generated by angzarr-cli"),
        "{header}"
    );
    assert!(
        source.starts_with("// @generated by angzarr-codegen"),
        "{source}"
    );
    assert!(
        source.contains("pub struct Wallet(pub crate::wallet_impl::WalletImpl);"),
        "{source}"
    );
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_scaffold_into_rescaffolded_leaves_every_users_file_byte_identical() {
    // A scaffold run after the first, with --missing, over files a user has
    // written in: none changes, because nothing is absent.
    let src = empty_dir();
    scaffold(&src, Existing::Keep);
    for file in FILES {
        let path = src.join(file);
        let mine = std::fs::read_to_string(&path)
            .unwrap()
            .replace("todo!()", "todo!(\"mine\")");
        std::fs::write(&path, mine).unwrap();
    }
    let before: Vec<String> = FILES
        .iter()
        .map(|f| std::fs::read_to_string(src.join(f)).unwrap())
        .collect();
    let done = scaffold(&src, Existing::Complete);
    assert!(
        done.iter().all(|d| matches!(d, Scaffolded::Kept(_))),
        "{done:?}"
    );
    let after: Vec<String> = FILES
        .iter()
        .map(|f| std::fs::read_to_string(src.join(f)).unwrap())
        .collect();
    assert_eq!(before, after);
    std::fs::remove_dir_all(&src).unwrap();
}

/// A policy's interface the adapters read is not there: a warning naming
/// it, not silence. With the interfaces in place, none.
#[test]
fn test_adapters_from_warns_of_a_policys_interface_that_is_not_there() {
    let out = empty_dir();
    let src = empty_dir();
    let warnings = adapters_from(&out, &src, BUYIN, &[&fixtures()], &[]).unwrap();
    let missing = |file: &str| BuildWarning::InterfaceMissing {
        path: src.join(file).display().to_string(),
    };
    assert_eq!(
        warnings,
        [missing("buy_in_wallet.rs"), missing("wallet_buy_in.rs")]
    );
    assert!(out.join("angzarr-adapters.rs").exists());
    scaffold(&src, Existing::Keep);
    assert_eq!(
        adapters_from(&out, &src, BUYIN, &[&fixtures()], &[]).unwrap(),
        []
    );
    std::fs::remove_dir_all(&out).unwrap();
    std::fs::remove_dir_all(&src).unwrap();
}
