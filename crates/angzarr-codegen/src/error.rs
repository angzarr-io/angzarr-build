//! Why generation refused a descriptor set. Every refusal names the proto
//! element to change. Each error is a code and its args
//! ([`Diagnostic`]); its English `Display` is its en-US catalog message
//! filled with them ([`english`]), so the English is written once.

use std::fmt;

use angzarr_diagnostic::{Arg, Diagnostic};

/// Each named field as the arg of its own name, its text:
/// `texts![message, component]`. An arg is named as the field it carries.
macro_rules! texts {
    ($($field:ident),+ $(,)?) => {
        vec![$((stringify!($field), ::angzarr_diagnostic::Arg::from($field.as_str()))),+]
    };
}
pub(crate) use texts;

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

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        english(self, f)
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
        use CodegenError as E;
        match self {
            E::Descriptors { detail } | E::AppManifest { detail } => texts![detail],
            E::FileNotInSet { file } => texts![file],
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
            E::Oneof { message, oneof } => texts![message, oneof],
            E::NestedType { message, nested } => texts![message, nested],
            E::WellKnownType {
                message,
                field,
                type_name,
            } => texts![message, field, type_name],
            E::EnumZero { enum_name, value } => texts![enum_name, value],
            E::NoWirePath { package } => texts![package],
            E::TranslateUnknownPolicy { policy } => texts![policy],
            E::TranslateNotATargetCommand { policy, command } => texts![policy, command],
            E::TranslateNotASourceFact { policy, fact } | E::TranslateNotAFact { policy, fact } => {
                texts![policy, fact]
            }
            E::ScaffoldImplNotFound { path, component } => texts![path, component],
            E::RefusesOnFact { message } => texts![message],
            E::ReservedName { message, name } => texts![message, name],
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
            E::RefusalChoiceName {
                message,
                name,
                command,
            } => texts![message, name, command],
            E::InputMismatch {
                path,
                message,
                expected,
                found,
            } => texts![path, message, expected, found],
            E::PythonNoDomain { domain } => texts![domain],
            E::AppConfig { cause } => vec![("cause", Arg::nested(cause))],
            E::AppRefused { lints } => {
                vec![(
                    "count",
                    i64::try_from(lints.len()).unwrap_or(i64::MAX).into(),
                )]
            }
            E::AppDomainPath { path } => texts![path],
            E::RefusalStatusMisplaced { value } => texts![value],
        }
    }
}

/// One value of every error this crate defines, for the catalog test.
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
    protos_samples()
        .into_iter()
        .chain(use_samples())
        .map(|e| Box::new(e) as Box<dyn Diagnostic>)
        .chain(crate::app::app_lint_samples())
        .collect()
}

/// A text sample: the field's own name, so a message filling one arg's
/// place with another's reads wrong.
pub(crate) fn text(field: &str) -> String {
    field.to_string()
}

/// One value of every refusal of the protos themselves: a descriptor set
/// this generator cannot read or map, or markup breaking a rule of the
/// generated domain layer.
fn protos_samples() -> Vec<CodegenError> {
    use CodegenError as E;
    vec![
        E::Descriptors {
            detail: text("detail"),
        },
        E::FileNotInSet { file: text("file") },
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
        E::NoWirePath {
            package: text("package"),
        },
        E::RefusesOnFact {
            message: text("message"),
        },
        E::ReservedName {
            message: text("message"),
            name: text("name"),
        },
        E::ComponentName {
            component: text("component"),
            name: text("name"),
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
        E::CardinalityWithoutEmits {
            message: text("message"),
            optional: true,
        },
        E::EmitsTuple {
            message: text("message"),
            count: 5,
        },
        E::RefusalChoiceName {
            message: text("message"),
            name: text("name"),
            command: text("command"),
        },
        E::RefusalStatusMisplaced {
            value: text("value"),
        },
    ]
}

/// One value of every refusal of a use of the protos: a translate
/// selection, a user's file, the Python domain asked for, an
/// application's host config or manifest.
fn use_samples() -> Vec<CodegenError> {
    use CodegenError as E;
    vec![
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
        E::PythonNoDomain {
            domain: text("domain"),
        },
        E::AppConfig {
            cause: angzarr_diagnostic::Report::new("angzarr-yaml-load-failed-at")
                .arg("kind", "angzarr-host-config")
                .arg("source", "angzarr.yaml"),
        },
        E::AppRefused { lints: Vec::new() },
        E::AppManifest {
            detail: text("detail"),
        },
        E::AppDomainPath { path: text("path") },
    ]
}

/// `d` in English, its `Display`: its en-US message ([`crate::EN_US`]) with
/// each `{ $arg }` its arg's value, a nested cause's its own `Display`. The
/// English is written once, in the catalog; a code with no message there,
/// or one naming an arg `d` lacks, is shown as its code and args.
pub(crate) fn english(d: &dyn Diagnostic, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match filled(crate::EN_US, d) {
        Some(text) => f.write_str(&text),
        None => angzarr_diagnostic::display(d, f),
    }
}

/// `d`'s message in `catalog`, one `id = text` a line, filled with its args;
/// `None` when the catalog has no message of its code, or the message names
/// an arg it lacks or leaves a `{` unclosed.
fn filled(catalog: &str, d: &dyn Diagnostic) -> Option<String> {
    let code = d.code();
    let mut rest = catalog
        .lines()
        .find_map(|line| line.strip_prefix(code)?.strip_prefix(" = "))?;
    let args = d.args();
    let mut out = String::new();
    while let Some((before, after)) = rest.split_once("{ $") {
        let (name, after) = after.split_once(" }")?;
        let (_, arg) = args.iter().find(|(n, _)| *n == name)?;
        out.push_str(before);
        match arg {
            Arg::Int(n) => out.push_str(&n.to_string()),
            Arg::Uint(n) => out.push_str(&n.to_string()),
            Arg::Str(s) => out.push_str(s),
            Arg::Nested(cause) => out.push_str(&cause.to_string()),
        }
        rest = after;
    }
    out.push_str(rest);
    Some(out)
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use angzarr_diagnostic::{Arg, Diagnostic, Report};

    use super::{english, filled};

    /// A report shown through [`english`].
    #[derive(Debug)]
    struct Shown(Report);

    impl fmt::Display for Shown {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            english(self, f)
        }
    }

    impl Diagnostic for Shown {
        fn code(&self) -> &str {
            self.0.code()
        }

        fn args(&self) -> Vec<(&str, Arg)> {
            self.0.args()
        }
    }

    const CATALOG: &str = "t-other = other\n\
                           t-shown-more = not this one\n\
                           t-shown = { $b } before { $a }: { $n }, { $u } and { $cause }.\n\
                           t-absent = { $missing }\n\
                           t-open = { $a\n";

    #[test]
    fn a_message_is_filled_by_arg_name_each_kind_of_arg_its_own_way() {
        let cause = Report::new("t-cause").arg("k", "v w");
        let shown = Report::new("t-shown")
            .arg("a", "first")
            .arg("b", "second")
            .arg("n", -3i64)
            .arg("u", 7u64)
            .arg("cause", Arg::Nested(Box::new(cause)));
        assert_eq!(
            filled(CATALOG, &shown).as_deref(),
            Some("second before first: -3, 7 and t-cause k=\"v w\".")
        );
    }

    #[test]
    fn no_message_of_the_code_an_absent_arg_or_an_open_place_fills_nothing() {
        let a = |code: &'static str| Report::new(code).arg("a", "x");
        assert_eq!(filled(CATALOG, &a("t-none")), None);
        assert_eq!(filled(CATALOG, &a("t-absent")), None);
        assert_eq!(filled(CATALOG, &a("t-open")), None);
        assert_eq!(filled(CATALOG, &a("t-other")).as_deref(), Some("other"));
    }

    #[test]
    fn english_with_no_message_is_the_code_and_args() {
        let shown = Shown(Report::new("angzarr-codegen-none").arg("a", "x y"));
        assert_eq!(shown.to_string(), "angzarr-codegen-none a=\"x y\"");
        let known = Shown(Report::new("angzarr-codegen-file-not-in-set").arg("file", "a.proto"));
        assert_eq!(known.to_string(), "a.proto is not in the descriptor set");
    }
}
