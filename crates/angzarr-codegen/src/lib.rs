//! angzarr's generator core. Pure: a descriptor set in, generated source out;
//! running protoc and writing files are the callers' (`angzarr-build`).
//!
//! The descriptor set must be protoc's own bytes (`--include_imports
//! --descriptor_set_out`): angzarr's declarations are custom options, which
//! survive only there, read through `prost-reflect`. prost drops them when it
//! decodes a set.

mod app;
mod descriptor;
mod error;
mod model;
mod names;
mod python;
mod rust;
mod scaffold;

pub use app::{
    AppLint, AppSeed, Exposure, RetrySeed, app, app_lint_samples, lint_app, remote_domains,
};
pub use descriptor::{Descriptor, descriptors, descriptors_rs};
pub use error::{CodegenError, diagnostic_samples};
pub use names::{ComponentName, NameKind, NameOf, component_name, component_names, kebab};
pub use rust::refusal_key;

/// This crate's en-US messages.
pub const EN_US: &str = include_str!("../locales/en-US/angzarr-codegen.ftl");
pub use python::{
    CLIENT_PYTHON_GIT, CLIENT_PYTHON_REV, ProtoInput, PythonApp, python_reroots, reroot_python,
    scaffold_python,
};
pub use scaffold::{
    Owner, ScaffoldFile, Translate, adapters, complete, markup_domain, register, scaffold,
};

/// The Rust source generated for one proto package.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Generated {
    /// The proto package, e.g. `shop.v1`.
    pub package: String,
    pub source: String,
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
