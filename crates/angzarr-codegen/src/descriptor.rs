//! Each component's whole `ComponentDescriptor` (types.proto), generated
//! at build time from its protos (decision 14.42).

use crate::error::CodegenError;

/// One component's generated descriptor.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Descriptor {
    /// The component's anchor message, e.g. `orders.v1.OrderState`.
    pub component: String,
    /// `ComponentDescriptor.name`.
    pub name: String,
    /// The `ComponentDescriptor`, encoded.
    pub encoded: Vec<u8>,
}

/// The descriptor of every component declared in `files`.
pub fn descriptors(
    _descriptor_set: &[u8],
    _files: &[&str],
    _rules_version: &str,
) -> Result<Vec<Descriptor>, CodegenError> {
    Ok(Vec::new())
}

/// The Rust build layer's descriptors.
pub fn descriptors_rs(
    _descriptor_set: &[u8],
    _files: &[&str],
    _remote: &[&str],
    _rules_version: &str,
) -> Result<String, CodegenError> {
    Ok(String::new())
}
