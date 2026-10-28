//! What a project's protos give angzarr-local (annals) besides the domain
//! layer: the scaffolded markup impls (annals-macros), written once into the
//! user's source tree, and the generated `register`, which registers every
//! component of the project with annals' `Registry`, so an application
//! calls one function.
//!
//! Pure, as the rest of the crate: descriptors (and, for [`complete`], the
//! user's current file) in, source out. Reading and writing files is
//! angzarr-build's.

use crate::Generated;
use crate::error::CodegenError;

/// One component's scaffold: the user's file, written once and never
/// overwritten.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScaffoldFile {
    /// The component's anchor message, e.g. `buyin.v1.WalletState`.
    pub component: String,
    /// Where it goes, relative to the crate's `src`: `wallet.rs`.
    pub path: String,
    pub source: String,
}

/// angzarr-cli's refusal switch (backlog OB-7), set to translate for one
/// command of one policy: the policy's scaffold gets an
/// `#[on_refused(command)]` callback stub returning `fact`, a fact of its
/// source. Every command without one is passed through, the default.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Translate {
    /// The policy's anchor message, e.g. `buyin.v1.BuyInWalletPolicy`.
    pub policy: String,
    /// A command of the policy's target, e.g. `buyin.v1.Hold`.
    pub command: String,
    /// A command or fact of the policy's source, e.g. `buyin.v1.HoldLost`.
    pub fact: String,
}

/// The scaffold of every component declared in `files`, one file each:
/// the struct and its decorated impl (`#[aggregate]`, `#[process_manager]`
/// or `#[policy]`), one stub per input and event, `todo!()` bodies.
pub fn scaffold(
    descriptor_set: &[u8],
    files: &[&str],
    translate: &[Translate],
) -> Result<Vec<ScaffoldFile>, CodegenError> {
    let _ = (descriptor_set, files, translate);
    Ok(Vec::new())
}

/// `existing`, the user's file for `fresh`'s component, with the stubs of
/// `fresh` that it lacks inserted into its decorated impl, and nothing else
/// changed; `None` when it lacks none. A stub is present when the impl has
/// any marker naming its type.
pub fn complete(fresh: &ScaffoldFile, existing: &str) -> Result<Option<String>, CodegenError> {
    let _ = (fresh, existing);
    Ok(None)
}

/// The generated `register(&mut Registry)`: every component declared in
/// `files`, registered with annals, each at the path its scaffold gives it.
pub fn register(descriptor_set: &[u8], files: &[&str]) -> Result<String, CodegenError> {
    let _ = (descriptor_set, files);
    Ok(String::new())
}

/// The domain layer for marked-up components: the domain types, their wire
/// conversions and each one's `DomainMessage` impl, and nothing the markup
/// generates itself (no payload enums, no `State` impl).
pub fn markup_domain(
    descriptor_set: &[u8],
    files: &[&str],
    wire: &[(&str, &str)],
) -> Result<Vec<Generated>, CodegenError> {
    let _ = (descriptor_set, files, wire);
    Ok(Vec::new())
}
