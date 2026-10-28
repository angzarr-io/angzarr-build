//! The domain layer for marked-up components: every domain type the
//! components reach, with its wire conversions and its annals-macros
//! `DomainMessage` impl, and nothing the markup generates itself.

mod common;

use angzarr_codegen::{Generated, markup_domain};
use common::*;

fn buyin() -> syn::File {
    let generated = markup_domain(&descriptors(BUYIN), BUYIN, BUYIN_WIRE).unwrap();
    let [Generated { package, source }] = generated.as_slice() else {
        panic!("one package expected, got {generated:#?}");
    };
    assert_eq!(package, "buyin.v1");
    syn::parse_file(source).expect("generated source parses")
}

/// `(trait, type)` of every impl in the file.
fn impls(file: &syn::File) -> Vec<(String, String)> {
    file.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Impl(imp) => Some((
                imp.trait_
                    .as_ref()
                    .map(|(path, _)| text(path))
                    .unwrap_or_default(),
                text(&imp.self_ty),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn test_markup_domain_implements_domain_message_for_every_input_event_and_refusal() {
    let impls = impls(&buyin());
    let mut messages: Vec<&str> = impls
        .iter()
        .filter(|(t, _)| t == "::annals::next::markup::DomainMessage")
        .map(|(_, ty)| ty.as_str())
        .collect();
    messages.sort();
    assert_eq!(
        messages,
        [
            "BuyInFailed",
            "BuyInHeld",
            "BuyInStarted",
            "Deposit",
            "Deposited",
            "Held",
            "Hold",
            "HoldLost",
            "InsufficientFunds",
            "RecordHeld",
            "StartBuyIn",
        ]
    );
}

#[test]
fn test_markup_domain_leaves_state_and_payloads_to_the_markup() {
    let impls = impls(&buyin());
    let left: Vec<&(String, String)> = impls
        .iter()
        .filter(|(t, _)| t == "::annals::next::State" || t == "::annals::next::Payload")
        .collect();
    assert_eq!(left, Vec::<&(String, String)>::new());
}

#[test]
fn test_markup_domain_accepts_policy_components() {
    // COMPONENT_KIND_POLICY marker messages are components, not refused as
    // a kind the generator does not emit, and carry no domain type.
    let generated = markup_domain(&descriptors(BUYIN), BUYIN, BUYIN_WIRE);
    let generated = generated.expect("policies are accepted");
    assert!(
        generated
            .iter()
            .all(|g| !g.source.contains("struct BuyInWalletPolicy")),
        "a policy's marker message is no domain type"
    );
    assert!(!generated.is_empty());
}
