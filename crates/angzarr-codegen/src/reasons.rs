//! The framework's reason keys as constants, from angzarr-project's
//! `catalog/reasons.toml` (decisions 14.37 and 14.42).

use crate::error::CodegenError;

/// The catalog's keys as Rust constants.
pub fn reasons_rust(_catalog: &str) -> Result<String, CodegenError> {
    Ok(String::new())
}

/// The catalog's keys as Python constants.
pub fn reasons_python(_catalog: &str) -> Result<String, CodegenError> {
    Ok(String::new())
}
