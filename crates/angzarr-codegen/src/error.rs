//! Why generation refused a descriptor set. Every refusal names the proto
//! element to change.

use std::fmt;

use angzarr_diagnostic::{Arg, Diagnostic};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CodegenError {
    /// The bytes are not a descriptor set protoc could have written.
    Descriptors { detail: String },
    /// A file asked for is not in the descriptor set.
    FileNotInSet { file: String },
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
    /// A oneof, which the domain layer does not map yet.
    Oneof { message: String, oneof: String },
    /// A message or enum declared inside a message, which the domain layer
    /// does not map yet.
    NestedType { message: String, nested: String },
    /// A well-known type other than `google.protobuf.Timestamp`.
    WellKnownType {
        message: String,
        field: String,
        type_name: String,
    },
    /// An enum whose zero value is not `*_UNSPECIFIED`: zero is the wire's
    /// absence, never a domain value, so it must say so.
    EnumZero { enum_name: String, value: String },
    /// A package with domain types but no Rust path for its prost types.
    NoWirePath { package: String },
    /// A translate selection names a policy no message declares.
    TranslateUnknownPolicy { policy: String },
    /// A translate selection names a command that is not one of the
    /// policy's target's.
    TranslateNotATargetCommand { policy: String, command: String },
    /// A translate selection's fact is not an input of the policy's source.
    TranslateNotASourceFact { policy: String, fact: String },
    /// The user's file for a component holds no decorated impl of it, so
    /// `--missing` has nowhere to add stubs; the file is left as it is.
    ScaffoldImplNotFound { path: String, component: String },
    /// A fact (`(command).fact`) declaring `refuses`: a fact cannot be
    /// refused.
    RefusesOnFact { message: String },
    /// A message or enum named as one of the names the generated domain
    /// layer gives the runtime's types (`Context`, `PolicyContext`,
    /// `CommandContext`, `Rejection`, `Bug`, `Failed`, `BusinessStatus`,
    /// `RetryableStatus`): the generated code would
    /// collide, and nothing is renamed silently.
    ReservedName { message: String, name: String },
    /// A translate selection's fact is an input of the policy's source, but
    /// a command, not a fact.
    TranslateNotAFact { policy: String, fact: String },
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
    /// A domain type taking the name the domain layer gives the choice of a
    /// command refusing several types (`<Command>Refuses`).
    RefusalChoiceName {
        message: String,
        name: String,
        command: String,
    },
    /// The user's file takes an input as a command (`cmd`) where the proto
    /// declares a fact (`fact`), or the reverse. `expected` and `found` are
    /// the parameters' names.
    InputMismatch {
        path: String,
        message: String,
        expected: String,
        found: String,
    },
    /// The protos declare no component of the domain, or of the name,
    /// asked for.
    PythonNoDomain { domain: String },
    /// A host config (`angzarr.yaml`) that does not load: `cause` is the
    /// reader's error, naming for a decode the key path where it goes
    /// wrong (`component[1].kind`).
    AppConfig { cause: angzarr_diagnostic::Report },
    /// `angzarr scaffold app` refused the host config: it has lints
    /// ([`crate::AppLint`]), each reported on its own.
    AppRefused { lints: Vec<crate::AppLint> },
    /// An application's manifest (`Cargo.toml`) that does not read as one,
    /// or whose `[package.metadata.angzarr.domain]` is malformed.
    AppManifest { detail: String },
    /// A domain crate's path, or its wire types', that is no Rust path.
    AppDomainPath { path: String },
    /// A `refusal_status` on an enum value that is no refusal's code
    /// (decision 14.52): a value of an enum other than the nested `Reason`
    /// of a message some command refuses with, or that Reason's zero.
    RefusalStatusMisplaced { value: String },
}

/// What a [`CodegenError`] refuses, which splits its message and its
/// arguments by concern.
enum Concern {
    /// A descriptor set this generator cannot read or map.
    Shape,
    /// Markup that breaks a rule of the generated domain layer.
    Markup,
    /// A translate, scaffold, Python or host-config refusal.
    Use,
}

impl CodegenError {
    fn concern(&self) -> Concern {
        match self {
            CodegenError::Descriptors { .. }
            | CodegenError::FileNotInSet { .. }
            | CodegenError::UnknownComponent { .. }
            | CodegenError::UnknownEmit { .. }
            | CodegenError::UnknownRefusal { .. }
            | CodegenError::UnownedDomain { .. }
            | CodegenError::UnsupportedKind { .. }
            | CodegenError::Commutative { .. }
            | CodegenError::Oneof { .. }
            | CodegenError::NestedType { .. }
            | CodegenError::WellKnownType { .. }
            | CodegenError::EnumZero { .. }
            | CodegenError::NoWirePath { .. } => Concern::Shape,
            CodegenError::RefusesOnFact { .. }
            | CodegenError::ReservedName { .. }
            | CodegenError::ComponentName { .. }
            | CodegenError::InputDomainsOnNonProjector { .. }
            | CodegenError::InputDomainsWithInputDomain { .. }
            | CodegenError::DuplicateSource { .. }
            | CodegenError::UnlistedSource { .. }
            | CodegenError::UnprojectedSource { .. }
            | CodegenError::CardinalityWithoutEmits { .. }
            | CodegenError::EmitsTuple { .. }
            | CodegenError::RefusalChoiceName { .. }
            | CodegenError::RefusalStatusMisplaced { .. } => Concern::Markup,
            CodegenError::TranslateUnknownPolicy { .. }
            | CodegenError::TranslateNotATargetCommand { .. }
            | CodegenError::TranslateNotASourceFact { .. }
            | CodegenError::TranslateNotAFact { .. }
            | CodegenError::ScaffoldImplNotFound { .. }
            | CodegenError::InputMismatch { .. }
            | CodegenError::PythonNoDomain { .. }
            | CodegenError::AppConfig { .. }
            | CodegenError::AppRefused { .. }
            | CodegenError::AppManifest { .. }
            | CodegenError::AppDomainPath { .. } => Concern::Use,
        }
    }

    /// The message of a descriptor set this generator cannot read or map ([`Concern::Shape`]).
    fn fmt_shape(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Descriptors { detail } => {
                write!(f, "the descriptor set does not decode: {detail}")
            }
            CodegenError::FileNotInSet { file } => {
                write!(f, "{file} is not in the descriptor set")
            }
            CodegenError::UnknownComponent { message, component } => write!(
                f,
                "{message} names the component {component}, which no message declares"
            ),
            CodegenError::UnknownEmit { command, event } => write!(
                f,
                "{command} emits {event}, which is not an event of its component"
            ),
            CodegenError::UnknownRefusal { command, refusal } => write!(
                f,
                "{command} refuses {refusal}, which is not a message of the descriptor set"
            ),
            CodegenError::UnownedDomain { policy, domain } => write!(
                f,
                "{policy} names the domain {domain}, which no aggregate or process manager owns"
            ),
            CodegenError::UnsupportedKind { component, kind } => write!(
                f,
                "{component} is a component of kind {kind}, which this generator does not emit"
            ),
            CodegenError::Commutative { component } => write!(
                f,
                "{component}'s state lists COMMUTATIVE, which is not generated yet"
            ),
            CodegenError::Oneof { message, oneof } => {
                write!(
                    f,
                    "{message} has the oneof {oneof}, which is not mapped yet"
                )
            }
            CodegenError::NestedType { message, nested } => write!(
                f,
                "{message} declares {nested} inside it; declare it at the top level"
            ),
            CodegenError::WellKnownType {
                message,
                field,
                type_name,
            } => write!(
                f,
                "{message}.{field} is a {type_name}; only google.protobuf.Timestamp is mapped"
            ),
            CodegenError::EnumZero { enum_name, value } => write!(
                f,
                "{enum_name}'s zero value is {value}; name it *_UNSPECIFIED"
            ),
            CodegenError::NoWirePath { package } => write!(
                f,
                "no Rust path is given for the prost types of package {package}"
            ),
            _ => unreachable!("{self:?} is not a shape error: Concern routes it"),
        }
    }

    /// The message of markup that breaks a rule of the generated domain layer ([`Concern::Markup`]).
    fn fmt_markup(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::RefusesOnFact { message } => write!(
                f,
                "{message} is a fact and declares refuses; a fact cannot be refused"
            ),
            CodegenError::ReservedName { message, name } => write!(
                f,
                "{message} is named {name}, which the generated domain layer gives the runtime's own {name}; rename it"
            ),
            CodegenError::InputDomainsOnNonProjector { component } => write!(
                f,
                "{component} sets input_domains, which only a projector takes"
            ),
            CodegenError::InputDomainsWithInputDomain { component } => write!(
                f,
                "{component} sets both input_domains and input_domain; list its sources in input_domains"
            ),
            CodegenError::DuplicateSource { component, domain } => {
                write!(f, "{component} lists the source domain {domain} twice")
            }
            CodegenError::UnlistedSource {
                component,
                event,
                domain,
            } => write!(
                f,
                "{event}'s entry for {component} names the domain \"{domain}\", which is not one of its input_domains"
            ),
            CodegenError::UnprojectedSource { component, domain } => write!(
                f,
                "{component} lists the source domain {domain}, which no (event) entry for it names"
            ),
            CodegenError::ComponentName { component, name } => write!(
                f,
                "{component}'s name \"{name}\" is not lower-case letters, digits and -, starting with a letter; set its component_name"
            ),
            CodegenError::CardinalityWithoutEmits { message, optional } => write!(
                f,
                "{message} declares no emits and the emits cardinality {}; only MANY is accepted without emits",
                if *optional { "OPTIONAL" } else { "ONE" }
            ),
            CodegenError::EmitsTuple { message, count } => write!(
                f,
                "{message} emits {count} types as one tuple, more than the markup's four; declare it MANY"
            ),
            CodegenError::RefusalChoiceName {
                message,
                name,
                command,
            } => write!(
                f,
                "{message} is named {name}, which the generated domain layer gives the refusals of {command}; rename it"
            ),
            CodegenError::RefusalStatusMisplaced { value } => write!(
                f,
                "{value} is marked (io.angzarr.v1.refusal_status), which marks a refusal's code alone: a non-zero value of the nested Reason of a message a command refuses with"
            ),
            _ => unreachable!("{self:?} is not a markup error: Concern routes it"),
        }
    }

    /// The message of a translate, scaffold, Python or host-config refusal ([`Concern::Use`]).
    fn fmt_use(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::TranslateUnknownPolicy { policy } => {
                write!(
                    f,
                    "translate names the policy {policy}, which no message declares"
                )
            }
            CodegenError::TranslateNotATargetCommand { policy, command } => write!(
                f,
                "translate names {command}, which is not a command of {policy}'s target"
            ),
            CodegenError::TranslateNotASourceFact { policy, fact } => write!(
                f,
                "translate names {fact}, which is not an input of {policy}'s source"
            ),
            CodegenError::ScaffoldImplNotFound { path, component } => write!(
                f,
                "{path} holds no decorated impl of {component}; it is left as it is"
            ),
            CodegenError::TranslateNotAFact { policy, fact } => write!(
                f,
                "translate names {fact}, a command of {policy}'s source; it must name a fact"
            ),
            CodegenError::InputMismatch {
                path,
                message,
                expected,
                found,
            } => write!(
                f,
                "{path} takes {message} as {found}, but the proto declares it a {expected}"
            ),
            CodegenError::PythonNoDomain { domain } => {
                write!(
                    f,
                    "the protos declare no component of the domain, or named, {domain}"
                )
            }
            CodegenError::AppConfig { cause } => {
                write!(f, "the host config does not read as one: {cause}")
            }
            CodegenError::AppRefused { lints } => write!(
                f,
                "the host config has {} lints; nothing is written",
                lints.len()
            ),
            CodegenError::AppManifest { detail } => {
                write!(f, "the manifest does not read as one: {detail}")
            }
            CodegenError::AppDomainPath { path } => write!(
                f,
                "the domain crate's {path} is no Rust path: name it as Rust does (examples_proto, not examples-proto)"
            ),
            _ => unreachable!("{self:?} is not a use error: Concern routes it"),
        }
    }

    /// The arguments of a descriptor set this generator cannot read or map ([`Concern::Shape`]).
    fn shape_args(&self) -> Vec<(&'static str, Arg)> {
        let s = |v: &String| Arg::from(v.as_str());
        match self {
            CodegenError::Descriptors { detail } => vec![("detail", s(detail))],
            CodegenError::FileNotInSet { file } => vec![("file", s(file))],
            CodegenError::UnknownComponent { message, component } => {
                vec![("message", s(message)), ("component", s(component))]
            }
            CodegenError::UnknownEmit { command, event } => {
                vec![("command", s(command)), ("event", s(event))]
            }
            CodegenError::UnknownRefusal { command, refusal } => {
                vec![("command", s(command)), ("refusal", s(refusal))]
            }
            CodegenError::UnownedDomain { policy, domain } => {
                vec![("policy", s(policy)), ("domain", s(domain))]
            }
            CodegenError::UnsupportedKind { component, kind } => {
                vec![("component", s(component)), ("kind", (*kind).into())]
            }
            CodegenError::Commutative { component } => vec![("component", s(component))],
            CodegenError::Oneof { message, oneof } => {
                vec![("message", s(message)), ("oneof", s(oneof))]
            }
            CodegenError::NestedType { message, nested } => {
                vec![("message", s(message)), ("nested", s(nested))]
            }
            CodegenError::WellKnownType {
                message,
                field,
                type_name,
            } => vec![
                ("message", s(message)),
                ("field", s(field)),
                ("type_name", s(type_name)),
            ],
            CodegenError::EnumZero { enum_name, value } => {
                vec![("enum_name", s(enum_name)), ("value", s(value))]
            }
            CodegenError::NoWirePath { package } => vec![("package", s(package))],
            _ => unreachable!("{self:?} is not a shape error: Concern routes it"),
        }
    }

    /// The arguments of markup that breaks a rule of the generated domain layer ([`Concern::Markup`]).
    fn markup_args(&self) -> Vec<(&'static str, Arg)> {
        let s = |v: &String| Arg::from(v.as_str());
        match self {
            CodegenError::RefusesOnFact { message } => vec![("message", s(message))],
            CodegenError::ReservedName { message, name } => {
                vec![("message", s(message)), ("name", s(name))]
            }
            CodegenError::ComponentName { component, name } => {
                vec![("component", s(component)), ("name", s(name))]
            }
            CodegenError::InputDomainsOnNonProjector { component }
            | CodegenError::InputDomainsWithInputDomain { component } => {
                vec![("component", s(component))]
            }
            CodegenError::DuplicateSource { component, domain }
            | CodegenError::UnprojectedSource { component, domain } => {
                vec![("component", s(component)), ("domain", s(domain))]
            }
            CodegenError::UnlistedSource {
                component,
                event,
                domain,
            } => vec![
                ("component", s(component)),
                ("event", s(event)),
                ("domain", s(domain)),
            ],
            CodegenError::CardinalityWithoutEmits { message, optional } => vec![
                ("message", s(message)),
                (
                    "cardinality",
                    Arg::from(if *optional { "OPTIONAL" } else { "ONE" }),
                ),
            ],
            CodegenError::EmitsTuple { message, count } => {
                vec![("message", s(message)), ("count", (*count).into())]
            }
            CodegenError::RefusalChoiceName {
                message,
                name,
                command,
            } => vec![
                ("message", s(message)),
                ("name", s(name)),
                ("command", s(command)),
            ],
            CodegenError::RefusalStatusMisplaced { value } => vec![("value", s(value))],
            _ => unreachable!("{self:?} is not a markup error: Concern routes it"),
        }
    }

    /// The arguments of a translate, scaffold, Python or host-config refusal ([`Concern::Use`]).
    fn use_args(&self) -> Vec<(&'static str, Arg)> {
        let s = |v: &String| Arg::from(v.as_str());
        match self {
            CodegenError::TranslateUnknownPolicy { policy } => vec![("policy", s(policy))],
            CodegenError::TranslateNotATargetCommand { policy, command } => {
                vec![("policy", s(policy)), ("command", s(command))]
            }
            CodegenError::TranslateNotASourceFact { policy, fact } => {
                vec![("policy", s(policy)), ("fact", s(fact))]
            }
            CodegenError::ScaffoldImplNotFound { path, component } => {
                vec![("path", s(path)), ("component", s(component))]
            }
            CodegenError::TranslateNotAFact { policy, fact } => {
                vec![("policy", s(policy)), ("fact", s(fact))]
            }
            CodegenError::InputMismatch {
                path,
                message,
                expected,
                found,
            } => vec![
                ("path", s(path)),
                ("message", s(message)),
                ("expected", s(expected)),
                ("found", s(found)),
            ],
            CodegenError::PythonNoDomain { domain } => vec![("domain", s(domain))],
            CodegenError::AppConfig { cause } => vec![("cause", Arg::nested(cause))],
            CodegenError::AppManifest { detail } => vec![("detail", s(detail))],
            CodegenError::AppDomainPath { path } => vec![("path", s(path))],
            CodegenError::AppRefused { lints } => {
                vec![(
                    "count",
                    i64::try_from(lints.len()).unwrap_or(i64::MAX).into(),
                )]
            }
            _ => unreachable!("{self:?} is not a use error: Concern routes it"),
        }
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.concern() {
            Concern::Shape => self.fmt_shape(f),
            Concern::Markup => self.fmt_markup(f),
            Concern::Use => self.fmt_use(f),
        }
    }
}

impl std::error::Error for CodegenError {}

impl Diagnostic for CodegenError {
    fn code(&self) -> &'static str {
        match self {
            CodegenError::Descriptors { .. } => "angzarr-codegen-descriptors",
            CodegenError::FileNotInSet { .. } => "angzarr-codegen-file-not-in-set",
            CodegenError::UnknownComponent { .. } => "angzarr-codegen-unknown-component",
            CodegenError::UnknownEmit { .. } => "angzarr-codegen-unknown-emit",
            CodegenError::UnknownRefusal { .. } => "angzarr-codegen-unknown-refusal",
            CodegenError::UnownedDomain { .. } => "angzarr-codegen-unowned-domain",
            CodegenError::UnsupportedKind { .. } => "angzarr-codegen-unsupported-kind",
            CodegenError::Commutative { .. } => "angzarr-codegen-commutative",
            CodegenError::Oneof { .. } => "angzarr-codegen-oneof",
            CodegenError::NestedType { .. } => "angzarr-codegen-nested-type",
            CodegenError::WellKnownType { .. } => "angzarr-codegen-well-known-type",
            CodegenError::EnumZero { .. } => "angzarr-codegen-enum-zero",
            CodegenError::NoWirePath { .. } => "angzarr-codegen-no-wire-path",
            CodegenError::TranslateUnknownPolicy { .. } => {
                "angzarr-codegen-translate-unknown-policy"
            }
            CodegenError::TranslateNotATargetCommand { .. } => {
                "angzarr-codegen-translate-not-a-target-command"
            }
            CodegenError::TranslateNotASourceFact { .. } => {
                "angzarr-codegen-translate-not-a-source-fact"
            }
            CodegenError::ScaffoldImplNotFound { .. } => "angzarr-codegen-scaffold-impl-not-found",
            CodegenError::RefusesOnFact { .. } => "angzarr-codegen-refuses-on-fact",
            CodegenError::ReservedName { .. } => "angzarr-codegen-reserved-name",
            CodegenError::TranslateNotAFact { .. } => "angzarr-codegen-translate-not-a-fact",
            CodegenError::RefusalChoiceName { .. } => "angzarr-codegen-refusal-choice-name",
            CodegenError::ComponentName { .. } => "angzarr-codegen-component-name",
            CodegenError::InputDomainsOnNonProjector { .. } => {
                "angzarr-codegen-input-domains-on-non-projector"
            }
            CodegenError::InputDomainsWithInputDomain { .. } => {
                "angzarr-codegen-input-domains-with-input-domain"
            }
            CodegenError::DuplicateSource { .. } => "angzarr-codegen-duplicate-source",
            CodegenError::UnlistedSource { .. } => "angzarr-codegen-unlisted-source",
            CodegenError::UnprojectedSource { .. } => "angzarr-codegen-unprojected-source",
            CodegenError::CardinalityWithoutEmits { .. } => {
                "angzarr-codegen-cardinality-without-emits"
            }
            CodegenError::EmitsTuple { .. } => "angzarr-codegen-emits-tuple",
            CodegenError::InputMismatch { .. } => "angzarr-codegen-input-mismatch",
            CodegenError::PythonNoDomain { .. } => "angzarr-codegen-python-no-domain",
            CodegenError::AppConfig { .. } => "angzarr-codegen-app-config",
            CodegenError::AppRefused { .. } => "angzarr-codegen-app-refused",
            CodegenError::AppManifest { .. } => "angzarr-codegen-app-manifest",
            CodegenError::AppDomainPath { .. } => "angzarr-codegen-app-domain-path",
            CodegenError::RefusalStatusMisplaced { .. } => {
                "angzarr-codegen-refusal-status-misplaced"
            }
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        match self.concern() {
            Concern::Shape => self.shape_args(),
            Concern::Markup => self.markup_args(),
            Concern::Use => self.use_args(),
        }
    }
}

/// One value of every error this crate defines, for the catalog test.
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
    shape_samples()
        .into_iter()
        .chain(rule_samples())
        .chain(crate::app::app_lint_samples())
        .collect()
}

/// One value of every [`Concern::Shape`] error.
fn shape_samples() -> Vec<Box<dyn Diagnostic>> {
    let s = || "x".to_string();
    vec![
        Box::new(CodegenError::Descriptors { detail: s() }),
        Box::new(CodegenError::FileNotInSet { file: s() }),
        Box::new(CodegenError::UnknownComponent {
            message: s(),
            component: s(),
        }),
        Box::new(CodegenError::UnknownEmit {
            command: s(),
            event: s(),
        }),
        Box::new(CodegenError::UnknownRefusal {
            command: s(),
            refusal: s(),
        }),
        Box::new(CodegenError::UnownedDomain {
            policy: s(),
            domain: s(),
        }),
        Box::new(CodegenError::UnsupportedKind {
            component: s(),
            kind: 4,
        }),
        Box::new(CodegenError::Commutative { component: s() }),
        Box::new(CodegenError::Oneof {
            message: s(),
            oneof: s(),
        }),
        Box::new(CodegenError::NestedType {
            message: s(),
            nested: s(),
        }),
        Box::new(CodegenError::WellKnownType {
            message: s(),
            field: s(),
            type_name: s(),
        }),
        Box::new(CodegenError::EnumZero {
            enum_name: s(),
            value: s(),
        }),
        Box::new(CodegenError::NoWirePath { package: s() }),
    ]
}

/// One value of every other error, a [`Concern::Markup`] or
/// [`Concern::Use`] one.
fn rule_samples() -> Vec<Box<dyn Diagnostic>> {
    let s = || "x".to_string();
    vec![
        Box::new(CodegenError::TranslateUnknownPolicy { policy: s() }),
        Box::new(CodegenError::TranslateNotATargetCommand {
            policy: s(),
            command: s(),
        }),
        Box::new(CodegenError::TranslateNotASourceFact {
            policy: s(),
            fact: s(),
        }),
        Box::new(CodegenError::ScaffoldImplNotFound {
            path: s(),
            component: s(),
        }),
        Box::new(CodegenError::RefusesOnFact { message: s() }),
        Box::new(CodegenError::TranslateNotAFact {
            policy: s(),
            fact: s(),
        }),
        Box::new(CodegenError::ComponentName {
            component: s(),
            name: s(),
        }),
        Box::new(CodegenError::InputDomainsOnNonProjector { component: s() }),
        Box::new(CodegenError::InputDomainsWithInputDomain { component: s() }),
        Box::new(CodegenError::DuplicateSource {
            component: s(),
            domain: s(),
        }),
        Box::new(CodegenError::UnlistedSource {
            component: s(),
            event: s(),
            domain: s(),
        }),
        Box::new(CodegenError::UnprojectedSource {
            component: s(),
            domain: s(),
        }),
        Box::new(CodegenError::CardinalityWithoutEmits {
            message: s(),
            optional: true,
        }),
        Box::new(CodegenError::EmitsTuple {
            message: s(),
            count: 5,
        }),
        Box::new(CodegenError::RefusalChoiceName {
            message: s(),
            name: s(),
            command: s(),
        }),
        Box::new(CodegenError::InputMismatch {
            path: s(),
            message: s(),
            expected: s(),
            found: s(),
        }),
        Box::new(CodegenError::PythonNoDomain { domain: s() }),
        Box::new(CodegenError::ReservedName {
            message: s(),
            name: s(),
        }),
        Box::new(CodegenError::AppConfig {
            cause: angzarr_diagnostic::Report::new("angzarr-yaml-load-failed-at")
                .arg("kind", "angzarr-host-config")
                .arg("source", "angzarr.yaml"),
        }),
        Box::new(CodegenError::AppRefused { lints: Vec::new() }),
        Box::new(CodegenError::AppManifest { detail: s() }),
        Box::new(CodegenError::AppDomainPath { path: s() }),
        Box::new(CodegenError::RefusalStatusMisplaced { value: s() }),
    ]
}
