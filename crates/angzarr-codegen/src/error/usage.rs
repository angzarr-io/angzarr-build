//! A use of the protos they cannot serve: what a caller asks of them (a
//! file, the wire path of a package's prost types, a translate selection,
//! the Python domain), or a user's file read back to complete that does
//! not match them. Each names what was asked for, or the user's file.

use std::fmt;

use angzarr_diagnostic::{Arg, Diagnostic, code};

use super::{text, texts};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UseError {
    /// A file asked for is not in the descriptor set.
    FileNotInSet { file: String },
    /// A package with domain types but no Rust path for its prost types.
    NoWirePath { package: String },
    /// A translate selection names a policy no message declares.
    TranslateUnknownPolicy { policy: String },
    /// A translate selection names a command that is not one of the
    /// policy's target's.
    TranslateNotATargetCommand { policy: String, command: String },
    /// A translate selection's fact is not an input of the policy's source.
    TranslateNotASourceFact { policy: String, fact: String },
    /// A translate selection's fact is an input of the policy's source, but
    /// a command, not a fact.
    TranslateNotAFact { policy: String, fact: String },
    /// The protos declare no component of the domain, or of the name,
    /// asked for.
    PythonNoDomain { domain: String },
    /// The user's file for a component holds no decorated impl of it, so
    /// `--missing` has nowhere to add stubs; the file is left as it is.
    ScaffoldImplNotFound { path: String, component: String },
    /// The user's file takes an input as a command (`cmd`) where the proto
    /// declares a fact (`fact`), or the reverse. `expected` and `found` are
    /// the parameters' names.
    InputMismatch {
        path: String,
        message: String,
        expected: String,
        found: String,
    },
}

impl fmt::Display for UseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl std::error::Error for UseError {}

impl Diagnostic for UseError {
    fn code(&self) -> &'static str {
        match self {
            UseError::FileNotInSet { .. } => code!("file-not-in-set"),
            UseError::NoWirePath { .. } => code!("no-wire-path"),
            UseError::TranslateUnknownPolicy { .. } => code!("translate-unknown-policy"),
            UseError::TranslateNotATargetCommand { .. } => {
                code!("translate-not-a-target-command")
            }
            UseError::TranslateNotASourceFact { .. } => {
                code!("translate-not-a-source-fact")
            }
            UseError::TranslateNotAFact { .. } => code!("translate-not-a-fact"),
            UseError::PythonNoDomain { .. } => code!("python-no-domain"),
            UseError::ScaffoldImplNotFound { .. } => code!("scaffold-impl-not-found"),
            UseError::InputMismatch { .. } => code!("input-mismatch"),
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        use UseError as E;
        match self {
            E::FileNotInSet { file } => texts![file],
            E::NoWirePath { package } => texts![package],
            E::TranslateUnknownPolicy { policy } => texts![policy],
            E::TranslateNotATargetCommand { policy, command } => texts![policy, command],
            E::TranslateNotASourceFact { policy, fact } | E::TranslateNotAFact { policy, fact } => {
                texts![policy, fact]
            }
            E::PythonNoDomain { domain } => texts![domain],
            E::ScaffoldImplNotFound { path, component } => texts![path, component],
            E::InputMismatch {
                path,
                message,
                expected,
                found,
            } => texts![path, message, expected, found],
        }
    }
}

/// One value of every [`UseError`].
pub(super) fn samples() -> Vec<UseError> {
    use UseError as E;
    vec![
        E::FileNotInSet { file: text("file") },
        E::NoWirePath {
            package: text("package"),
        },
        E::TranslateUnknownPolicy {
            policy: text("policy"),
        },
        E::TranslateNotATargetCommand {
            policy: text("policy"),
            command: text("command"),
        },
        E::TranslateNotASourceFact {
            policy: text("policy"),
            fact: text("fact"),
        },
        E::TranslateNotAFact {
            policy: text("policy"),
            fact: text("fact"),
        },
        E::PythonNoDomain {
            domain: text("domain"),
        },
        E::ScaffoldImplNotFound {
            path: text("path"),
            component: text("component"),
        },
        E::InputMismatch {
            path: text("path"),
            message: text("message"),
            expected: text("expected"),
            found: text("found"),
        },
    ]
}
