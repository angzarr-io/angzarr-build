//! Why generation refused a descriptor set. Every refusal names the proto
//! element to change.

use std::fmt;

use annals_diagnostic::{Arg, Diagnostic};

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
            CodegenError::UnsupportedKind { component, kind } => write!(
                f,
                "{component} is a component of kind {kind}; only aggregates are generated"
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
            CodegenError::UnsupportedKind { .. } => "angzarr-codegen-unsupported-kind",
            CodegenError::Commutative { .. } => "angzarr-codegen-commutative",
            CodegenError::Oneof { .. } => "angzarr-codegen-oneof",
            CodegenError::NestedType { .. } => "angzarr-codegen-nested-type",
            CodegenError::WellKnownType { .. } => "angzarr-codegen-well-known-type",
            CodegenError::EnumZero { .. } => "angzarr-codegen-enum-zero",
            CodegenError::NoWirePath { .. } => "angzarr-codegen-no-wire-path",
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
        }
    }
}

/// One value of every error this crate defines, for the catalog test.
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
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
