//! The file side of the scaffold and of `register`: the scaffold goes into
//! the user's `src` once and is never overwritten, `Existing::Complete`
//! (`--missing`) adds only absent stubs, and `register` goes into the
//! output directory, regenerated every build.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use angzarr_build::{
    Existing, Scaffolded, Translate, markup_domain_into, register_into, scaffold_into,
};

const BUYIN: &[&str] = &[
    "buyin/v1/wallet.proto",
    "buyin/v1/buy_in.proto",
    "buyin/v1/policies.proto",
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
    assert_eq!(
        scaffold(&src, Existing::Keep),
        [
            Scaffolded::Created(src.join("buy_in.rs")),
            Scaffolded::Created(src.join("buy_in_wallet.rs")),
            Scaffolded::Created(src.join("wallet.rs")),
            Scaffolded::Created(src.join("wallet_buy_in.rs")),
        ]
    );
    for file in [
        "buy_in.rs",
        "buy_in_wallet.rs",
        "wallet.rs",
        "wallet_buy_in.rs",
    ] {
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
    // The switch now translates Hold for BuyInWallet: its callback stub is
    // the one stub absent.
    let translate = [Translate {
        policy: "buyin.v1.BuyInWalletPolicy".to_string(),
        command: "buyin.v1.Hold".to_string(),
        fact: "buyin.v1.HoldLost".to_string(),
    }];
    let mut done =
        scaffold_into(&src, BUYIN, &[&fixtures()], &translate, Existing::Complete).unwrap();
    done.sort_by_key(|d| format!("{d:?}"));
    assert_eq!(
        done,
        [
            Scaffolded::Completed(src.join("buy_in_wallet.rs")),
            Scaffolded::Kept(src.join("buy_in.rs")),
            Scaffolded::Kept(src.join("wallet.rs")),
            Scaffolded::Kept(src.join("wallet_buy_in.rs")),
        ]
    );
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("on_refused(Hold)"), "{after}");
    assert!(after.len() > before.len());
    std::fs::remove_dir_all(&src).unwrap();
}

#[test]
fn test_register_into_writes_the_generated_register() {
    let out = empty_dir();
    register_into(&out, BUYIN, &[&fixtures()]).unwrap();
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
        source.contains("::annals::next::markup::DomainMessage"),
        "{source}"
    );
    std::fs::remove_dir_all(&out).unwrap();
}
