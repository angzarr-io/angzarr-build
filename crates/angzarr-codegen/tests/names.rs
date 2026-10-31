//! A component's name in an application (decision 14.50):
//! `component_name`, else its kind's default, through the one public
//! helper the generator, the app config and hand-written code share.

mod common;

use angzarr_codegen::{
    CodegenError, ComponentName, NameKind, NameOf, component_name, component_names, kebab,
    markup_domain,
};
use common::*;

/// A component of `kind` with the domains and anchor every case shares,
/// declaring `declared` (empty: none).
fn of(kind: NameKind, declared: &'static str) -> NameOf<'static> {
    NameOf {
        kind,
        component_name: declared,
        domain: "buy-in",
        input_domain: "table",
        output_domain: "player",
        anchor: "LedgerProjection",
    }
}

#[test]
fn test_each_kind_defaults_its_name() {
    for (kind, name) in [
        (NameKind::Aggregate, "buy-in"),
        (NameKind::ProcessManager, "pmg-buy-in"),
        (NameKind::Policy, "policy-table-player"),
        (NameKind::Projector, "projector-ledger-projection"),
    ] {
        assert_eq!(component_name(&of(kind, "")).unwrap(), name, "{kind:?}");
    }
}

#[test]
fn test_a_declared_name_is_the_name() {
    for kind in [NameKind::Aggregate, NameKind::Projector] {
        assert_eq!(
            component_name(&of(kind, "projector-wallet-table-ledger")).unwrap(),
            "projector-wallet-table-ledger"
        );
    }
}

#[test]
fn test_a_name_is_lower_case_letters_digits_and_dashes_from_a_letter() {
    assert_eq!(
        component_name(&of(NameKind::Aggregate, "a1-b")).unwrap(),
        "a1-b"
    );
    for bad in ["Table", "table_1", "1table", "-table", "table!"] {
        assert_eq!(
            component_name(&of(NameKind::Aggregate, bad)),
            Err(CodegenError::ComponentName {
                component: "LedgerProjection".to_string(),
                name: bad.to_string(),
            }),
            "{bad}"
        );
    }
}

#[test]
fn test_kebab_case_splits_words_and_acronyms() {
    assert_eq!(kebab("HTTPServerView"), "http-server-view");
    assert_eq!(kebab("LedgerProjection"), "ledger-projection");
    assert_eq!(kebab("Level2Reached"), "level2-reached");
}

#[test]
fn test_component_names_are_the_blackjack_spec_names() {
    let names: Vec<(String, NameKind, String, String)> =
        component_names(&descriptors(BLACKJACK), BLACKJACK)
            .unwrap()
            .into_iter()
            .map(
                |ComponentName {
                     component,
                     kind,
                     name,
                     domain,
                 }| (component, kind, name, domain),
            )
            .collect();
    let at = |component: &str| {
        names
            .iter()
            .find(|n| n.0 == format!("io.angzarr.examples.v1.{component}"))
            .unwrap_or_else(|| panic!("{component}: {names:?}"))
    };
    let s = |v: &str| v.to_string();
    assert_eq!(
        at("TableState"),
        &(
            s("io.angzarr.examples.v1.TableState"),
            NameKind::Aggregate,
            s("table"),
            s("table")
        )
    );
    assert_eq!(at("BuyInState").2, "pmg-buy-in");
    assert_eq!(at("BuyInState").1, NameKind::ProcessManager);
    assert_eq!(at("TablePlayerPolicy").2, "policy-table-player");
    assert_eq!(at("TablePlayerPolicy").3, "");
    assert_eq!(at("LedgerProjection").2, "projector-wallet-table-ledger");
    assert_eq!(at("LedgerProjection").1, NameKind::Projector);
}

#[test]
fn test_generation_refuses_a_malformed_component_name() {
    const FILE: &str = "bad/v1/component_name.proto";
    assert_eq!(
        markup_domain(&descriptors(&[FILE]), &[FILE], &[("bad.v1", "crate::wire")])
            .expect_err("refused"),
        CodegenError::ComponentName {
            component: "bad.v1.ThingState".to_string(),
            name: "Things_1".to_string(),
        }
    );
}
