//! Whens running the generator, and thens on what it made or why it
//! refused. "The protos are linted" is `angzarr lint app`'s reading of
//! them (`angzarr_codegen::warnings`): a declaration the contract forbids
//! refused, its exit status non-zero, and its warnings, which leave the
//! status 0; the tier's warning labels (`ANZ100`) are the crate's codes
//! ([`coded`]). Generating runs every check generation makes: the
//! descriptors, the domain layer, the scaffold and the adapters.

use angzarr_codegen::{CodegenError, Warning};
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

#[when("the protos are linted")]
fn linted(w: &mut Generator) {
    w.lint_protos();
}

#[when(
    regex = r"^(?:the component's descriptor is generated|the handler interfaces are generated|angzarr-cli scaffolds the component)$"
)]
fn generate(w: &mut Generator) {
    w.lint();
}

#[then("lint exits with status 0")]
fn zero(w: &mut Generator) {
    let _ = w.generated();
}

/// Whether `warning` names `name` (a full name, or its last part).
fn warns_of(warning: &angzarr_codegen::Warning, name: &str) -> bool {
    let full = full(name);
    warning
        .args()
        .iter()
        .map(|(_, arg)| format!("{arg:?}"))
        .any(|a| names_in(&a, &full) || names_in(&a, name))
}

/// The cli's warning for the condition the tier labels `label` (`ANZ100`),
/// by its variant.
fn coded(label: &str) -> fn(&Warning) -> bool {
    match label {
        "ANZ100" => |w| matches!(w, Warning::UnappliedEmit { .. }),
        "ANZ101" => |w| matches!(w, Warning::UnownedTarget { .. }),
        "ANZ102" => |w| matches!(w, Warning::UnproducedSource { .. }),
        "ANZ103" => |w| matches!(w, Warning::Idle { .. }),
        other => panic!("no cli code for the tier's {other}"),
    }
}

#[then(
    regex = r#"^lint reports (?:warning (ANZ\d+)|a warning) naming (?:command )?"([^"]+)"(.*)$"#
)]
fn warning_naming(w: &mut Generator, label: String, name: String, _rest: String) {
    let named = w.named(&name);
    let code = (!label.is_empty()).then(|| coded(&label));
    let found = w.warnings.iter().any(|warning| {
        code.is_none_or(|is| is(warning)) && (warns_of(warning, &named) || warns_of(warning, &name))
    });
    assert!(found, "no warning {label} naming {name}: {:?}", w.warnings);
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

/// `text` with its whitespace removed: generated code read as one line.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect()
}

/// The text of the Python file of `files` whose path ends with `suffix`.
fn python_file<'a>(files: &'a [(String, String)], suffix: &str) -> &'a str {
    files
        .iter()
        .find(|(path, _)| path.ends_with(suffix))
        .map(|(_, text)| text.as_str())
        .unwrap_or_else(|| panic!("no {suffix} in {:?}", files.iter().map(|f| &f.0)))
}

/// The entries of the Python table `name` (`_COMMANDS = { .. }`), squeezed.
fn python_table(text: &str, name: &str) -> String {
    let text = squeezed(text);
    let open = format!("{name}={{");
    let at = text
        .find(&open)
        .unwrap_or_else(|| panic!("no table {name} in {text}"));
    let rest = &text[at + open.len()..];
    rest[..rest.find('}').unwrap_or(rest.len())].to_string()
}

#[then(regex = r#"^the reaction of "(\w+)" to "([^"]+)" may return "([^"]+)"$"#)]
fn reaction_returns(w: &mut Generator, policy: String, event: String, output: String) {
    let bare = full(&output)
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_string();
    // Rust: the adapter's reaction takes it from what the interface returns.
    let adapters = squeezed(&w.generated().adapters);
    assert!(
        adapters.contains(&format!("Output::{bare}(output)=>")),
        "the Rust reaction takes no {bare}: {adapters}"
    );
    // Python: the logic's reaction is typed to return it.
    let files = w.python(&policy);
    let logic = squeezed(python_file(&files, "/logic.py"));
    let event = full(&event)
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_string();
    let reaction = format!("def{}(", snake(&event));
    let at = logic
        .find(&reaction)
        .unwrap_or_else(|| panic!("no reaction {reaction} in {logic}"));
    // The interface's method, through its ` ...` body.
    let rest = &logic[at..];
    let method = &rest[..rest.find("...").unwrap_or(rest.len())];
    let returns = &method[method.find("->").unwrap_or(method.len())..];
    assert!(
        returns.contains(&bare),
        "the Python reaction returns no {bare}: {method}"
    );
}

#[then(regex = r#"^the generated code of "(\w+)" sends "([^"]+)" as a (Command|Fact)$"#)]
fn sends_as(w: &mut Generator, policy: String, output: String, arm: String) {
    let bare = full(&output)
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_string();
    // Rust: the arm the adapter's reaction builds it in.
    let adapters = squeezed(&w.generated().adapters);
    let call = if arm == "Fact" { "fact" } else { "to" };
    // prettyplease may brace the arm's body.
    let sent = format!("commands.{call}(root,output)");
    assert!(
        adapters.contains(&format!("Output::{bare}(output)=>{sent}"))
            || adapters.contains(&format!("Output::{bare}(output)=>{{{sent}}}")),
        "the Rust reaction does not send {bare} as a {arm}: {adapters}"
    );
    // Python: the table the dispatch sends it from.
    let files = w.python(&policy);
    let dispatch = python_file(&files, &format!("{}_angzarr.py", snake(&policy)));
    let entry = format!("_d.{bare}:");
    let (from, other) = if arm == "Fact" {
        ("_SENT_FACTS", "_COMMANDS")
    } else {
        ("_COMMANDS", "_SENT_FACTS")
    };
    assert!(
        python_table(dispatch, from).contains(&entry),
        "the Python dispatch does not send {bare} as a {arm}: {dispatch}"
    );
    assert!(
        !python_table(dispatch, other).contains(&entry),
        "the Python dispatch sends {bare} in both arms: {dispatch}"
    );
}

#[then(regex = r#"^"(\w+)" (is given|is not given) a handler of the refusal of "([^"]+)"$"#)]
fn refusal_handler(w: &mut Generator, domain: String, given: String, command: String) {
    // The Python scaffold's: its logic's on_refused_<command>, and the
    // dispatch's registration of it. Rust's generated domain layer gives no
    // domain a refusal handler.
    let name = full(&command);
    let bare = name.rsplit('.').next().unwrap_or_default().to_string();
    let files = w.python(&domain);
    let logic = python_file(&files, "/logic.py");
    let method = format!("def on_refused_{}(", snake(&bare));
    let dispatch = python_file(&files, &format!("{}_angzarr.py", snake(&domain)));
    let registered = squeezed(dispatch).contains(&format!("dispatch.on_refused(\"{name}\""));
    let expected = given == "is given";
    assert_eq!(logic.contains(&method), expected, "{method} in {logic}");
    assert_eq!(
        registered, expected,
        "the refusal of {name} registered in {dispatch}"
    );
}

/// `BuyInWallet` as `buy_in_wallet`, as the generator names a method or a
/// module.
fn snake(camel: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = camel.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let before = chars[i - 1];
            let after = chars.get(i + 1);
            if before.is_ascii_lowercase()
                || before.is_ascii_digit()
                || (before.is_ascii_uppercase() && after.is_some_and(char::is_ascii_lowercase))
            {
                out.push('_');
            }
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// The Rust adapter's handler of `input` and what follows it to the next
/// handler, squeezed: its markup, signature and body.
fn handler_body(w: &Generator, input: &str) -> String {
    let name = full(input);
    let bare = name.rsplit('.').next().unwrap_or_default();
    let adapters = squeezed(&w.generated().adapters);
    let at = ["handles_cmd", "handles_fact"]
        .iter()
        .find_map(|markup| adapters.find(&format!("#[{markup}({bare}")))
        .unwrap_or_else(|| panic!("no handler of {bare} in {adapters}"));
    let rest = &adapters[at + 1..];
    rest[..rest.find("#[").unwrap_or(rest.len())].to_string()
}

#[then(regex = r#"^the (\w+) handler returns a list of the component's events$"#)]
fn returns_a_list(w: &mut Generator, message: String) {
    let input = w.last_input.clone().expect("an input");
    assert!(input.ends_with(&message), "{input}");
    let component = w.last_component.clone().expect("a component");
    let bare = component.rsplit('.').next().unwrap_or_default().to_string();
    // Rust: the interface's list, each event a step of the markup.
    let body = handler_body(w, &input);
    assert!(
        body.contains(&format!("|events:Vec<{bare}Event>|")),
        "{body}"
    );
    // Python: the logic's signature returns a list.
    let files = w.python(&bare);
    let logic = squeezed(python_file(&files, "/logic.py"));
    let method = format!("def{}(", snake(&message));
    let at = logic
        .find(&method)
        .unwrap_or_else(|| panic!("no {method} in {logic}"));
    let signature = &logic[at..at + logic[at..].find(":...").unwrap_or(0)];
    assert!(signature.contains(")->list["), "{signature}");
}

#[then("it may return an empty list")]
fn may_be_empty(w: &mut Generator) {
    let input = w.last_input.clone().expect("an input");
    // An empty list folds to no event: the markup's `of(None)`.
    let body = handler_body(w, &input);
    assert!(
        body.contains("Emit::of(None::<") || body.contains("Record::of(None::<"),
        "{body}"
    );
}
