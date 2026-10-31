//! Projectors (decision 14.50): one projector of many source domains,
//! `input_domains` (or one `input_domain`), each held to its `(event)`
//! entries; an async user interface; one `#[projector]` over every
//! source; registered with the rest. The kinds fixture's LedgerView is
//! pinned by `tests/refusals.rs`'s golden files and run on
//! angzarr-embedded by angzarr-build's kinds-app.

mod common;

use angzarr_codegen::{CodegenError, adapters, descriptors, markup_domain, register, scaffold};
use angzarr_proto::v1::ComponentDescriptor;
use prost::Message;

const KINDS: &str = "kinds/v1/kinds.proto";

fn refusal(file: &str) -> CodegenError {
    markup_domain(
        &common::descriptors(&[file]),
        &[file],
        &[("bad.v1", "crate::wire")],
    )
    .expect_err("refused")
}

#[test]
fn test_a_projector_subscribes_to_its_input_domains() {
    let set = common::descriptors(&[KINDS]);
    let view = descriptors(&set, &[KINDS], "1")
        .unwrap()
        .into_iter()
        .find(|d| d.component == "kinds.v1.LedgerView")
        .expect("LedgerView's descriptor");
    let decoded = ComponentDescriptor::decode(view.encoded.as_slice()).unwrap();
    let inputs: Vec<&str> = decoded.inputs.iter().map(|t| t.domain.as_str()).collect();
    assert_eq!(inputs, ["ledger", "audit"]);
}

#[test]
fn test_the_projector_adapter_projects_every_source_under_its_name() {
    let source = adapters(&common::descriptors(&[KINDS]), &[KINDS], &[], &|_| None).unwrap();
    assert!(
        source.contains(
            "#[projector(name = \"projector-ledger-view\", sources = [\"ledger\", \"audit\"])]"
        ),
        "{source}"
    );
    assert!(source.contains("#[projects(Audited)]"), "{source}");
    let register = register(&common::descriptors(&[KINDS]), &[KINDS], &[]).unwrap();
    assert!(
        register.contains("registry.projector(crate::adapters::LedgerView::default());"),
        "{register}"
    );
}

#[test]
fn test_one_input_domain_is_the_one_entry_list() {
    const ONE: &str = "kinds/v1/one_source.proto";
    let source = adapters(&common::descriptors(&[ONE]), &[ONE], &[], &|_| None).unwrap();
    assert!(
        source.contains("#[projector(name = \"projector-thing-view\", sources = [\"things\"])]"),
        "{source}"
    );
    let files = scaffold(&common::descriptors(&[ONE]), &[ONE], &[]).unwrap();
    assert!(files.iter().any(|f| f.path == "thing_view_impl.rs"));
}

#[test]
fn test_input_domains_are_refused_off_a_projector() {
    assert_eq!(
        refusal("bad/v1/input_domains_on_policy.proto"),
        CodegenError::InputDomainsOnNonProjector {
            component: "bad.v1.ThingView".to_string(),
        }
    );
}

#[test]
fn test_input_domains_and_input_domain_together_are_refused() {
    assert_eq!(
        refusal("bad/v1/input_domains_and_input_domain.proto"),
        CodegenError::InputDomainsWithInputDomain {
            component: "bad.v1.ThingView".to_string(),
        }
    );
}

#[test]
fn test_a_source_listed_twice_is_refused() {
    assert_eq!(
        refusal("bad/v1/duplicate_source.proto"),
        CodegenError::DuplicateSource {
            component: "bad.v1.ThingView".to_string(),
            domain: "things".to_string(),
        }
    );
}

#[test]
fn test_a_listed_source_no_entry_names_is_refused() {
    assert_eq!(
        refusal("bad/v1/unprojected_source.proto"),
        CodegenError::UnprojectedSource {
            component: "bad.v1.ThingView".to_string(),
            domain: "others".to_string(),
        }
    );
}

#[test]
fn test_an_entry_naming_an_unlisted_or_no_source_is_refused() {
    let unlisted = |event: &str, domain: &str| CodegenError::UnlistedSource {
        component: "bad.v1.ThingView".to_string(),
        event: format!("bad.v1.{event}"),
        domain: domain.to_string(),
    };
    assert_eq!(
        refusal("bad/v1/unlisted_source.proto"),
        unlisted("Poked", "things")
    );
    assert_eq!(refusal("bad/v1/unset_source.proto"), unlisted("Poked", ""));
    assert_eq!(
        refusal("bad/v1/unset_of_one.proto"),
        unlisted("Prodded", "others")
    );
}
