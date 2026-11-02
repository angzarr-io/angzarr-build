//! What a build script gets: angzarr's prost types for a wire crate, the
//! domain layer, and the descriptors, each written into the output
//! directory, and the failures it reports.

use std::path::Path;

use angzarr_build::{BuildError, markup_domain_into, wire_into};

#[path = "../../../test-support/test_dir.rs"]
mod test_dir;
use test_dir::TestDir;

const SHOP: &str = "shop/v1/shop.proto";

fn fixtures() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../angzarr-codegen/tests/fixtures")
        .display()
        .to_string()
}

/// A fresh, empty output directory.
fn out_dir() -> TestDir {
    TestDir::new(concat!(env!("CARGO_PKG_NAME"), "-test"))
}

#[test]
fn test_wire_into_writes_prost_types_by_package() {
    let out = out_dir();
    wire_into(&out, &[SHOP], &[&fixtures()]).unwrap();
    let source = std::fs::read_to_string(out.join("shop.v1.rs")).expect("shop.v1.rs");
    let file = syn::parse_file(&source).unwrap();
    assert!(
        file.items
            .iter()
            .any(|i| matches!(i, syn::Item::Struct(s) if s.ident == "CartState")),
        "prost's CartState"
    );
}

#[test]
fn test_markup_domain_into_reports_the_generators_refusal() {
    let out = out_dir();
    let error = markup_domain_into(&out, &[SHOP], &[&fixtures()], &[]).expect_err("no wire path");
    assert!(matches!(error, BuildError::Codegen(_)), "{error:?}");
}

#[test]
fn test_markup_domain_into_reports_a_proto_protoc_cannot_find() {
    let out = out_dir();
    let error = markup_domain_into(&out, &["shop/v1/absent.proto"], &[&fixtures()], &[])
        .expect_err("no such proto");
    assert!(matches!(error, BuildError::Protoc { .. }), "{error:?}");
}

#[test]
fn test_every_error_has_a_catalog_message_with_its_args() {
    let ftl = angzarr_build::EN_US.1;
    let samples = angzarr_build::diagnostic_samples();
    let problems = angzarr_fmt::check(ftl, &samples);
    assert!(problems.is_empty(), "{problems:#?}");
    // A sample of every message the catalog has.
    let mut codes: Vec<&str> = samples.iter().map(|d| d.code()).collect();
    codes.sort_unstable();
    let mut messages: Vec<&str> = ftl
        .lines()
        .filter_map(|l| l.split_once(" = ").map(|(id, _)| id))
        .collect();
    messages.sort_unstable();
    assert_eq!(codes, messages);
}

/// A build script's `main` returning the error prints its `Debug`: that is
/// where a person reads it, so it is the English, the generator's refusal
/// as its own.
#[test]
fn test_a_failing_build_script_prints_the_errors_english() {
    let s = || "x y".to_string();
    let cases = [
        (
            BuildError::NoOutDir,
            "OUT_DIR is unset: run this from a build script",
        ),
        (
            BuildError::Io {
                path: "a/b".to_string(),
                detail: s(),
            },
            "a/b: x y",
        ),
        (BuildError::Protoc { detail: s() }, "protoc failed: x y"),
        (BuildError::Prost { detail: s() }, "prost-build failed: x y"),
        (BuildError::Buf { detail: s() }, "buf generate failed: x y"),
        (BuildError::Uv { detail: s() }, "uv failed: x y"),
        (
            BuildError::Codegen(angzarr_codegen::CodegenError::Use(
                angzarr_codegen::UseError::NoWirePath {
                    package: "shop.v1".to_string(),
                },
            )),
            "no Rust path is given for the prost types of package shop.v1",
        ),
    ];
    for (error, english) in cases {
        assert_eq!(format!("{error:?}"), english);
    }
}

/// Every error displays as its code and args, never English (decision
/// 14.37), and core's catalog check agrees with the catalog.
#[test]
fn test_every_error_displays_as_its_code_and_args() {
    let (krate, ftl) = angzarr_build::EN_US;
    let problems =
        angzarr_diagnostic::catalog::check(krate, ftl, &angzarr_build::diagnostic_samples());
    assert_eq!(problems, Vec::new());
}

#[test]
fn test_component_names_are_the_protos_with_their_application_names() {
    let names: Vec<(String, String)> = angzarr_build::component_names(
        &[
            "buyin/v1/wallet.proto",
            "buyin/v1/buy_in.proto",
            "buyin/v1/policies.proto",
        ],
        &[&fixtures()],
    )
    .unwrap()
    .into_iter()
    .map(|c| (c.component, c.name))
    .collect();
    let s = |a: &str, b: &str| (a.to_string(), b.to_string());
    assert_eq!(
        names,
        [
            s("buyin.v1.WalletState", "wallet"),
            s("buyin.v1.BuyInState", "pmg-buyin"),
            s("buyin.v1.BuyInWalletPolicy", "policy-buyin-wallet"),
            s("buyin.v1.WalletBuyInPolicy", "policy-wallet-buyin"),
        ]
    );
}

#[test]
fn test_descriptors_into_writes_each_components_descriptor_with_the_version() {
    let out = out_dir();
    angzarr_build::descriptors_into(
        &out,
        &["orders/v1/orders.proto"],
        &[&fixtures()],
        &[],
        "2.0.0",
    )
    .unwrap();
    let source = std::fs::read_to_string(out.join("angzarr-descriptors.rs"))
        .expect("angzarr-descriptors.rs");
    let file = syn::parse_file(&source).unwrap();
    assert!(
        file.items
            .iter()
            .any(|i| matches!(i, syn::Item::Const(c) if c.ident == "DESCRIPTORS")),
        "const DESCRIPTORS"
    );
    // The version is in the encoded descriptors.
    assert!(source.contains("2.0.0"), "{source}");
}

/// A child-served domain's stand-in, generated from its proto into the
/// output directory, for a crate that hand-writes the policies naming it.
#[test]
fn test_stand_ins_into_writes_the_remote_domains_stand_in() {
    let out = out_dir();
    angzarr_build::stand_ins_into(&out, &["buyin/v1/wallet.proto"], &[&fixtures()]).unwrap();
    let source = std::fs::read_to_string(out.join("angzarr-stand-ins.rs")).unwrap();
    assert!(source.contains("impl CommandTarget for Wallet"), "{source}");
    assert!(source.contains("impl Decides<Hold> for Wallet"), "{source}");
}
