//! angzarr's declarations on the protos (`(command)`, `(event)`,
//! `(component)`, `refusal_status` and the rest) breaking a rule of the
//! generated domain layer, or asking for what this generator does not emit
//! yet. Each names the declaration to change.

use std::fmt;

use angzarr_diagnostic::{Arg, Diagnostic};

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
    /// An input with no emits whose `emits_cardinality` is ONE (`optional`
    /// false) or OPTIONAL: only MANY is accepted without emits.
    CardinalityWithoutEmits { message: String, optional: bool },
    /// A ONE or OPTIONAL input listing more emits than the markup's widest
    /// tuple holds.
    EmitsTuple { message: String, count: usize },
    /// A `refusal_status` on an enum value that is no refusal's code
    /// (decision 14.52): a value of an enum other than the nested `Reason`
    /// of a message some command refuses with, or that Reason's zero.
    RefusalStatusMisplaced { value: String },
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
            MarkupError::UnknownComponent { .. } => "angzarr-codegen-unknown-component",
            MarkupError::UnknownEmit { .. } => "angzarr-codegen-unknown-emit",
            MarkupError::UnknownRefusal { .. } => "angzarr-codegen-unknown-refusal",
            MarkupError::UnownedDomain { .. } => "angzarr-codegen-unowned-domain",
            MarkupError::UnsupportedKind { .. } => "angzarr-codegen-unsupported-kind",
            MarkupError::Commutative { .. } => "angzarr-codegen-commutative",
            MarkupError::RefusesOnFact { .. } => "angzarr-codegen-refuses-on-fact",
            MarkupError::InputDomainsOnNonProjector { .. } => {
                "angzarr-codegen-input-domains-on-non-projector"
            }
            MarkupError::InputDomainsWithInputDomain { .. } => {
                "angzarr-codegen-input-domains-with-input-domain"
            }
            MarkupError::DuplicateSource { .. } => "angzarr-codegen-duplicate-source",
            MarkupError::UnlistedSource { .. } => "angzarr-codegen-unlisted-source",
            MarkupError::UnprojectedSource { .. } => "angzarr-codegen-unprojected-source",
            MarkupError::ComponentName { .. } => "angzarr-codegen-component-name",
            MarkupError::CardinalityWithoutEmits { .. } => {
                "angzarr-codegen-cardinality-without-emits"
            }
            MarkupError::EmitsTuple { .. } => "angzarr-codegen-emits-tuple",
            MarkupError::RefusalStatusMisplaced { .. } => {
                "angzarr-codegen-refusal-status-misplaced"
            }
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
            E::CardinalityWithoutEmits { message, optional } => {
                let cardinality = if *optional { "OPTIONAL" } else { "ONE" };
                vec![
                    ("message", message.as_str().into()),
                    ("cardinality", cardinality.into()),
                ]
            }
            E::EmitsTuple { message, count } => {
                vec![
                    ("message", message.as_str().into()),
                    ("count", (*count).into()),
                ]
            }
            E::RefusalStatusMisplaced { value } => texts![value],
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
        E::CardinalityWithoutEmits {
            message: text("message"),
            optional: true,
        },
        E::EmitsTuple {
            message: text("message"),
            count: 5,
        },
        E::RefusalStatusMisplaced {
            value: text("value"),
        },
    ]
}
