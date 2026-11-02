//! angzarr's declarations on the protos (`(command)`, `(event)`,
//! `(component)`, `refusal_status` and the rest) breaking a rule of the
//! generated domain layer, or asking for what this generator does not emit
//! yet. Each names the declaration to change.

use std::fmt;

use angzarr_diagnostic::{Arg, Diagnostic, code};

use super::{text, texts};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MarkupError {
    /// A `(command)` or `(event)` names a component no message declares.
    UnknownComponent { message: String, component: String },
    /// A command's `emits` names a message that is not one of its
    /// component's events.
    UnknownEmit { command: String, event: String },
    /// A command's `refuses` names a message the descriptor set lacks.
    UnknownRefusal { command: String, refusal: String },
    /// A policy's `input_domain` or `output_domain` names a domain no
    /// aggregate or process manager owns.
    UnownedDomain { policy: String, domain: String },
    /// A component kind this generator does not emit yet.
    UnsupportedKind { component: String, kind: i32 },
    /// A state listing COMMUTATIVE, whose `changed` this generator does not
    /// emit yet.
    Commutative { component: String },
    /// A fact (`(command).fact`) declaring `refuses`: a fact cannot be
    /// refused.
    RefusesOnFact { message: String },
    /// `input_domains` on a component that is not a projector.
    InputDomainsOnNonProjector { component: String },
    /// A projector setting both `input_domains` and `input_domain`.
    InputDomainsWithInputDomain { component: String },
    /// A projector listing one source domain twice.
    DuplicateSource { component: String, domain: String },
    /// A projector's `(event)` entry whose domain (empty: unset) is not one
    /// of its listed sources.
    UnlistedSource {
        component: String,
        event: String,
        domain: String,
    },
    /// A projector listing a source domain no `(event)` entry names.
    UnprojectedSource { component: String, domain: String },
    /// A component's name in an application (declared, or its kind's
    /// default) that is not lower-case ASCII letters, digits and `-`,
    /// starting with a letter.
    ComponentName { component: String, name: String },
    /// A command of a policy's target declaring no `refuses`: a policy may
    /// send any command of its target, and every command it sends has a
    /// refusal path (decision 14.76; C-0798).
    SentCommandRefusesNothing { policy: String, command: String },
    /// A policy whose target takes a command, sourced from a domain that
    /// takes no fact: its refusal callback answers one of its source's
    /// facts (decision 14.76; C-0799).
    SourceTakesNoFact { policy: String, domain: String },
    /// A `refusal_status` on an enum value that is no refusal's code
    /// (decision 14.52): a value of an enum other than the nested `Reason`
    /// of a message some command refuses with, or that Reason's zero.
    RefusalStatusMisplaced { value: String },
    /// A process manager's `(event)` entry naming a domain other than its
    /// own: a process manager subscribes to nothing; another domain's
    /// events reach it as facts through policies (decision 14.31; C-0604).
    ProcessManagerSubscribes {
        component: String,
        event: String,
        domain: String,
    },
    /// A process manager declaring an `output_domain`: its decisions reach
    /// other domains through policies from its domain (decision 14.31;
    /// C-0605).
    ProcessManagerOutputDomain { component: String, domain: String },
    /// A policy declaring no `output_domain`: its one destination (C-0582).
    NoOutputDomain { component: String },
    /// An input of a component other than a process manager setting
    /// `starts`: only a process manager's input starts an instance
    /// (C-0606).
    StartsOnNonProcessManager { message: String, component: String },
    /// A command's `refuses` naming a message of another package, another
    /// domain's: a command refuses with its own domain's types (C-0626).
    ForeignRefusal { command: String, refusal: String },
}

impl fmt::Display for MarkupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl std::error::Error for MarkupError {}

impl Diagnostic for MarkupError {
    fn code(&self) -> &'static str {
        match self {
            MarkupError::UnknownComponent { .. } => code!("unknown-component"),
            MarkupError::UnknownEmit { .. } => code!("unknown-emit"),
            MarkupError::UnknownRefusal { .. } => code!("unknown-refusal"),
            MarkupError::UnownedDomain { .. } => code!("unowned-domain"),
            MarkupError::UnsupportedKind { .. } => code!("unsupported-kind"),
            MarkupError::Commutative { .. } => code!("commutative"),
            MarkupError::RefusesOnFact { .. } => code!("refuses-on-fact"),
            MarkupError::InputDomainsOnNonProjector { .. } => {
                code!("input-domains-on-non-projector")
            }
            MarkupError::InputDomainsWithInputDomain { .. } => {
                code!("input-domains-with-input-domain")
            }
            MarkupError::DuplicateSource { .. } => code!("duplicate-source"),
            MarkupError::UnlistedSource { .. } => code!("unlisted-source"),
            MarkupError::UnprojectedSource { .. } => code!("unprojected-source"),
            MarkupError::ComponentName { .. } => code!("component-name"),
            MarkupError::SentCommandRefusesNothing { .. } => {
                code!("sent-command-refuses-nothing")
            }
            MarkupError::SourceTakesNoFact { .. } => code!("source-takes-no-fact"),
            MarkupError::RefusalStatusMisplaced { .. } => {
                code!("refusal-status-misplaced")
            }
            MarkupError::ProcessManagerSubscribes { .. } => {
                code!("process-manager-subscribes")
            }
            MarkupError::ProcessManagerOutputDomain { .. } => {
                code!("process-manager-output-domain")
            }
            MarkupError::NoOutputDomain { .. } => code!("no-output-domain"),
            MarkupError::StartsOnNonProcessManager { .. } => {
                code!("starts-on-non-process-manager")
            }
            MarkupError::ForeignRefusal { .. } => code!("foreign-refusal"),
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        use MarkupError as E;
        match self {
            E::UnknownComponent { message, component } => texts![message, component],
            E::UnknownEmit { command, event } => texts![command, event],
            E::UnknownRefusal { command, refusal } => texts![command, refusal],
            E::UnownedDomain { policy, domain } => texts![policy, domain],
            E::UnsupportedKind { component, kind } => {
                vec![
                    ("component", component.as_str().into()),
                    ("kind", (*kind).into()),
                ]
            }
            E::Commutative { component }
            | E::InputDomainsOnNonProjector { component }
            | E::InputDomainsWithInputDomain { component } => texts![component],
            E::RefusesOnFact { message } => texts![message],
            E::DuplicateSource { component, domain }
            | E::UnprojectedSource { component, domain } => {
                texts![component, domain]
            }
            E::UnlistedSource {
                component,
                event,
                domain,
            } => texts![component, event, domain],
            E::ComponentName { component, name } => texts![component, name],
            E::SentCommandRefusesNothing { policy, command } => texts![policy, command],
            E::SourceTakesNoFact { policy, domain } => texts![policy, domain],
            E::RefusalStatusMisplaced { value } => texts![value],
            E::ProcessManagerSubscribes {
                component,
                event,
                domain,
            } => texts![component, event, domain],
            E::ProcessManagerOutputDomain { component, domain } => texts![component, domain],
            E::NoOutputDomain { component } => texts![component],
            E::StartsOnNonProcessManager { message, component } => texts![message, component],
            E::ForeignRefusal { command, refusal } => texts![command, refusal],
        }
    }
}

/// One value of every [`MarkupError`].
pub(super) fn samples() -> Vec<MarkupError> {
    use MarkupError as E;
    vec![
        E::UnknownComponent {
            message: text("message"),
            component: text("component"),
        },
        E::UnknownEmit {
            command: text("command"),
            event: text("event"),
        },
        E::UnknownRefusal {
            command: text("command"),
            refusal: text("refusal"),
        },
        E::UnownedDomain {
            policy: text("policy"),
            domain: text("domain"),
        },
        E::UnsupportedKind {
            component: text("component"),
            kind: 4,
        },
        E::Commutative {
            component: text("component"),
        },
        E::RefusesOnFact {
            message: text("message"),
        },
        E::InputDomainsOnNonProjector {
            component: text("component"),
        },
        E::InputDomainsWithInputDomain {
            component: text("component"),
        },
        E::DuplicateSource {
            component: text("component"),
            domain: text("domain"),
        },
        E::UnlistedSource {
            component: text("component"),
            event: text("event"),
            domain: text("domain"),
        },
        E::UnprojectedSource {
            component: text("component"),
            domain: text("domain"),
        },
        E::ComponentName {
            component: text("component"),
            name: text("name"),
        },
        E::SentCommandRefusesNothing {
            policy: text("policy"),
            command: text("command"),
        },
        E::SourceTakesNoFact {
            policy: text("policy"),
            domain: text("domain"),
        },
        E::RefusalStatusMisplaced {
            value: text("value"),
        },
        E::ProcessManagerSubscribes {
            component: text("component"),
            event: text("event"),
            domain: text("domain"),
        },
        E::ProcessManagerOutputDomain {
            component: text("component"),
            domain: text("domain"),
        },
        E::NoOutputDomain {
            component: text("component"),
        },
        E::StartsOnNonProcessManager {
            message: text("message"),
            component: text("component"),
        },
        E::ForeignRefusal {
            command: text("command"),
            refusal: text("refusal"),
        },
    ]
}
