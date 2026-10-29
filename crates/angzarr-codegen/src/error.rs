//! Why generation refused a descriptor set. Every refusal names the proto
//! element to change.

use std::fmt;

use angzarr_embedded_diagnostic::{Arg, Diagnostic};

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
    /// `CommandContext`, `Rejection`, `Bug`, `Failed`, `BusinessStatus`): the generated code would
    /// collide, and nothing is renamed silently.
    ReservedName { message: String, name: String },
    /// A translate selection's fact is an input of the policy's source, but
    /// a command, not a fact.
    TranslateNotAFact { policy: String, fact: String },
    /// A command declaring more than one typed refusal, which the scaffold
    /// does not emit yet.
    SeveralRefusals { command: String },
    /// The user's file takes an input as a command (`cmd`) where the proto
    /// declares a fact (`fact`), or the reverse. `expected` and `found` are
    /// the parameters' names.
    InputMismatch {
        path: String,
        message: String,
        expected: String,
        found: String,
    },
    /// The protos declare no aggregate of the domain asked for.
    PythonNoDomain { domain: String },
    /// A component kind the Python scaffold does not emit yet: only
    /// aggregates.
    PythonKind { component: String },
    /// A fact, which the Python scaffold does not emit yet.
    PythonFact { message: String },
    /// A command declaring typed refusals, which the Python scaffold does
    /// not emit yet.
    PythonRefuses { message: String },
    /// The startup's components span more than one proto package, which
    /// its `main.rs` does not lay out yet.
    StartupPackages { files: String },
    /// A host config that does not read as one.
    StartupConfig { detail: String },
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            CodegenError::RefusesOnFact { message } => write!(
                f,
                "{message} is a fact and declares refuses; a fact cannot be refused"
            ),
            CodegenError::ReservedName { message, name } => write!(
                f,
                "{message} is named {name}, which the generated domain layer gives the runtime's own {name}; rename it"
            ),
            CodegenError::TranslateNotAFact { policy, fact } => write!(
                f,
                "translate names {fact}, a command of {policy}'s source; it must name a fact"
            ),
            CodegenError::SeveralRefusals { command } => write!(
                f,
                "{command} declares more than one refusal, which the scaffold does not emit yet"
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
                write!(f, "the protos declare no aggregate of the domain {domain}")
            }
            CodegenError::PythonKind { component } => write!(
                f,
                "{component} is not an aggregate; the Python scaffold emits aggregates only"
            ),
            CodegenError::PythonFact { message } => write!(
                f,
                "{message} is a fact, which the Python scaffold does not emit yet"
            ),
            CodegenError::PythonRefuses { message } => write!(
                f,
                "{message} declares refuses, which the Python scaffold does not emit yet"
            ),
            CodegenError::StartupPackages { files } => write!(
                f,
                "{files} declare components of more than one package, which the startup does not lay out yet"
            ),
            CodegenError::StartupConfig { detail } => {
                write!(f, "the host config does not read: {detail}")
            }
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
            CodegenError::SeveralRefusals { .. } => "angzarr-codegen-several-refusals",
            CodegenError::InputMismatch { .. } => "angzarr-codegen-input-mismatch",
            CodegenError::PythonNoDomain { .. } => "angzarr-codegen-python-no-domain",
            CodegenError::PythonKind { .. } => "angzarr-codegen-python-kind",
            CodegenError::PythonFact { .. } => "angzarr-codegen-python-fact",
            CodegenError::PythonRefuses { .. } => "angzarr-codegen-python-refuses",
            CodegenError::StartupPackages { .. } => "angzarr-codegen-startup-packages",
            CodegenError::StartupConfig { .. } => "angzarr-codegen-startup-config",
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
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
            CodegenError::RefusesOnFact { message } => vec![("message", s(message))],
            CodegenError::ReservedName { message, name } => {
                vec![("message", s(message)), ("name", s(name))]
            }
            CodegenError::TranslateNotAFact { policy, fact } => {
                vec![("policy", s(policy)), ("fact", s(fact))]
            }
            CodegenError::SeveralRefusals { command } => vec![("command", s(command))],
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
            CodegenError::PythonKind { component } => vec![("component", s(component))],
            CodegenError::PythonFact { message } | CodegenError::PythonRefuses { message } => {
                vec![("message", s(message))]
            }
            CodegenError::StartupPackages { files } => vec![("files", s(files))],
            CodegenError::StartupConfig { detail } => vec![("detail", s(detail))],
        }
    }
}

/// One value of every error this crate defines, for the catalog test.
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
    let s = || "x".to_string();
    let mut samples: Vec<Box<dyn Diagnostic>> = vec![
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
        Box::new(CodegenError::SeveralRefusals { command: s() }),
        Box::new(CodegenError::InputMismatch {
            path: s(),
            message: s(),
            expected: s(),
            found: s(),
        }),
        Box::new(CodegenError::PythonNoDomain { domain: s() }),
        Box::new(CodegenError::PythonKind { component: s() }),
        Box::new(CodegenError::PythonFact { message: s() }),
        Box::new(CodegenError::ReservedName {
            message: s(),
            name: s(),
        }),
        Box::new(CodegenError::PythonRefuses { message: s() }),
        Box::new(CodegenError::StartupPackages { files: s() }),
        Box::new(CodegenError::StartupConfig { detail: s() }),
    ];
    samples.extend(crate::startup::startup_lint_samples());
    samples
}
