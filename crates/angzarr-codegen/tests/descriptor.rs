//! The generated `ComponentDescriptor` (spec decision 14.42,
//! `features/codegen/descriptor.feature`): the whole descriptor of each
//! component, generated at build time from its protos, with the build's
//! version as its rules version. Rust gets it in `OUT_DIR`
//! (`descriptors_rs`), a Python project in its generated `_descriptor.py`;
//! the business process answers `Describe` with it unchanged.

mod common;

use angzarr_codegen::{
    Descriptor, ProtoInput, PythonApp, descriptors, descriptors_rs, scaffold_python,
};
use angzarr_proto::v1::{ComponentDescriptor, ComponentKind, ConflictResolution, Target};
use common::*;
use prost::Message;

const ORDERS: &str = "orders/v1/orders.proto";
const FULFILLMENT: &str = "fulfillment/fulfillment.proto";
const FILES: &[&str] = &[ORDERS, FULFILLMENT];
const VERSION: &str = "1.4.2+build.7";

fn generated() -> Vec<Descriptor> {
    descriptors(&descriptors_set(), FILES, VERSION).unwrap()
}

fn descriptors_set() -> Vec<u8> {
    common::descriptors(FILES)
}

/// The descriptor of the component named `name`, decoded.
fn descriptor(name: &str) -> ComponentDescriptor {
    let all = generated();
    let found = all
        .iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("no descriptor of {name}"));
    ComponentDescriptor::decode(found.encoded.as_slice()).expect("an encoded ComponentDescriptor")
}

fn domains(inputs: &[Target]) -> Vec<&str> {
    inputs.iter().map(|t| t.domain.as_str()).collect()
}

// C-0715
#[test]
fn a_policy_descriptor_names_its_input_and_output_domain() {
    let policy = descriptor("OrderToInventory");
    assert_eq!(policy.kind, ComponentKind::Policy as i32);
    assert_eq!(domains(&policy.inputs), ["order"]);
    assert_eq!(policy.output_domain, "inventory");
    assert_eq!(policy.state, None);
    assert!(policy.starting_types.is_empty());
    assert_eq!(policy.rules_version, VERSION);
}

// C-0535
#[test]
fn the_generated_descriptor_carries_the_build_version_and_the_state_type() {
    let order = descriptor("order");
    assert_eq!(order.name, "order");
    assert_eq!(order.kind, ComponentKind::Aggregate as i32);
    assert_eq!(order.rules_version, VERSION);
    let state = order.state.expect("an aggregate's state");
    assert_eq!(state.type_url, "/orders.v1.OrderState");
    // The implicit base alone, in enum-number order.
    assert_eq!(
        state
            .options
            .expect("the resolved options")
            .conflict_resolutions,
        [
            ConflictResolution::Strict as i32,
            ConflictResolution::AggregateHandles as i32
        ]
    );
    // An aggregate subscribes to nothing.
    assert!(order.inputs.is_empty());
    assert!(order.starting_types.is_empty());
    assert_eq!(order.output_domain, "");
}

#[test]
fn a_listed_base_resolution_is_not_repeated() {
    // FulfillmentState lists STRICT, already in the base.
    let state = descriptor("Fulfillment").state.expect("a PM's state");
    assert_eq!(state.type_url, "/fulfillment.FulfillmentState");
    assert_eq!(
        state
            .options
            .expect("the resolved options")
            .conflict_resolutions,
        [
            ConflictResolution::Strict as i32,
            ConflictResolution::AggregateHandles as i32
        ]
    );
}

// C-0716
#[test]
fn only_a_policy_descriptor_carries_an_output_domain() {
    for (name, kind) in [
        ("order", ComponentKind::Aggregate),
        ("Fulfillment", ComponentKind::ProcessManager),
        ("OrderSummary", ComponentKind::Projector),
    ] {
        let d = descriptor(name);
        assert_eq!(d.kind, kind as i32, "{name}'s kind");
        assert_eq!(d.output_domain, "", "{name} carries no output domain");
    }
}

// C-0607, with the bare type names of decision 14.41.
#[test]
fn a_process_manager_descriptor_lists_its_starting_type_names() {
    let pm = descriptor("Fulfillment");
    assert_eq!(pm.starting_types, ["fulfillment.StartFulfillment"]);
    assert!(
        pm.inputs.is_empty(),
        "a process manager subscribes to nothing"
    );
    assert_eq!(pm.rules_version, VERSION);
}

#[test]
fn a_projector_descriptor_subscribes_to_its_events_domains_and_has_no_state() {
    let summary = descriptor("OrderSummary");
    assert_eq!(domains(&summary.inputs), ["order"]);
    assert_eq!(summary.state, None);
    assert!(summary.starting_types.is_empty());
}

#[test]
fn a_projector_descriptor_subscribes_to_its_input_domain_when_it_names_one() {
    assert_eq!(domains(&descriptor("AuditTrail").inputs), ["audit"]);
}

#[test]
fn every_component_has_one_descriptor_in_declaration_order() {
    let all: Vec<(String, String)> = generated()
        .into_iter()
        .map(|d| (d.component, d.name))
        .collect();
    assert_eq!(
        all,
        s(&[
            ("orders.v1.OrderState", "order"),
            ("orders.v1.OrderToInventoryPolicy", "OrderToInventory"),
            ("orders.v1.OrderSummaryView", "OrderSummary"),
            ("orders.v1.AuditTrailView", "AuditTrail"),
            ("fulfillment.FulfillmentState", "Fulfillment"),
        ])
    );
}

/// `descriptors_rs`'s `DESCRIPTORS`: each `(name, bytes)` entry.
fn rust_entries(source: &str) -> Vec<(String, Vec<u8>)> {
    let file = syn::parse_file(source).expect("the generated Rust parses");
    let constant = file
        .items
        .iter()
        .find_map(|i| match i {
            syn::Item::Const(c) if c.ident == "DESCRIPTORS" => Some(c),
            _ => None,
        })
        .expect("const DESCRIPTORS");
    assert_eq!(text(&constant.ty), "&[(&str,&[u8])]");
    let syn::Expr::Reference(slice) = &*constant.expr else {
        panic!("DESCRIPTORS is a slice reference");
    };
    let syn::Expr::Array(array) = &*slice.expr else {
        panic!("DESCRIPTORS is an array");
    };
    array
        .elems
        .iter()
        .map(|e| {
            let syn::Expr::Tuple(pair) = e else {
                panic!("an entry is a pair");
            };
            let lit = |i: usize| match &pair.elems[i] {
                syn::Expr::Lit(l) => l.lit.clone(),
                other => panic!("a literal, not {}", text(other)),
            };
            let (syn::Lit::Str(name), syn::Lit::ByteStr(bytes)) = (lit(0), lit(1)) else {
                panic!("a name and its bytes");
            };
            (name.value(), bytes.value())
        })
        .collect()
}

#[test]
fn the_rust_build_layer_holds_each_descriptor_unchanged() {
    let source = descriptors_rs(&descriptors_set(), FILES, &[], VERSION).unwrap();
    let expected: Vec<(String, Vec<u8>)> = generated()
        .into_iter()
        .map(|d| (d.name, d.encoded))
        .collect();
    assert_eq!(rust_entries(&source), expected);
}

#[test]
fn the_rust_build_layer_leaves_a_remote_domain_to_its_process() {
    // The order domain is served by a child process: its aggregate's
    // descriptor is that process's; the policy and the projectors stay.
    let source = descriptors_rs(&descriptors_set(), FILES, &["order"], VERSION).unwrap();
    let names: Vec<String> = rust_entries(&source).into_iter().map(|e| e.0).collect();
    assert_eq!(
        names,
        [
            "OrderToInventory",
            "OrderSummary",
            "AuditTrail",
            "Fulfillment"
        ]
    );
}

fn order_app() -> PythonApp {
    PythonApp {
        domain: "order".to_string(),
        protos: ProtoInput::Directory("proto".to_string()),
        rules_version: VERSION.to_string(),
    }
}

/// The bytes of `_descriptor.py`'s `bytes.fromhex(...)`: its hex strings,
/// concatenated.
fn python_bytes(source: &str) -> Vec<u8> {
    let start = source.find("bytes.fromhex(").expect("bytes.fromhex") + "bytes.fromhex(".len();
    let end = start + source[start..].find(')').expect("a closing paren");
    let hex: String = source[start..end].split('"').skip(1).step_by(2).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect()
}

// C-0565: what the business process answers Describe with is the
// generated descriptor, unchanged.
#[test]
fn the_python_project_holds_its_components_generated_descriptor() {
    let files = scaffold_python(&descriptors_set(), &[ORDERS], &order_app()).unwrap();
    let file = files
        .iter()
        .find(|f| f.path == "src/angzarr_order/_descriptor.py")
        .expect("_descriptor.py");
    assert_eq!(file.owner, angzarr_codegen::Owner::Generated);
    assert!(
        file.source
            .contains("DESCRIPTOR = _t.ComponentDescriptor.FromString("),
        "{}",
        file.source
    );
    let order = generated().into_iter().find(|d| d.name == "order").unwrap();
    assert_eq!(python_bytes(&file.source), order.encoded);
    for line in file.source.lines() {
        assert!(line.len() <= 88, "longer than Black's width: {line}");
    }
}
