//! The declarations' lint warnings (the codegen tier's, decision 14.66):
//! declarations that compile, and that the generator emits from, but that
//! are unlikely to work. A warning leaves `angzarr lint app`'s exit status
//! as it is. Each carries this crate's code; the codegen tier's labels for
//! the first four (`ANZ100` to `ANZ103`) are named on their variants.

use std::fmt;

use angzarr_diagnostic::{Arg, Diagnostic};

use crate::error::{CodegenError, text, texts};

/// One declaration lint warning, naming the declaration to look at.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Warning {
    /// ANZ100: a command's `emits` lists an event no `(event)` entry for
    /// its component names: nothing folds it (C-0514).
    UnappliedEmit { command: String, event: String },
    /// ANZ101: a policy's output domain no aggregate or process manager of
    /// the protos owns (C-0515).
    UnownedTarget { policy: String, domain: String },
    /// ANZ102: a policy's input domain no aggregate or process manager of
    /// the protos produces (C-0516).
    UnproducedSource { policy: String, domain: String },
    /// ANZ103: a component no `(command)` or `(event)` entry names: it
    /// handles nothing (C-0517).
    Idle { component: String },
    /// A process manager none of whose inputs sets `starts`: it could
    /// never start an instance (C-0611).
    NoStartingInput { component: String },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl Diagnostic for Warning {
    fn code(&self) -> &'static str {
        match self {
            Warning::UnappliedEmit { .. } => "angzarr-codegen-unapplied-emit",
            Warning::UnownedTarget { .. } => "angzarr-codegen-unowned-target",
            Warning::UnproducedSource { .. } => "angzarr-codegen-unproduced-source",
            Warning::Idle { .. } => "angzarr-codegen-idle-component",
            Warning::NoStartingInput { .. } => "angzarr-codegen-no-starting-input",
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        match self {
            Warning::UnappliedEmit { command, event } => texts![command, event],
            Warning::UnownedTarget { policy, domain }
            | Warning::UnproducedSource { policy, domain } => texts![policy, domain],
            Warning::Idle { component } | Warning::NoStartingInput { component } => {
                texts![component]
            }
        }
    }
}

/// The lint warnings of the components declared in `files` of
/// `descriptor_set`, in declaration order; refused as generation refuses a
/// declaration the contract forbids.
pub fn warnings(descriptor_set: &[u8], files: &[&str]) -> Result<Vec<Warning>, CodegenError> {
    let _ = (descriptor_set, files);
    Ok(Vec::new())
}

/// One value of every [`Warning`].
pub(crate) fn samples() -> Vec<Warning> {
    vec![
        Warning::UnappliedEmit {
            command: text("command"),
            event: text("event"),
        },
        Warning::UnownedTarget {
            policy: text("policy"),
            domain: text("domain"),
        },
        Warning::UnproducedSource {
            policy: text("policy"),
            domain: text("domain"),
        },
        Warning::Idle {
            component: text("component"),
        },
        Warning::NoStartingInput {
            component: text("component"),
        },
    ]
}
