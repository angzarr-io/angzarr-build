//! The generated `register(&mut Registry)`: every component the protos
//! declare, its adapter registered with angzarr-embedded, so an application
//! calls one function; a domain served by a child process is left to the
//! host. Pinned by a golden file; the
//! end-to-end test (angzarr-build's `tests/local.rs`) runs it.

mod common;

use angzarr_codegen::register;
use common::*;

const GOLDEN: &str = include_str!("golden/buyin.register.rs");

fn buyin_register() -> String {
    register(&descriptors(BUYIN), BUYIN, &[]).unwrap()
}

#[test]
fn test_register_matches_the_golden_file() {
    assert_eq!(buyin_register(), GOLDEN);
}

/// Each registration in `register`'s body, as text without spacing.
fn registrations(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("register parses");
    let body = file
        .items
        .iter()
        .find_map(|i| match i {
            syn::Item::Fn(f) if f.sig.ident == "register" => Some(f),
            _ => None,
        })
        .expect("fn register");
    assert_eq!(
        // syn keeps `pub` in `vis`, apart from the signature.
        format!("{}{}", text(&body.vis), text(&body.sig)),
        "pubfnregister(registry:&mut::angzarr_embedded::next::Registry)"
    );
    body.block.stmts.iter().map(text).collect()
}

#[test]
fn test_register_registers_every_component_once_by_its_kind() {
    assert_eq!(
        registrations(&buyin_register()),
        [
            "registry.aggregate::<crate::adapters::Wallet>(crate::adapters::Wallet::default);",
            "registry.process_manager::<crate::adapters::BuyIn>(crate::adapters::BuyIn::default);",
            "registry.policy(crate::adapters::BuyInWallet::default());",
            "registry.policy(crate::adapters::WalletBuyIn::default());",
        ]
    );
}

#[test]
fn test_register_of_one_aggregate_registers_it_alone() {
    const SHOP: &str = "shop/v1/shop.proto";
    let source = register(&descriptors(&[SHOP]), &[SHOP], &[]).unwrap();
    assert_eq!(
        registrations(&source),
        ["registry.aggregate::<crate::adapters::Cart>(crate::adapters::Cart::default);"]
    );
}

#[test]
fn test_register_leaves_a_remote_domain_to_the_host_and_keeps_its_policies() {
    let source = register(&descriptors(BUYIN), BUYIN, &["buyin"]).unwrap();
    assert_eq!(
        registrations(&source),
        [
            "registry.aggregate::<crate::adapters::Wallet>(crate::adapters::Wallet::default);",
            "registry.policy(crate::adapters::BuyInWallet::default());",
            "registry.policy(crate::adapters::WalletBuyIn::default());",
        ]
    );
}

#[test]
fn test_register_leaves_out_a_policy_whose_domain_is_not_in_the_files() {
    // The blackjack table and its policies, without the wallet:
    // TableWalletPolicy has no target here, TablePlayerPolicy and
    // PlayerTablePolicy have the remote player.
    const FILES: &[&str] = &[
        "io/angzarr/examples/v1/table.proto",
        "io/angzarr/examples/v1/player.proto",
        "io/angzarr/examples/v1/policies.proto",
    ];
    let source = register(&descriptors(BLACKJACK), FILES, &["player"]).unwrap();
    assert_eq!(
        registrations(&source),
        [
            "registry.aggregate::<crate::adapters::TableAggregate,>(crate::adapters::TableAggregate::default);",
            "registry.policy(crate::adapters::TablePlayerPolicy::default());",
            "registry.policy(crate::adapters::PlayerTablePolicy::default());",
        ]
    );
}

/// rustc with warnings denied on `source` as a library, `::angzarr_embedded`
/// a stub holding only the `next::Registry` the signature names: the
/// diagnostics, if rustc refused it.
fn compile_denying_warnings(source: &str) -> Result<(), String> {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "angzarr-codegen-register-test-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let rustc = |args: &[&str]| {
        Command::new(&rustc)
            .current_dir(&dir)
            .args(["--edition=2021", "--crate-type=lib", "-Dwarnings"])
            .args(args)
            .output()
            .expect("rustc on PATH")
    };
    std::fs::write(dir.join("stub.rs"), "pub mod next { pub struct Registry; }").unwrap();
    let stub = rustc(&["--crate-name=angzarr_embedded", "stub.rs"]);
    assert!(
        stub.status.success(),
        "{}",
        String::from_utf8_lossy(&stub.stderr)
    );
    std::fs::write(dir.join("register.rs"), source).unwrap();
    let out = rustc(&[
        "--crate-name=generated",
        "--extern=angzarr_embedded=libangzarr_embedded.rlib",
        "register.rs",
    ]);
    std::fs::remove_dir_all(&dir).unwrap();
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[test]
fn test_register_with_every_domain_remote_compiles_with_warnings_denied() {
    // The one domain, carts, is remote: nothing is registered in-process.
    const SHOP: &str = "shop/v1/shop.proto";
    let source = register(&descriptors(&[SHOP]), &[SHOP], &["carts"]).unwrap();
    if let Err(diagnostics) = compile_denying_warnings(&source) {
        panic!("{source}\n{diagnostics}");
    }
}
