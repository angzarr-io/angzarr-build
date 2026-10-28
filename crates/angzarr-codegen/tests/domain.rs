//! The domain layer generated from a component's protos: one domain type per
//! message the component reaches, its wire conversions, payload enums of its
//! commands and events, and its state's annals impl. Whether the generated
//! code compiles and converts correctly is held by the crates that build on
//! it; these tests hold what is generated, read back as Rust items.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use angzarr_codegen::{CodegenError, Generated, domain};
use quote::ToTokens;

const SHOP: &str = "shop/v1/shop.proto";
const SHOP_WIRE: &[(&str, &str)] = &[("shop.v1", "::shop_proto::v1")];
const BAD_WIRE: &[(&str, &str)] = &[("bad.v1", "::bad_proto::v1")];

/// protoc's descriptor set for `files` under `tests/fixtures`, with
/// angzarr's protos importable.
fn descriptors(files: &[&str]) -> Vec<u8> {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "angzarr-codegen-test-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    let angzarr = angzarr_proto::protos::write_include_dir(&dir.join("angzarr")).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let out: PathBuf = dir.join("set.bin");
    let protoc = std::env::var("PROTOC").unwrap_or_else(|_| "protoc".to_string());
    let status = Command::new(protoc)
        .arg("--include_imports")
        .arg(format!("--descriptor_set_out={}", out.display()))
        .arg(format!("-I{}", fixtures.display()))
        .arg(format!("-I{}", angzarr.display()))
        .args(files)
        .status()
        .expect("protoc on PATH");
    assert!(status.success(), "protoc failed on {files:?}");
    let bytes = std::fs::read(&out).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    bytes
}

fn shop() -> syn::File {
    let generated = domain(&descriptors(&[SHOP]), &[SHOP], SHOP_WIRE).unwrap();
    let [Generated { package, source }] = generated.as_slice() else {
        panic!("one package expected, got {generated:#?}");
    };
    assert_eq!(package, "shop.v1");
    syn::parse_file(source).expect("generated source parses")
}

fn refusal(file: &str) -> CodegenError {
    domain(&descriptors(&[file]), &[file], BAD_WIRE).expect_err("refused")
}

/// Tokens as text without spacing, for comparing types.
fn text(tokens: impl ToTokens) -> String {
    tokens.to_token_stream().to_string().replace(' ', "")
}

fn item_struct<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemStruct {
    file.items
        .iter()
        .find_map(|i| match i {
            syn::Item::Struct(s) if s.ident == name => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no struct {name}"))
}

fn fields(file: &syn::File, name: &str) -> Vec<(String, String)> {
    item_struct(file, name)
        .fields
        .iter()
        .map(|f| (f.ident.as_ref().unwrap().to_string(), text(&f.ty)))
        .collect()
}

fn variants(file: &syn::File, name: &str) -> Vec<(String, String)> {
    let e = file
        .items
        .iter()
        .find_map(|i| match i {
            syn::Item::Enum(e) if e.ident == name => Some(e),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no enum {name}"));
    e.variants
        .iter()
        .map(|v| (v.ident.to_string(), text(&v.fields)))
        .collect()
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

fn s(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

#[test]
fn test_domain_emits_payload_enums_of_a_components_commands_and_events() {
    let file = shop();
    assert_eq!(
        variants(&file, "CartCommand"),
        s(&[("AddLine", "(AddLine)"), ("Checkout", "(Checkout)")])
    );
    assert_eq!(
        variants(&file, "CartEvent"),
        s(&[("LineAdded", "(LineAdded)"), ("CheckedOut", "(CheckedOut)")])
    );
    let impls = impls(&file);
    for payload in ["CartCommand", "CartEvent"] {
        assert!(
            impls.contains(&("::annals::next::Payload".to_string(), payload.to_string())),
            "{payload} implements Payload: {impls:?}"
        );
    }
}

#[test]
fn test_domain_names_the_components_stream_domain() {
    let file = shop();
    let constant = file
        .items
        .iter()
        .find_map(|i| match i {
            syn::Item::Const(c) if c.ident == "CART_DOMAIN" => Some(text(&c.expr)),
            _ => None,
        })
        .expect("CART_DOMAIN");
    assert_eq!(constant, "\"carts\"");
}

#[test]
fn test_domain_message_fields_are_required_and_optional_scalars_stay_optional() {
    let file = shop();
    assert_eq!(
        fields(&file, "CartState"),
        s(&[
            ("owner", "String"),
            ("lines", "Vec<Line>"),
            ("totals", "::std::collections::BTreeMap<String,i64>"),
            ("discount", "Option<i32>"),
            ("opened_at", "::annals::next::Timestamp"),
            ("by_sku", "::std::collections::BTreeMap<String,Line>"),
        ])
    );
}

#[test]
fn test_domain_enums_are_closed_and_omit_the_zero_value() {
    let file = shop();
    assert_eq!(
        fields(&file, "Line"),
        s(&[
            ("sku", "String"),
            ("count", "i64"),
            ("status", "Status"),
            ("history", "Vec<Status>"),
            ("last", "Option<Status>"),
            ("note", "Vec<u8>"),
        ])
    );
    assert_eq!(
        variants(&file, "Status"),
        s(&[("Open", ""), ("Shipped", "")])
    );
}

#[test]
fn test_domain_converts_each_type_to_and_from_its_wire_message() {
    let impls = impls(&shop());
    for name in [
        "CartState",
        "Line",
        "AddLine",
        "Checkout",
        "LineAdded",
        "CheckedOut",
    ] {
        let wire = format!("::shop_proto::v1::{name}");
        assert!(
            impls.contains(&(format!("From<{name}>"), wire.clone())),
            "{name} into {wire}: {impls:?}"
        );
        assert!(
            impls.contains(&(format!("TryFrom<{wire}>"), name.to_string())),
            "{wire} into {name}: {impls:?}"
        );
    }
    assert!(impls.contains(&("From<Status>".to_string(), "i32".to_string())));
    assert!(impls.contains(&("TryFrom<i32>".to_string(), "Status".to_string())));
}

#[test]
fn test_domain_implements_annals_state_for_the_anchor_only() {
    let impls = impls(&shop());
    let states: Vec<&String> = impls
        .iter()
        .filter(|(t, _)| t == "::annals::next::State")
        .map(|(_, ty)| ty)
        .collect();
    assert_eq!(states, ["CartState"]);
}

#[test]
fn test_domain_without_a_component_generates_nothing() {
    let generated = domain(
        &descriptors(&["io/angzarr/v1/options.proto"]),
        &["io/angzarr/v1/options.proto"],
        &[],
    )
    .unwrap();
    assert_eq!(generated, Vec::new());
}

#[test]
fn test_domain_refuses_a_command_naming_an_unknown_component() {
    assert_eq!(
        refusal("bad/v1/unknown_component.proto"),
        CodegenError::UnknownComponent {
            message: "bad.v1.Poke".to_string(),
            component: "bad.v1.Nowhere".to_string(),
        }
    );
}

#[test]
fn test_domain_refuses_a_command_emitting_a_non_event() {
    assert_eq!(
        refusal("bad/v1/unknown_emit.proto"),
        CodegenError::UnknownEmit {
            command: "bad.v1.Poke".to_string(),
            event: "bad.v1.Poke".to_string(),
        }
    );
}

#[test]
fn test_domain_refuses_an_enum_whose_zero_is_a_value() {
    assert_eq!(
        refusal("bad/v1/enum_zero.proto"),
        CodegenError::EnumZero {
            enum_name: "bad.v1.Mood".to_string(),
            value: "MOOD_CALM".to_string(),
        }
    );
}

#[test]
fn test_domain_refuses_a_oneof() {
    assert_eq!(
        refusal("bad/v1/oneof.proto"),
        CodegenError::Oneof {
            message: "bad.v1.ThingState".to_string(),
            oneof: "shape".to_string(),
        }
    );
}

#[test]
fn test_domain_refuses_a_nested_type() {
    assert_eq!(
        refusal("bad/v1/nested.proto"),
        CodegenError::NestedType {
            message: "bad.v1.ThingState".to_string(),
            nested: "bad.v1.ThingState.Inner".to_string(),
        }
    );
}

#[test]
fn test_domain_refuses_a_commutative_state() {
    assert_eq!(
        refusal("bad/v1/commutative.proto"),
        CodegenError::Commutative {
            component: "bad.v1.ThingState".to_string(),
        }
    );
}

#[test]
fn test_domain_refuses_a_component_kind_it_does_not_emit() {
    assert_eq!(
        refusal("bad/v1/projector.proto"),
        CodegenError::UnsupportedKind {
            component: "bad.v1.WatchState".to_string(),
            kind: 4,
        }
    );
}

#[test]
fn test_domain_refuses_refuses_on_a_fact() {
    assert_eq!(
        refusal("bad/v1/refusing_fact.proto"),
        CodegenError::RefusesOnFact {
            message: "bad.v1.Happened".to_string(),
        }
    );
}

#[test]
fn test_domain_refuses_a_package_without_a_wire_path() {
    assert_eq!(
        domain(&descriptors(&[SHOP]), &[SHOP], &[]),
        Err(CodegenError::NoWirePath {
            package: "shop.v1".to_string(),
        })
    );
}

#[test]
fn test_domain_refuses_a_file_not_in_the_set() {
    assert_eq!(
        domain(&descriptors(&[SHOP]), &["shop/v1/absent.proto"], SHOP_WIRE),
        Err(CodegenError::FileNotInSet {
            file: "shop/v1/absent.proto".to_string(),
        })
    );
}

#[test]
fn test_domain_refuses_bytes_that_are_not_a_descriptor_set() {
    assert!(matches!(
        domain(&[0xff, 0xff], &[SHOP], SHOP_WIRE),
        Err(CodegenError::Descriptors { .. })
    ));
}

#[test]
fn test_every_error_has_a_catalog_message_with_its_args() {
    let ftl = include_str!("../locales/en-US/angzarr-codegen.ftl");
    let problems = angzarr_fmt::check(ftl, &angzarr_codegen::diagnostic_samples());
    assert!(problems.is_empty(), "{problems:#?}");
}
