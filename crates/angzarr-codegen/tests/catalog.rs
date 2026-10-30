//! The reason-key constants (spec decisions 14.37 and 14.42): one per key
//! of angzarr-project's `catalog/reasons.toml`, for Rust and for Python,
//! and none for a retired key. `tests/fixtures/catalog/reasons.toml` is the
//! catalog as it is at the spec commit angzarr-proto is pinned to.

use angzarr_codegen::{CodegenError, reasons_python, reasons_rust};

const CATALOG: &str = include_str!("fixtures/catalog/reasons.toml");

/// The catalog's keys, and its retired ones.
fn catalog() -> (Vec<String>, Vec<String>) {
    let table: toml::Table = CATALOG.parse().expect("the catalog parses");
    let keys = table["keys"]
        .as_table()
        .expect("[keys]")
        .keys()
        .cloned()
        .collect();
    let retired = table["retired"]
        .as_table()
        .expect("[retired]")
        .values()
        .flat_map(|v| v.as_array().expect("a list of keys").clone())
        .map(|k| k.as_str().expect("a key").to_string())
        .collect();
    (keys, retired)
}

/// Each `pub const NAME: &str = "VALUE";` of `source`, as `(NAME, VALUE)`.
fn rust_constants(source: &str) -> Vec<(String, String)> {
    let file = syn::parse_file(source).expect("the generated Rust parses");
    file.items
        .iter()
        .map(|item| {
            let syn::Item::Const(c) = item else {
                panic!("only constants");
            };
            assert!(matches!(c.vis, syn::Visibility::Public(_)), "pub");
            assert_eq!(c.ty.to_token_stream().to_string(), "& str");
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) = &*c.expr
            else {
                panic!("a string literal");
            };
            (c.ident.to_string(), value.value())
        })
        .collect()
}

use quote::ToTokens;

/// Each `NAME = "VALUE"` line of `source`.
fn python_constants(source: &str) -> Vec<(String, String)> {
    source
        .lines()
        .filter_map(|line| line.split_once(" = "))
        .map(|(name, value)| {
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or_else(|| panic!("a string: {value}"));
            (name.to_string(), value.to_string())
        })
        .collect()
}

fn each_key_itself(keys: &[String]) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = keys.iter().map(|k| (k.clone(), k.clone())).collect();
    pairs.sort();
    pairs
}

#[test]
fn every_catalog_key_is_a_generated_constant() {
    let (keys, _) = catalog();
    assert!(keys.contains(&"POLICY_INPUT_COUNT".to_string()));
    let mut rust = rust_constants(&reasons_rust(CATALOG).unwrap());
    rust.sort();
    assert_eq!(rust, each_key_itself(&keys), "Rust");
    let mut python = python_constants(&reasons_python(CATALOG).unwrap());
    python.sort();
    assert_eq!(python, each_key_itself(&keys), "Python");
}

#[test]
fn a_retired_key_is_never_generated() {
    let (_, retired) = catalog();
    assert!(retired.contains(&"MISSING_COMMAND_BOOK".to_string()));
    let rust = reasons_rust(CATALOG).unwrap();
    let python = reasons_python(CATALOG).unwrap();
    for key in retired {
        let quoted = format!("\"{key}\"");
        assert!(!rust.contains(&quoted), "Rust names retired {key}");
        assert!(!python.contains(&quoted), "Python names retired {key}");
    }
}

#[test]
fn a_constant_says_who_answers_it_with_what_status_and_args() {
    let rust = reasons_rust(CATALOG).unwrap();
    assert!(
        rust.contains(
            "/// Answered by the coordinator, with status INTERNAL; args: `expected_domain`, `actual_domain`.\npub const POLICY_COMMAND_OUTSIDE_DESTINATION"
        ),
        "{rust}"
    );
    assert!(
        rust.contains("/// Answered by the router, with status INTERNAL; no args.\npub const CONFLICT_RESOLUTION_UNSUPPORTED"),
        "{rust}"
    );
    let python = reasons_python(CATALOG).unwrap();
    assert!(
        python.contains(
            "POLICY_INPUT_COUNT = \"POLICY_INPUT_COUNT\"\n\"\"\"Answered by the coordinator, as a readiness refusal; args: ``component``, ``input_count``.\"\"\""
        ),
        "{python}"
    );
}

#[test]
fn a_catalog_that_is_not_toml_is_refused() {
    assert!(matches!(
        reasons_rust("[keys"),
        Err(CodegenError::Catalog { .. })
    ));
    assert!(matches!(
        reasons_python("[keys.X]\nowner = 1"),
        Err(CodegenError::Catalog { .. })
    ));
}
