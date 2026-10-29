//! angzarr's generator core. Pure: a descriptor set in, generated source out;
//! running protoc and writing files are the callers' (`angzarr-build`).
//!
//! The descriptor set must be protoc's own bytes (`--include_imports
//! --descriptor_set_out`): angzarr's declarations are custom options, which
//! survive only there, read through `prost-reflect`. prost drops them when it
//! decodes a set.

mod error;
mod model;
mod python;
mod rust;
mod scaffold;

pub use error::{CodegenError, diagnostic_samples};

/// This crate's en-US messages.
pub const EN_US: &str = include_str!("../locales/en-US/angzarr-codegen.ftl");
pub use python::{
    CLIENT_PYTHON_GIT, CLIENT_PYTHON_REV, ProtoInput, PythonApp, python_reroots, reroot_python,
    scaffold_python,
};
pub use scaffold::{Owner, ScaffoldFile, Translate, complete, markup_domain, register, scaffold};

/// The Rust source generated for one proto package.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Generated {
    /// The proto package, e.g. `shop.v1`.
    pub package: String,
    pub source: String,
}

/// The domain layer for every component declared in `files`: one
/// [`Generated`] per package holding one.
///
/// `wire` maps each proto package to the Rust path of its prost types, as
/// prost-build's `extern_path` does (`("shop.v1", "::shop_proto::v1")`).
pub fn domain(
    descriptor_set: &[u8],
    files: &[&str],
    wire: &[(&str, &str)],
) -> Result<Vec<Generated>, CodegenError> {
    generate(model::build(descriptor_set, files)?, wire, rust::package)
}

/// One [`Generated`] per package, each emitted with its prost types' Rust
/// path from `wire`.
fn generate(
    packages: std::collections::BTreeMap<String, model::Package>,
    wire: &[(&str, &str)],
    emit: fn(&model::Package, &syn::Path) -> String,
) -> Result<Vec<Generated>, CodegenError> {
    let mut generated = Vec::with_capacity(packages.len());
    for (package, content) in packages {
        let path = wire
            .iter()
            .find(|(p, _)| *p == package)
            .and_then(|(_, rust)| syn::parse_str::<syn::Path>(rust).ok())
            .ok_or_else(|| CodegenError::NoWirePath {
                package: package.clone(),
            })?;
        generated.push(Generated {
            source: emit(&content, &path),
            package,
        });
    }
    Ok(generated)
}
