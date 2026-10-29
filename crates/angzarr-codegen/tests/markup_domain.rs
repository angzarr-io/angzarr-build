//! The domain layer for marked-up components: every domain type the
//! components reach, with its wire conversions and its angzarr-embedded-macros
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
        .filter(|(t, _)| t == "::angzarr_embedded::next::markup::DomainMessage")
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
        .filter(|(t, _)| {
            t == "::angzarr_embedded::next::State" || t == "::angzarr_embedded::next::Payload"
        })
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

#[test]
fn test_markup_domain_refuses_refuses_on_a_fact() {
    const FILE: &str = "bad/v1/refusing_fact.proto";
    assert_eq!(
        markup_domain(
            &descriptors(&[FILE]),
            &[FILE],
            &[("bad.v1", "::bad_proto::v1")]
        ),
        Err(angzarr_codegen::CodegenError::RefusesOnFact {
            message: "bad.v1.Happened".to_string(),
        })
    );
}

/// The names of the file's top-level enums and structs.
fn items(file: &syn::File) -> Vec<String> {
    file.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Enum(e) => Some(e.ident.to_string()),
            syn::Item::Struct(s) => Some(s.ident.to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn test_markup_domain_has_the_rejection_and_each_components_command_and_event_enums() {
    let names = items(&buyin());
    for name in [
        "Rejection",
        "WalletCommand",
        "WalletEvent",
        "BuyInCommand",
        "BuyInEvent",
    ] {
        assert!(names.contains(&name.to_string()), "{name}: {names:?}");
    }
}

#[test]
fn test_markup_domain_gives_a_typed_refusal_its_diagnostic() {
    assert!(impls(&buyin()).contains(&(
        "::angzarr_embedded_diagnostic::Diagnostic".to_string(),
        "InsufficientFunds".to_string()
    )));
}

#[test]
fn test_markup_domain_maps_a_nested_enum_after_its_message() {
    const FILES: &[&str] = &[
        "io/angzarr/examples/v1/table.proto",
        "io/angzarr/examples/v1/player.proto",
    ];
    let generated = markup_domain(
        &descriptors(BLACKJACK),
        FILES,
        &[("io.angzarr.examples.v1", "crate::wire")],
    )
    .expect("SeatOutcome.Outcome is mapped");
    let file = syn::parse_file(&generated[0].source).unwrap();
    assert!(
        items(&file).contains(&"SeatOutcomeOutcome".to_string()),
        "{:?}",
        items(&file)
    );
}

/// Only a component's state derives `Default`, and the markup domain layer
/// holds no state (the markup's own type is the state).
#[test]
fn test_markup_domain_derives_default_for_no_type() {
    let defaulted: Vec<String> = buyin()
        .items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Struct(s)
                if s.attrs
                    .iter()
                    .any(|a| a.path().is_ident("derive") && text(a).contains("Default")) =>
            {
                Some(s.ident.to_string())
            }
            _ => None,
        })
        .collect::<Vec<String>>();
    assert_eq!(defaulted, Vec::<String>::new());
}

#[test]
fn test_markup_domain_command_enum_holds_the_commands_not_the_facts() {
    let file = buyin();
    let variants: Vec<String> = file
        .items
        .iter()
        .find_map(|i| match i {
            syn::Item::Enum(e) if e.ident == "BuyInCommand" => {
                Some(e.variants.iter().map(|v| v.ident.to_string()).collect())
            }
            _ => None,
        })
        .expect("BuyInCommand");
    assert!(variants.contains(&"StartBuyIn".to_string()), "{variants:?}");
    assert!(!variants.contains(&"HoldLost".to_string()), "{variants:?}");
}
