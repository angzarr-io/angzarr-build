//! The domain layer for marked-up components: every domain type the
//! components reach, with its wire conversions and its angzarr-embedded-macros
//! `DomainMessage` impl, and nothing the markup generates itself; and what
//! the generator refuses. Whether the generated code compiles and converts
//! correctly is held by the crates that build on it (angzarr-build's local
//! app); these tests hold what is generated, read back as Rust items.

mod common;

use angzarr_codegen::{CodegenError, Generated, markup_domain};
use common::*;

const SHOP: &str = "shop/v1/shop.proto";
const SHOP_WIRE: &[(&str, &str)] = &[("shop.v1", "::shop_proto::v1")];
const BAD_WIRE: &[(&str, &str)] = &[("bad.v1", "::bad_proto::v1")];

fn buyin() -> syn::File {
    let generated = markup_domain(&descriptors(BUYIN), BUYIN, BUYIN_WIRE).unwrap();
    let [Generated { package, source }] = generated.as_slice() else {
        panic!("one package expected, got {generated:#?}");
    };
    assert_eq!(package, "buyin.v1");
    syn::parse_file(source).expect("generated source parses")
}

fn shop() -> syn::File {
    let generated = markup_domain(&descriptors(&[SHOP]), &[SHOP], SHOP_WIRE).unwrap();
    let [Generated { package, source }] = generated.as_slice() else {
        panic!("one package expected, got {generated:#?}");
    };
    assert_eq!(package, "shop.v1");
    syn::parse_file(source).expect("generated source parses")
}

fn refusal(file: &str) -> CodegenError {
    markup_domain(&descriptors(&[file]), &[file], BAD_WIRE).expect_err("refused")
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

/// A message named as one of the names the domain layer gives the
/// runtime's types (`Context`, `PolicyContext`, `CommandContext`,
/// `Rejection`, `Bug`, `Failed`, `BusinessStatus`, `RetryableStatus`) is
/// refused, naming
/// both, never renamed.
#[test]
fn test_markup_domain_refuses_a_message_named_as_a_reserved_name() {
    const FILE: &str = "bad/v1/reserved_name.proto";
    assert_eq!(
        markup_domain(
            &descriptors(&[FILE]),
            &[FILE],
            &[("bad.v1", "::bad_proto::v1")]
        ),
        Err(angzarr_codegen::CodegenError::ReservedName {
            message: "bad.v1.Context".to_string(),
            name: "Context".to_string(),
        })
    );
}

/// `RetryableStatus` is reserved as well: the status a handler's
/// retryable bug goes out under, re-exported by the domain layer.
#[test]
fn test_markup_domain_refuses_a_message_named_retryable_status() {
    const FILE: &str = "bad/v1/reserved_retryable_status.proto";
    assert_eq!(
        markup_domain(
            &descriptors(&[FILE]),
            &[FILE],
            &[("bad.v1", "::bad_proto::v1")]
        ),
        Err(angzarr_codegen::CodegenError::ReservedName {
            message: "bad.v1.RetryableStatus".to_string(),
            name: "RetryableStatus".to_string(),
        })
    );
}

/// `Bug` is reserved as well: the name a defect's class takes in the
/// domain layers.
#[test]
fn test_markup_domain_refuses_a_message_named_bug() {
    const FILE: &str = "bad/v1/reserved_bug.proto";
    assert_eq!(
        markup_domain(
            &descriptors(&[FILE]),
            &[FILE],
            &[("bad.v1", "::bad_proto::v1")]
        ),
        Err(angzarr_codegen::CodegenError::ReservedName {
            message: "bad.v1.Bug".to_string(),
            name: "Bug".to_string(),
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

/// The domain layer gives the user's files the runtime's contexts and what
/// a handler fails with under its own names, so a user file imports only
/// `crate::domain`: a handler's `Context`, a policy reaction's
/// `PolicyContext`, a refused command's `CommandContext`, `Failed`,
/// `BusinessStatus` and `RetryableStatus`, and `Bug`, a diagnostic key and its args, which a
/// command handler's `Failed` of its rejection or of a typed refusal takes
/// by `?`.
#[test]
fn test_markup_domain_gives_the_users_files_the_contexts_and_the_bug() {
    let file = buyin();
    let reexports: Vec<String> = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Use(u) => Some(
                quote::ToTokens::to_token_stream(&u.tree)
                    .to_string()
                    .replace(' ', ""),
            ),
            _ => None,
        })
        .collect();
    for name in [
        "Context",
        "PolicyContext",
        "CommandContext",
        "{BusinessStatus,Failed,RetryableStatus}",
    ] {
        assert!(
            reexports.contains(&format!("angzarr_embedded::next::markup::{name}")),
            "{name}: {reexports:?}"
        );
    }
    assert!(
        items(&file).contains(&"Bug".to_string()),
        "{:?}",
        items(&file)
    );
    let impls = impls(&file);
    assert!(impls.contains(&(
        "::angzarr_embedded_diagnostic::Diagnostic".to_string(),
        "Bug".to_string()
    )));
    for failed in ["Failed<Rejection>", "Failed<InsufficientFunds>"] {
        assert!(
            impls.contains(&("::core::convert::From<Bug>".to_string(), failed.to_string())),
            "{failed}: {impls:?}"
        );
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

#[test]
fn test_markup_domain_message_fields_are_required_and_optional_scalars_stay_optional() {
    let file = shop();
    assert_eq!(
        fields(&file, "CheckedOut"),
        s(&[
            ("owner", "String"),
            ("lines", "Vec<Line>"),
            ("totals", "::std::collections::BTreeMap<String,i64>"),
            ("discount", "Option<i32>"),
            ("opened_at", "::angzarr_embedded::next::Timestamp"),
            ("by_sku", "::std::collections::BTreeMap<String,Line>"),
        ])
    );
}

#[test]
fn test_markup_domain_enums_are_closed_and_omit_the_zero_value() {
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
fn test_markup_domain_converts_each_type_to_and_from_its_wire_message() {
    let impls = impls(&shop());
    for name in ["Line", "AddLine", "Checkout", "LineAdded", "CheckedOut"] {
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
fn test_markup_domain_maps_a_map_field_without_a_type_for_its_entry() {
    let names: Vec<String> = shop()
        .items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Struct(s) => Some(s.ident.to_string()),
            _ => None,
        })
        .collect();
    assert!(names.contains(&"Line".to_string()), "{names:?}");
    assert!(!names.iter().any(|n| n.ends_with("Entry")), "{names:?}");
}

#[test]
fn test_markup_domain_without_a_component_generates_nothing() {
    let generated = markup_domain(
        &descriptors(&["io/angzarr/v1/options.proto"]),
        &["io/angzarr/v1/options.proto"],
        &[],
    )
    .unwrap();
    assert_eq!(generated, Vec::new());
}

#[test]
fn test_markup_domain_refuses_a_command_naming_an_unknown_component() {
    assert_eq!(
        refusal("bad/v1/unknown_component.proto"),
        CodegenError::UnknownComponent {
            message: "bad.v1.Poke".to_string(),
            component: "bad.v1.Nowhere".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_command_naming_a_message_that_is_no_component() {
    assert_eq!(
        refusal("bad/v1/not_a_component.proto"),
        CodegenError::UnknownComponent {
            message: "bad.v1.Poke".to_string(),
            component: "bad.v1.Plain".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_command_emitting_a_non_event() {
    assert_eq!(
        refusal("bad/v1/unknown_emit.proto"),
        CodegenError::UnknownEmit {
            command: "bad.v1.Poke".to_string(),
            event: "bad.v1.Poke".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_an_enum_whose_zero_is_a_value() {
    assert_eq!(
        refusal("bad/v1/enum_zero.proto"),
        CodegenError::EnumZero {
            enum_name: "bad.v1.Mood".to_string(),
            value: "MOOD_CALM".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_oneof() {
    assert_eq!(
        refusal("bad/v1/oneof.proto"),
        CodegenError::Oneof {
            message: "bad.v1.Poke".to_string(),
            oneof: "shape".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_nested_type() {
    assert_eq!(
        refusal("bad/v1/nested.proto"),
        CodegenError::NestedType {
            message: "bad.v1.Poke".to_string(),
            nested: "bad.v1.Poke.Inner".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_commutative_state() {
    assert_eq!(
        refusal("bad/v1/commutative.proto"),
        CodegenError::Commutative {
            component: "bad.v1.ThingState".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_component_of_no_kind() {
    assert_eq!(
        refusal("bad/v1/unspecified_kind.proto"),
        CodegenError::UnsupportedKind {
            component: "bad.v1.ThingState".to_string(),
            kind: 0,
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_well_known_type_other_than_timestamp() {
    assert_eq!(
        refusal("bad/v1/well_known.proto"),
        CodegenError::WellKnownType {
            message: "bad.v1.Poke".to_string(),
            field: "timeout".to_string(),
            type_name: "google.protobuf.Duration".to_string(),
        }
    );
}

#[test]
fn test_markup_domain_refuses_a_package_without_a_wire_path() {
    assert_eq!(
        markup_domain(&descriptors(&[SHOP]), &[SHOP], &[]),
        Err(CodegenError::NoWirePath {
            package: "shop.v1".to_string(),
        })
    );
}

#[test]
fn test_markup_domain_refuses_a_file_not_in_the_set() {
    assert_eq!(
        markup_domain(&descriptors(&[SHOP]), &["shop/v1/absent.proto"], SHOP_WIRE),
        Err(CodegenError::FileNotInSet {
            file: "shop/v1/absent.proto".to_string(),
        })
    );
}

#[test]
fn test_markup_domain_refuses_bytes_that_are_not_a_descriptor_set() {
    assert!(matches!(
        markup_domain(&[0xff, 0xff], &[SHOP], SHOP_WIRE),
        Err(CodegenError::Descriptors { .. })
    ));
}

#[test]
fn test_every_error_has_a_catalog_message_with_its_args() {
    let ftl = include_str!("../locales/en-US/angzarr-codegen.ftl");
    let problems = angzarr_fmt::check(ftl, &angzarr_codegen::diagnostic_samples());
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn test_every_catalog_message_is_the_code_of_a_sample() {
    let ftl = include_str!("../locales/en-US/angzarr-codegen.ftl");
    let mut messages: Vec<&str> = ftl
        .lines()
        .filter_map(|line| line.split_once(" = ").map(|(id, _)| id))
        .filter(|id| id.starts_with("angzarr-codegen-"))
        .collect();
    messages.sort_unstable();
    let mut codes: Vec<&str> = angzarr_codegen::diagnostic_samples()
        .iter()
        .map(|d| d.code())
        .collect();
    codes.sort_unstable();
    assert_eq!(codes, messages);
}

#[test]
fn test_every_errors_english_is_its_catalog_message() {
    let mut catalog = angzarr_fmt::Catalog::en_us();
    catalog.add(angzarr_codegen::EN_US).unwrap();
    for sample in angzarr_codegen::diagnostic_samples() {
        let rendered: String = catalog
            .render(&*sample)
            .chars()
            .filter(|c| !matches!(c, '\u{2068}' | '\u{2069}'))
            .collect();
        assert_eq!(sample.to_string(), rendered, "{}", sample.code());
    }
}
