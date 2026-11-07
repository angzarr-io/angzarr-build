//! Givens declaring the protos: components and their `ComponentOptions`,
//! and the `(command)`, `(fact)` and `(event)` entries naming them.

use cucumber::given;

use crate::protos::full;
use crate::world::Generator;

/// `domain`'s aggregate, `<Domain>State`.
fn state_of(domain: &str) -> String {
    let mut chars = domain.chars();
    let first = chars
        .next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_default();
    format!("{first}{}State", chars.as_str())
}

/// The scenarios' kinds of component, as they name them.
fn kind_of(words: &str) -> &'static str {
    match words {
        "an aggregate" => "AGGREGATE",
        "a process manager" => "PROCESS_MANAGER",
        "a policy" => "POLICY",
        "a projector" => "PROJECTOR",
        other => panic!("no kind {other}"),
    }
}

/// A stateful component `name` owning `domain`, and what the scenario says
/// of it after: an input, an event it folds, or that it names nothing.
#[given(
    regex = r#"^(an aggregate|a process manager) "([\w-]+)" owning domain "([\w-]+)"(?: with (?:(command|fact|message|input) )?"([^"]+)"| that folds "([^"]+)"| that takes no fact| that no \(command\), \(fact\) or \(event\) entry names)?$"#
)]
fn stateful(
    w: &mut Generator,
    words: String,
    name: String,
    domain: String,
    kind: String,
    input: String,
    folded: String,
) {
    w.component(&name, kind_of(&words), &domain).domain = Some(domain.clone());
    match kind.as_str() {
        "message" => {
            w.protos.message(&full(&input));
        }
        _ if !input.is_empty() => {
            let input = w.input(&input, &name);
            input.fact = kind == "fact";
        }
        _ => {}
    }
    if !folded.is_empty() {
        let component = w.named(&name);
        w.protos
            .message(&full(&folded))
            .events
            .push((component, None));
    }
}

#[given(
    regex = r#"^(an aggregate|a process manager)(?: component)? "(\w+)" owning domain "([\w-]+)" whose state message is "([^"]+)"$"#
)]
fn with_state(w: &mut Generator, words: String, name: String, domain: String, state: String) {
    w.declare(&name, &full(&state), kind_of(&words)).domain = Some(domain);
}

#[given(regex = r#"^(an aggregate|a process manager|a projector) component "([\w-]+)"$"#)]
fn component_of_kind(w: &mut Generator, words: String, name: String) {
    let kind = kind_of(&words);
    let component = w.component(&name, kind, &name);
    if kind != "PROJECTOR" {
        component.domain = Some(name);
    }
}

#[given(
    regex = r#"^an aggregate anchored on "([^"]+)" owning domain "([\w-]+)" whose component_name is (empty|"[^"]*")$"#
)]
fn anchored(w: &mut Generator, anchor: String, domain: String, name: String) {
    let component = w.declare(&anchor, &full(&anchor), "AGGREGATE");
    component.domain = Some(domain);
    component.component_name = Some(name.trim_matches('"').replace("empty", ""));
}

#[given(regex = r#"^aggregates owning domains "([\w-]+)" and "([\w-]+)"$"#)]
fn aggregates(w: &mut Generator, first: String, second: String) {
    for domain in [first, second] {
        let name = state_of(&domain);
        w.component(&name, "AGGREGATE", &domain).domain = Some(domain.clone());
    }
}

#[given(
    regex = r#"^aggregates "(\w+)" owning domain "([\w-]+)" and "(\w+)" owning domain "([\w-]+)"$"#
)]
fn two_aggregates(w: &mut Generator, first: String, a: String, second: String, b: String) {
    for (name, domain) in [(first, a), (second, b)] {
        w.component(&name, "AGGREGATE", &domain).domain = Some(domain.clone());
    }
}

/// A policy, in a package of its own: it owns no domain.
#[given(
    regex = r#"^a policy "([\w-]+)" with (?:output_domain "([\w-]+)"|no output_domain)(?: whose \(event\) entries name "([^"]+)")?$"#
)]
fn policy(w: &mut Generator, name: String, output: String, event: String) {
    w.component(&name, "POLICY", "policies").output_domain = Some(output).filter(|o| !o.is_empty());
    if !event.is_empty() {
        w.consumed(&event, &name);
    }
}

#[given(regex = r#"^a projector "([\w-]+)" whose \(event\) entries name "([^"]+)" and "([^"]+)"$"#)]
fn projector(w: &mut Generator, name: String, first: String, second: String) {
    w.component(&name, "PROJECTOR", "projectors");
    w.consumed(&first, &name);
    w.consumed(&second, &name);
}

/// The component declared by `name` has `declared` starting types.
#[given(regex = r#"^"([\w-]+)" declares starting_types "([^"]+)"$"#)]
fn starting_types(w: &mut Generator, name: String, listed: String) {
    let anchor = w.named(&name);
    let listed = full(&listed);
    w.protos
        .message(&anchor)
        .component
        .as_mut()
        .expect("a component")
        .starting_types
        .push(listed);
}

#[given(regex = r#"^"([\w-]+)" declares no starting_types$"#)]
fn no_starting_types(w: &mut Generator, name: String) {
    let anchor = w.named(&name);
    let component = w
        .protos
        .message(&anchor)
        .component
        .as_mut()
        .expect("a component");
    assert!(component.starting_types.is_empty());
}

#[given(regex = r#"^"([\w-]+)" takes the command "([^"]+)" and the fact "([^"]+)"$"#)]
fn takes(w: &mut Generator, name: String, command: String, fact: String) {
    w.input(&command, &name);
    w.input(&fact, &name).fact = true;
}

#[given(regex = r#"^the message "([^"]+)" carries \((command|fact)\) naming "(\w+)"$"#)]
fn carries(w: &mut Generator, message: String, option: String, component: String) {
    w.input(&message, &component).fact = option == "fact";
}

#[given(
    regex = r#"^an \(event\) entry for "([^"]+)" naming "([\w-]+)" with the domain override "([\w-]+)"$"#
)]
fn event_override(w: &mut Generator, event: String, component: String, domain: String) {
    let component = w.named(&component);
    w.protos
        .message(&full(&event))
        .events
        .push((component, Some(domain)));
}

#[given(regex = r#"^no \(event\) entry for "([^"]+)" names "(\w+)"$"#)]
fn no_event_entry(w: &mut Generator, event: String, component: String) {
    let component = w.named(&component);
    let message = w.protos.message(&full(&event));
    message.events.retain(|(named, _)| *named != component);
}

#[given(regex = r#"^no component in the descriptor set folds "([^"]+)"$"#)]
fn no_folder(w: &mut Generator, event: String) {
    let event = full(&event);
    let entries = w.protos.message(&event).events.clone();
    for (component, _) in entries {
        let kind = w
            .protos
            .messages
            .get(&component)
            .and_then(|m| m.component.as_ref())
            .map(|c| c.kind.clone());
        assert!(
            !matches!(kind.as_deref(), Some("AGGREGATE" | "PROCESS_MANAGER")),
            "{component} folds {event}"
        );
    }
}

#[given(
    regex = r#"^a command "([^"]+)" for "(\w+)" whose \(command\)\.(emits|refuses) lists "([^"]+)"$"#
)]
fn command_listing(
    w: &mut Generator,
    command: String,
    component: String,
    list: String,
    listed: String,
) {
    let input = w.input(&command, &component);
    match list.as_str() {
        "emits" => input.emits.push(full(&listed)),
        _ => input.refuses.push(full(&listed)),
    }
    // A refusal type is a message of its own, unless the scenario names
    // one that does not resolve (`wallet.NoSuchRefusal`).
    if list == "refuses" && !listed.contains("NoSuch") {
        w.protos.message(&full(&listed));
    }
}

#[given(regex = r#"^the business binary is built as version "([^"]+)"$"#)]
fn built_as(w: &mut Generator, version: String) {
    w.version = Some(version);
}
