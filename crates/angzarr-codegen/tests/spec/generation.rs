//! Whens running the generator, and thens on what it made or why it
//! refused. angzarr-cli has no lint of the protos of its own: a lint
//! error is the generator refusing them (every check generation makes:
//! the descriptors, the domain layer, the scaffold and the adapters), its
//! exit status non-zero; it reports no warnings.

use angzarr_codegen::CodegenError;
use angzarr_diagnostic::Diagnostic as _;
use cucumber::{then, when};

use crate::protos::full;
use crate::world::Generator;

/// Every arg of `error`, as text.
fn args(error: &CodegenError) -> Vec<String> {
    error
        .args()
        .iter()
        .map(|(_, arg)| format!("{arg:?}"))
        .collect()
}

/// Whether `arg` names `wanted`: each of its dot-separated parts, in
/// order (`table.TableRefused.REASON_UNSPECIFIED` names the nested
/// `table.TableRefused.Reason.REASON_UNSPECIFIED`).
fn names_in(arg: &str, wanted: &str) -> bool {
    let mut parts = arg.split(['.', '"', '(', ')', ' ']);
    wanted.split('.').all(|w| parts.any(|p| p == w))
}

/// Whether `error` names `name` (a full name, or its last part).
fn names(error: &CodegenError, name: &str) -> bool {
    let full = full(name);
    args(error)
        .iter()
        .any(|a| names_in(a, &full) || names_in(a, name))
}

#[when(
    regex = r"^(?:the protos are linted|the component's descriptor is generated|the handler interfaces are generated|angzarr-cli scaffolds the component)$"
)]
fn generate(w: &mut Generator) {
    w.lint();
}

#[then("lint exits with a non-zero status")]
fn non_zero(w: &mut Generator) {
    let _ = w.refused();
}

#[then(regex = r#"^lint reports an error naming "([^"]+)"(.*)$"#)]
fn error_naming(w: &mut Generator, name: String, _rest: String) {
    let named = w.named(&name);
    let error = w.refused();
    assert!(
        names(error, &named) || names(error, &name),
        "{error:?} names no {name}"
    );
}

#[then(regex = r#"^lint reports no error for "([^"]+)"$"#)]
fn no_error(w: &mut Generator, _name: String) {
    let _ = w.generated();
}

#[then(
    regex = r#"^(?:the|the PM's) descriptor lists (?:starting type "([^"]+)"|"([^"]+)" among its starting types)$"#
)]
fn starting_type(w: &mut Generator, listed: String, among: String) {
    let wanted = full(if listed.is_empty() { &among } else { &listed });
    let descriptor = w.only_descriptor();
    assert!(
        descriptor.starting_types.contains(&wanted),
        "{:?}",
        descriptor.starting_types
    );
}

#[then("the descriptor lists no input domains")]
fn no_inputs(w: &mut Generator) {
    assert!(w.only_descriptor().inputs.is_empty());
}

#[then(regex = r#"^the descriptor carries rules version "([^"]+)"$"#)]
fn rules_version(w: &mut Generator, version: String) {
    assert_eq!(w.only_descriptor().rules_version, version);
}

#[then(regex = r#"^the descriptor carries the state type "([^"]+)"$"#)]
fn state_type(w: &mut Generator, state: String) {
    let descriptor = w.only_descriptor();
    assert_eq!(descriptor.state.map(|s| s.type_url), Some(state));
}

#[then(regex = r#"^the descriptor lists input domain "([\w-]+)" only$"#)]
fn input_only(w: &mut Generator, domain: String) {
    let inputs: Vec<String> = w
        .only_descriptor()
        .inputs
        .into_iter()
        .map(|t| t.domain)
        .collect();
    assert_eq!(inputs, [domain]);
}

#[then(regex = r#"^the descriptor carries output domain "([\w-]+)"$"#)]
fn output_domain(w: &mut Generator, domain: String) {
    assert_eq!(w.only_descriptor().output_domain, domain);
}

#[then("the descriptor carries no state")]
fn no_state(w: &mut Generator) {
    assert_eq!(w.only_descriptor().state, None);
}

#[then("the descriptor carries no output domain")]
fn no_output_domain(w: &mut Generator) {
    assert_eq!(w.only_descriptor().output_domain, "");
}

/// The adapter method that handles `input`, its markup through its return
/// type, as one line.
fn handler(w: &Generator, input: &str) -> String {
    let name = full(input);
    let bare = name.rsplit('.').next().unwrap_or_default();
    let adapters: String = w.generated().adapters.split_whitespace().collect();
    let at = ["handles_cmd", "handles_fact"]
        .iter()
        .find_map(|markup| adapters.find(&format!("#[{markup}({bare}")))
        .unwrap_or_else(|| panic!("no handler of {bare} in {adapters}"));
    let rest = &adapters[at..];
    rest[..rest.find('{').unwrap_or(rest.len())].to_string()
}

#[then(regex = r#"^its handler for "([^"]+)" is marked (#\[handles_cmd\]|#\[handles_fact\])$"#)]
fn marked(w: &mut Generator, input: String, markup: String) {
    // `#[handles_cmd]` as the adapter writes it, with its type: `#[handles_cmd(`.
    let opening = format!("{}(", markup.trim_end_matches(']'));
    let handler = handler(w, &input);
    assert!(handler.starts_with(&opening), "{handler}");
}

#[then(regex = r#"^that handler (may|may not) refuse$"#)]
fn may_refuse(w: &mut Generator, may: String) {
    let input = w.last_input.clone().expect("an input");
    let handler = handler(w, &input);
    assert_eq!(handler.contains("->Result<"), may == "may", "{handler}");
}
