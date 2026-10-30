//! The naming fixture (`tests/fixtures/naming`): an aggregate whose names
//! exercise the case conversions, whose command and fact each emit either
//! of two events, and a policy to a domain nothing owns.

mod common;

use angzarr_codegen::{ScaffoldFile, adapters, markup_domain, scaffold};
use common::*;

const NAMING: &str = "naming/v1/naming.proto";
const BOTH: &[&str] = &[NAMING, "naming/v1/policy.proto"];

fn files() -> Vec<ScaffoldFile> {
    scaffold(&descriptors(BOTH), BOTH, &[]).unwrap()
}

fn generated() -> String {
    let generated = markup_domain(
        &descriptors(BOTH),
        &[NAMING],
        &[("naming.v1", "::naming_proto::v1")],
    )
    .unwrap();
    generated[0].source.clone()
}

#[test]
fn test_names_are_snake_cased_at_each_word_an_acronym_and_a_digit() {
    let files = files();
    let interface = &files
        .iter()
        .find(|f| f.path == "order_book.rs")
        .unwrap_or_else(|| panic!("no order_book.rs: {files:#?}"))
        .source;
    for expected in [
        "fn assign_tracking_id(",
        "fn shipment_noted(",
        "fn apply_tracking_id_assigned(",
        "fn apply_level2_reached(",
    ] {
        assert!(interface.contains(expected), "{expected}:\n{interface}");
    }
}

/// `DELIVERY_STATUS_SHIPPED_LATE` of `DeliveryStatus` loses the enum's
/// screaming-cased name as its prefix, its words camel-cased.
#[test]
fn test_enum_values_lose_the_enums_screaming_cased_prefix() {
    let source = generated();
    assert!(source.contains("ShippedLate"), "{source}");
    assert!(!source.contains("DeliveryStatusShippedLate"), "{source}");
}

#[test]
fn test_a_policy_to_a_domain_nothing_owns_is_not_scaffolded() {
    let paths: Vec<String> = files().into_iter().map(|f| f.path).collect();
    assert!(
        !paths.iter().any(|p| p.starts_with("books_nowhere")),
        "{paths:?}"
    );
}

/// The `use angzarr_embedded_macros::{..}` line of `source`.
fn macros(source: &str) -> String {
    source
        .lines()
        .find(|l| l.contains("angzarr_embedded_macros::"))
        .unwrap_or_else(|| panic!("no macros import:\n{source}"))
        .to_string()
}

#[test]
fn test_adapters_of_an_aggregate_import_the_aggregate_markup_only() {
    let none = |_: &str| None;
    for remote in [&[][..], &["books"][..]] {
        let source = adapters(&descriptors(BOTH), BOTH, remote, &none).unwrap();
        let line = macros(&source);
        assert!(line.contains("aggregate"), "{remote:?}: {line}");
        assert!(!line.contains("process_manager"), "{remote:?}: {line}");
    }
}

#[test]
fn test_adapters_fold_a_decision_and_a_fact_of_several_events() {
    let none = |_: &str| None;
    let source = adapters(&descriptors(BOTH), BOTH, &[], &none).unwrap();
    let flat = source.replace([' ', '\n'], "");
    for expected in [
        "Emit::of(None::<TrackingIDAssigned>)",
        "Record::of(None::<TrackingIDAssigned>)",
    ] {
        assert!(flat.contains(expected), "{expected}:\n{source}");
    }
}
