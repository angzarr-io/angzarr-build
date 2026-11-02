//! The descriptor set as protobuf: bytes this generator cannot read, or
//! types and names the generated domain layer cannot map. Each names the
//! proto element to change.

use std::fmt;

use angzarr_diagnostic::{Arg, Diagnostic, code};

use super::{text, texts};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProtoError {
    /// The bytes are not a descriptor set protoc could have written.
    Descriptors { detail: String },
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
    /// A message or enum named as one of the names the generated domain
    /// layer gives the runtime's types (`Context`, `PolicyContext`,
    /// `CommandContext`, `Rejection`, `Bug`, `Failed`, `BusinessStatus`,
    /// `RetryableStatus`): the generated code would
    /// collide, and nothing is renamed silently.
    ReservedName { message: String, name: String },
    /// A domain type taking the name the domain layer gives the choice of a
    /// command refusing several types (`<Command>Refuses`).
    RefusalChoiceName {
        message: String,
        name: String,
        command: String,
    },
}

impl fmt::Display for ProtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl std::error::Error for ProtoError {}

impl Diagnostic for ProtoError {
    fn code(&self) -> &'static str {
        match self {
            ProtoError::Descriptors { .. } => code!("descriptors"),
            ProtoError::Oneof { .. } => code!("oneof"),
            ProtoError::NestedType { .. } => code!("nested-type"),
            ProtoError::WellKnownType { .. } => code!("well-known-type"),
            ProtoError::EnumZero { .. } => code!("enum-zero"),
            ProtoError::ReservedName { .. } => code!("reserved-name"),
            ProtoError::RefusalChoiceName { .. } => code!("refusal-choice-name"),
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        use ProtoError as E;
        match self {
            E::Descriptors { detail } => texts![detail],
            E::Oneof { message, oneof } => texts![message, oneof],
            E::NestedType { message, nested } => texts![message, nested],
            E::WellKnownType {
                message,
                field,
                type_name,
            } => texts![message, field, type_name],
            E::EnumZero { enum_name, value } => texts![enum_name, value],
            E::ReservedName { message, name } => texts![message, name],
            E::RefusalChoiceName {
                message,
                name,
                command,
            } => texts![message, name, command],
        }
    }
}

/// One value of every [`ProtoError`].
pub(super) fn samples() -> Vec<ProtoError> {
    use ProtoError as E;
    vec![
        E::Descriptors {
            detail: text("detail"),
        },
        E::Oneof {
            message: text("message"),
            oneof: text("oneof"),
        },
        E::NestedType {
            message: text("message"),
            nested: text("nested"),
        },
        E::WellKnownType {
            message: text("message"),
            field: text("field"),
            type_name: text("type_name"),
        },
        E::EnumZero {
            enum_name: text("enum_name"),
            value: text("value"),
        },
        E::ReservedName {
            message: text("message"),
            name: text("name"),
        },
        E::RefusalChoiceName {
            message: text("message"),
            name: text("name"),
            command: text("command"),
        },
    ]
}
