//! What the scaffold and register tests share: protoc's descriptor set for
//! the fixtures, and reading generated source back as Rust items.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use quote::ToTokens;

/// The toy buy-in project: a wallet aggregate, a buy-in process manager,
/// and a policy each way between them.
pub const BUYIN: &[&str] = &[
    "buyin/v1/wallet.proto",
    "buyin/v1/buy_in.proto",
    "buyin/v1/policies.proto",
];
pub const BUYIN_WIRE: &[(&str, &str)] = &[("buyin.v1", "::buyin_proto::v1")];

/// protoc's descriptor set for `files` under `tests/fixtures`, with
/// angzarr's protos importable.
pub fn descriptors(files: &[&str]) -> Vec<u8> {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "angzarr-codegen-scaffold-test-{}-{}",
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

/// Tokens as text without spacing.
pub fn text(tokens: impl ToTokens) -> String {
    tokens.to_token_stream().to_string().replace(' ', "")
}

/// The inherent impl of `ty` in `file`.
pub fn inherent_impl<'a>(file: &'a syn::File, ty: &str) -> &'a syn::ItemImpl {
    file.items
        .iter()
        .find_map(|i| match i {
            syn::Item::Impl(imp) if imp.trait_.is_none() && text(&imp.self_ty) == ty => Some(imp),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no impl {ty}"))
}

/// An attribute as `(name, args)`, args without spacing: `("handles_cmd",
/// "Hold,refuses(InsufficientFunds)")`.
pub fn attribute(attr: &syn::Attribute) -> (String, String) {
    let name = text(attr.path());
    let args = match &attr.meta {
        syn::Meta::List(list) => text(&list.tokens),
        _ => String::new(),
    };
    (name, args)
}

/// The impl's own markup attribute (`#[aggregate(..)]` and the like).
pub fn impl_markup(imp: &syn::ItemImpl) -> (String, String) {
    imp.attrs
        .iter()
        .filter(|a| !a.path().is_ident("doc"))
        .map(attribute)
        .next()
        .expect("a markup attribute on the impl")
}

/// The impl's methods.
pub fn methods(imp: &syn::ItemImpl) -> Vec<&syn::ImplItemFn> {
    imp.items
        .iter()
        .filter_map(|i| match i {
            syn::ImplItem::Fn(f) => Some(f),
            _ => None,
        })
        .collect()
}

/// Every marker on the impl's methods, in order, as `(name, args)`.
pub fn markers(imp: &syn::ItemImpl) -> Vec<(String, String)> {
    methods(imp)
        .iter()
        .flat_map(|f| f.attrs.iter())
        .filter(|a| !a.path().is_ident("doc"))
        .map(attribute)
        .collect()
}

/// Each method's signature, without spacing.
pub fn signatures(imp: &syn::ItemImpl) -> Vec<String> {
    methods(imp).iter().map(|f| text(&f.sig)).collect()
}

pub fn s(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

/// The blackjack example of spec decision 14.38 (`tests/fixtures/io`, the
/// spec's protos as they are).
pub const BLACKJACK: &[&str] = &[
    "io/angzarr/examples/v1/buy_in.proto",
    "io/angzarr/examples/v1/cards.proto",
    "io/angzarr/examples/v1/ledger.proto",
    "io/angzarr/examples/v1/player.proto",
    "io/angzarr/examples/v1/policies.proto",
    "io/angzarr/examples/v1/table.proto",
    "io/angzarr/examples/v1/wallet.proto",
];
pub const PLAYER: &str = "io/angzarr/examples/v1/player.proto";
