//! What a build script gets: angzarr's prost types for a wire crate, and the
//! domain layer for a domain crate, each written into the output directory
//! by package.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use angzarr_build::{BuildError, domain_into, wire_into};

const SHOP: &str = "shop/v1/shop.proto";

fn fixtures() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../angzarr-codegen/tests/fixtures")
        .display()
        .to_string()
}

/// A fresh, empty output directory.
fn out_dir() -> PathBuf {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "angzarr-build-test-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
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
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_domain_into_writes_the_domain_layer_by_package() {
    let out = out_dir();
    domain_into(
        &out,
        &[SHOP],
        &[&fixtures()],
        &[("shop.v1", "::shop_proto::v1")],
    )
    .unwrap();
    let source = std::fs::read_to_string(out.join("shop.v1.domain.rs")).expect("domain file");
    let file = syn::parse_file(&source).unwrap();
    assert!(
        file.items
            .iter()
            .any(|i| matches!(i, syn::Item::Enum(e) if e.ident == "CartCommand")),
        "the payload enum"
    );
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_domain_into_reports_the_generators_refusal() {
    let out = out_dir();
    let error = domain_into(&out, &[SHOP], &[&fixtures()], &[]).expect_err("no wire path");
    assert!(matches!(error, BuildError::Codegen(_)), "{error:?}");
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_domain_into_reports_a_proto_protoc_cannot_find() {
    let out = out_dir();
    let error = domain_into(&out, &["shop/v1/absent.proto"], &[&fixtures()], &[])
        .expect_err("no such proto");
    assert!(matches!(error, BuildError::Protoc { .. }), "{error:?}");
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_every_error_has_a_catalog_message_with_its_args() {
    let ftl = include_str!("../locales/en-US/angzarr-build.ftl");
    let problems = angzarr_fmt::check(ftl, &angzarr_build::diagnostic_samples());
    assert!(problems.is_empty(), "{problems:#?}");
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
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn test_reasons_into_writes_the_catalogs_constants() {
    let out = out_dir();
    let catalog = Path::new(&fixtures()).join("catalog/reasons.toml");
    let rust = out.join("reasons.rs");
    let python = out.join("reasons.py");
    assert_eq!(
        angzarr_build::reasons_rust_into(&catalog, &rust).unwrap(),
        33
    );
    assert_eq!(
        angzarr_build::reasons_python_into(&catalog, &python).unwrap(),
        33
    );
    let rust = std::fs::read_to_string(rust).unwrap();
    assert!(rust.contains("pub const POLICY_INPUT_COUNT: &str = \"POLICY_INPUT_COUNT\";"));
    let python = std::fs::read_to_string(python).unwrap();
    assert!(python.contains("\nPOLICY_INPUT_COUNT = \"POLICY_INPUT_COUNT\"\n"));
    assert!(matches!(
        angzarr_build::reasons_rust_into(&out.join("absent.toml"), &out.join("x.rs")),
        Err(BuildError::Io { .. })
    ));
    std::fs::remove_dir_all(&out).unwrap();
}

/// `DESCRIPTORS`' names in an `angzarr-descriptors.rs`.
fn descriptor_names(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).unwrap();
    let Some(syn::Item::Const(c)) = file.items.first() else {
        panic!("const DESCRIPTORS");
    };
    let syn::Expr::Reference(slice) = &*c.expr else {
        panic!("a slice");
    };
    let syn::Expr::Array(array) = &*slice.expr else {
        panic!("an array");
    };
    array
        .elems
        .iter()
        .map(|e| match e {
            syn::Expr::Tuple(t) => match &t.elems[0] {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => s.value(),
                _ => panic!("a name"),
            },
            _ => panic!("a pair"),
        })
        .collect()
}

#[test]
fn test_app_into_writes_the_whole_layer_with_the_crates_version() {
    let out = out_dir();
    let config = out.join("angzarr.toml");
    std::fs::write(
        &config,
        "[[component]]\nname = \"wallet\"\nlanguage = \"rust\"\n\n\
         [[component]]\nname = \"buyin\"\nlanguage = \"python\"\n",
    )
    .unwrap();
    angzarr_build::app_into(
        &out,
        &config,
        &[
            "buyin/v1/wallet.proto",
            "buyin/v1/buy_in.proto",
            "buyin/v1/policies.proto",
        ],
        &[&fixtures()],
    )
    .unwrap();
    for file in [
        "buyin.v1.rs",
        "buyin.v1.domain.rs",
        "angzarr-adapters.rs",
        "angzarr-register.rs",
    ] {
        assert!(out.join(file).exists(), "{file}");
    }
    let source = std::fs::read_to_string(out.join("angzarr-descriptors.rs")).unwrap();
    // The buy-in is the Python child's: its descriptor is the child's.
    assert_eq!(
        descriptor_names(&source),
        ["Wallet", "BuyInWallet", "WalletBuyIn"]
    );
    assert!(
        source.contains(&format!("{:?}", env!("CARGO_PKG_VERSION"))),
        "the crate's version: {source}"
    );
    std::fs::remove_dir_all(&out).unwrap();
}
