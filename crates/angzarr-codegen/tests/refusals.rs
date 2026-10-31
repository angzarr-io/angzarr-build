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
