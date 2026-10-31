//! A component's name in an application (decision 14.50): what the host
//! config's `[[component]]` entry, the generated registration and
//! hand-written code call it, all through [`component_name`].
//! `ComponentOptions.component_name` when declared, else a default by kind:
//!
//! | kind            | default                                  | e.g.                          |
//! |-----------------|------------------------------------------|-------------------------------|
//! | aggregate       | its domain                               | `table`                       |
//! | process manager | `pmg-` + its domain                      | `pmg-buy-in`                  |
//! | policy          | `policy-` + input + `-` + output domain  | `policy-table-player`         |
//! | projector       | `projector-` + [`kebab`] of its anchor   | `projector-ledger-projection` |
//!
//! A name is lower-case ASCII letters, digits and `-`, starting with a
//! letter; any other fails generation ([`CodegenError::ComponentName`]).
//! `ComponentOptions.name` is not this: it is the code's base name.

use crate::error::CodegenError;
use crate::model;

/// A component's kind, as its name's default reads it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NameKind {
    Aggregate,
    ProcessManager,
    Policy,
    Projector,
}

/// What a component's name is computed from: its `ComponentOptions`, and
/// its anchor message's name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NameOf<'a> {
    pub kind: NameKind,
    /// `component_name`, empty when not declared.
    pub component_name: &'a str,
    pub domain: &'a str,
    pub input_domain: &'a str,
    pub output_domain: &'a str,
    /// The anchor message's name (`LedgerProjection`), not its full name.
    pub anchor: &'a str,
}

/// The component's name in an application: its declared `component_name`,
/// else its kind's default (see the module's table), refused unless it is
/// lower-case ASCII letters, digits and `-`, starting with a letter.
pub fn component_name(of: &NameOf<'_>) -> Result<String, CodegenError> {
    let name = unchecked(of);
    if !well_formed(&name) {
        return Err(CodegenError::ComponentName {
            component: of.anchor.to_string(),
            name,
        });
    }
    Ok(name)
}

/// [`component_name`] before its check.
pub(crate) fn unchecked(of: &NameOf<'_>) -> String {
    if !of.component_name.is_empty() {
        return of.component_name.to_string();
    }
    match of.kind {
        NameKind::Aggregate => of.domain.to_string(),
        NameKind::ProcessManager => format!("pmg-{}", of.domain),
        NameKind::Policy => format!("policy-{}-{}", of.input_domain, of.output_domain),
        NameKind::Projector => format!("projector-{}", kebab(of.anchor)),
    }
}

/// Lower-case ASCII letters, digits and `-`, starting with a letter.
pub(crate) fn well_formed(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `HTTPServerView` as `http-server-view`: a `-` before each capital that
/// follows a lower-case letter or a digit, or that is followed by a
/// lower-case letter and follows a capital, then all lower-case.
pub fn kebab(name: &str) -> String {
    crate::rust::snake(name).replace('_', "-")
}

/// One component of a descriptor set and its name in an application.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComponentName {
    /// The anchor message's full name, `io.angzarr.examples.v1.TableState`.
    pub component: String,
    pub kind: NameKind,
    /// Its name in an application ([`component_name`]).
    pub name: String,
    /// The stream domain it owns: an aggregate's or a process manager's,
    /// else empty.
    pub domain: String,
}

/// Every component declared in `files` of the descriptor set (protoc's
/// own bytes, as [`crate::markup_domain`] takes), with its name, in
/// declaration order: what an application's config and hand-written code
/// read the names from.
pub fn component_names(
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<Vec<ComponentName>, CodegenError> {
    Ok(model::components(descriptor_set, files, model::ALL_KINDS)?
        .into_iter()
        .map(|c| ComponentName {
            component: c.anchor.full_name().to_string(),
            kind: c.kind.name_kind(),
            name: c.component_name,
            domain: c.domain,
        })
        .collect())
}
