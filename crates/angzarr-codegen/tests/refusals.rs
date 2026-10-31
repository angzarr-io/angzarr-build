//! Typed refusals (decision 14.50): a refusal's diagnostic key from its
//! full name and its `Reason`, its args the fields that are set, and a
//! command refusing several types. The fixture is
//! `kinds/v1/kinds.proto`, one refusal per field kind; the generated layer
//! is pinned by golden files, which `angzarr-build`'s `tests/local`
//! kinds-app type-checks against angzarr-embedded.

mod common;

use angzarr_codegen::{CodegenError, adapters, markup_domain, refusal_key, scaffold};
use common::*;

const KINDS: &str = "kinds/v1/kinds.proto";
const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");

/// `actual` is the golden file at `path` (rewritten under `ANGZARR_BLESS`).
fn golden(path: &str, actual: &str) {
    let path = format!("{GOLDEN}/{path}");
    if std::env::var_os("ANGZARR_BLESS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("no {path}"));
    assert_eq!(actual, expected, "{path}");
}

fn kinds_domain() -> String {
    let generated = markup_domain(
        &descriptors(&[KINDS]),
        &[KINDS],
        &[("kinds.v1", "crate::wire")],
    )
    .unwrap();
    let [one] = generated.as_slice() else {
        panic!("one package expected");
    };
    one.source.clone()
}

#[test]
fn test_the_kinds_domain_layer_matches_the_golden_file() {
    golden("kinds.domain.rs", &kinds_domain());
}

#[test]
fn test_the_kinds_adapters_match_the_golden_file() {
    let source = adapters(&descriptors(&[KINDS]), &[KINDS], &[], &|_| None).unwrap();
    golden("kinds.adapters.rs", &source);
}

#[test]
fn test_the_kinds_scaffold_matches_the_golden_files() {
    for file in scaffold(&descriptors(&[KINDS]), &[KINDS], &[]).unwrap() {
        golden(&format!("rust/kinds/{}", file.path), &file.source);
    }
}

#[test]
fn test_a_refusal_key_is_its_full_name_then_its_reason_in_kebab_case() {
    assert_eq!(
        refusal_key("refusals.v1.BetRefused", Some("ALREADY_BET")),
        "refusals-v1-betrefused-already-bet"
    );
    assert_eq!(
        refusal_key("io.angzarr.examples.v1.SeatNotConfirmable", None),
        "io-angzarr-examples-v1-seatnotconfirmable"
    );
}

#[test]
fn test_a_command_refusing_several_types_refuses_with_their_choice() {
    let files = scaffold(&descriptors(&[KINDS]), &[KINDS], &[]).unwrap();
    let interface = files
        .iter()
        .find(|f| f.path == "ledger_state.rs")
        .expect("ledger_state.rs");
    assert!(
        interface.source.contains(
            "fn post(&self, cmd: &Post, ctx: &Context) -> Result<Posted, Failed<PostRefuses>>;"
        ),
        "{}",
        interface.source
    );
    let adapters = adapters(&descriptors(&[KINDS]), &[KINDS], &[], &|_| None).unwrap();
    assert!(
        adapters.contains(".map_err(PostRefuses::widen)"),
        "{adapters}"
    );
}

#[test]
fn test_markup_domain_refuses_a_type_named_as_a_refusal_choice() {
    const FILE: &str = "bad/v1/refusal_choice.proto";
    assert_eq!(
        markup_domain(&descriptors(&[FILE]), &[FILE], &[("bad.v1", "crate::wire")])
            .expect_err("refused"),
        CodegenError::RefusalChoiceName {
            message: "bad.v1.BookRefuses".to_string(),
            name: "BookRefuses".to_string(),
            command: "bad.v1.Book".to_string(),
        }
    );
}

#[test]
fn test_a_command_refusing_one_type_has_no_choice_to_collide_with() {
    const FILE: &str = "lone/v1/lone.proto";
    let generated = markup_domain(
        &descriptors(&[FILE]),
        &[FILE],
        &[("lone.v1", "crate::wire")],
    )
    .unwrap();
    assert!(generated[0].source.contains("pub struct BookRefuses"));
}

/// A typed refusal with a Reason is refused under its code's status
/// (decision 14.52): the value's `refusal_status`, FAILED_PRECONDITION
/// where unmarked. The domain layer says which (`status`), for a
/// hand-written handler too, and the adapter refuses under it, for one
/// refusal type and for a choice of several.
#[test]
fn test_a_reasons_status_is_its_values_refusal_status() {
    let domain = kinds_domain();
    let file = syn::parse_file(&domain).unwrap();
    let status =
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Impl(imp)
                    if text(&imp.self_ty) == "PostRefused"
                        && imp.trait_.is_none()
                        && imp.items.iter().any(
                            |i| matches!(i, syn::ImplItem::Fn(f) if f.sig.ident == "status"),
                        ) =>
                {
                    Some(text(imp))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no PostRefused::status:\n{domain}"));
    assert!(
        status.contains(
            "pubfnstatus(&self)->BusinessStatus{matchself.reason{\
             PostRefusedReason::Closed=>BusinessStatus::FailedPrecondition,\
             PostRefusedReason::OverLimit=>BusinessStatus::OutOfRange,}}"
        ),
        "{status}"
    );
    // A refusal with no Reason has no code to take a status from: the
    // handler's own stands.
    assert!(!domain.contains("impl Unreasoned {"), "{domain}");
    let adapters = adapters(&descriptors(&[KINDS]), &[KINDS], &[], &|_| None).unwrap();
    for refused in [
        ".map_err(PostRefuses::with_code_status).map_err(PostRefuses::widen)",
        ".map_err(PostRefused::with_code_status).map_err(Failed::widen)",
    ] {
        assert!(
            adapters
                .split_whitespace()
                .collect::<String>()
                .contains(refused),
            "{refused}:\n{adapters}"
        );
    }
}

/// The status option marks a refusal's code alone (C-0746): on any other
/// enum's value, on a Reason's zero value, or on the Reason of a message
/// no command refuses with, generation fails naming the value.
#[test]
fn test_a_refusal_status_anywhere_but_a_refusals_code_is_refused() {
    for (file, value) in [
        (
            "bad/v1/refusal_status_elsewhere.proto",
            "bad.v1.Suit.SUIT_HEARTS",
        ),
        (
            "bad/v1/refusal_status_zero.proto",
            "bad.v1.PokeRefused.Reason.REASON_UNSPECIFIED",
        ),
        (
            "bad/v1/refusal_status_unrefused.proto",
            "bad.v1.Poked.Reason.REASON_GONE",
        ),
    ] {
        assert_eq!(
            markup_domain(&descriptors(&[file]), &[file], &[("bad.v1", "crate::wire")])
                .expect_err(file),
            CodegenError::RefusalStatusMisplaced {
                value: value.to_string()
            },
            "{file}"
        );
    }
}
