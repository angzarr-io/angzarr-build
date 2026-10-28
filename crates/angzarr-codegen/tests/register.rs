//! The generated `register(&mut Registry)`: every component the protos
//! declare, registered with annals at the path its scaffold gives it, so an
//! application calls one function. Pinned by a golden file; the
//! end-to-end test (angzarr-build's `tests/local.rs`) runs it.

mod common;

use angzarr_codegen::register;
use common::*;

const GOLDEN: &str = include_str!("golden/buyin.register.rs");

fn buyin_register() -> String {
    register(&descriptors(BUYIN), BUYIN).unwrap()
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
        text(&body.sig),
        "pubfnregister(registry:&mut::annals::next::Registry)"
    );
    body.block.stmts.iter().map(text).collect()
}

#[test]
fn test_register_registers_every_component_once_by_its_kind() {
    assert_eq!(
        registrations(&buyin_register()),
        [
            "registry.aggregate::<crate::wallet::Wallet>(crate::wallet::Wallet::default);",
            "registry.process_manager::<crate::buy_in::BuyIn>(crate::buy_in::BuyIn::default);",
            "registry.policy(crate::buy_in_wallet::BuyInWallet);",
            "registry.policy(crate::wallet_buy_in::WalletBuyIn);",
        ]
    );
}

#[test]
fn test_register_of_one_aggregate_registers_it_alone() {
    const SHOP: &str = "shop/v1/shop.proto";
    let source = register(&descriptors(&[SHOP]), &[SHOP]).unwrap();
    assert_eq!(
        registrations(&source),
        ["registry.aggregate::<crate::cart::Cart>(crate::cart::Cart::default);"]
    );
}
