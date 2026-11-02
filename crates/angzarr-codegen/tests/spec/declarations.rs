//! Givens declaring the protos: components and their `ComponentOptions`,
//! the `(command)` and `(event)` entries naming them, and refusals' codes
//! with their `(refusal_status)`.

use cucumber::given;

use crate::protos::{full, package};
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

#[given(
    regex = r#"^an aggregate "(\w+)" owning domain "([\w-]+)"(?: with (command|fact|message) "([^"]+)")?$"#
)]
fn aggregate(w: &mut Generator, name: String, domain: String, kind: String, input: String) {
    w.component(&name, "AGGREGATE", &domain).domain = Some(domain.clone());
    match kind.as_str() {
        "command" => {
            w.input(&input, &name);
        }
        "fact" => w.input(&input, &name).fact = true,
        "message" => {
            w.protos.message(&full(&input));
        }
        _ => {}
    }
}

#[given(regex = r#"^aggregates owning domains "([\w-]+)" and "([\w-]+)"$"#)]
fn aggregates(w: &mut Generator, first: String, second: String) {
    for domain in [first, second] {
        aggregate(w, state_of(&domain), domain, String::new(), String::new());
    }
}

#[given(
    regex = r#"^an? POLICY component "(\w+)" triggered by domain "([\w-]+)" with (?:output_domain "([\w-]+)"|no output_domain)$"#
)]
fn policy(w: &mut Generator, name: String, input: String, output: String) {
    let component = w.component(&name, "POLICY", &input);
    component.input_domain = Some(input);
    component.output_domain = (!output.is_empty()).then_some(output);
}

#[given(
    regex = r#"^a POLICY component "(\w+)" with input domain "([\w-]+)" and output domain "([\w-]+)"$"#
)]
fn policy_between(w: &mut Generator, name: String, input: String, output: String) {
    policy(w, name, input, output);
}

#[given(
    regex = r#"^a PROCESS_MANAGER component "(\w+)" owning domain "([\w-]+)"(?: with output_domain "([\w-]+)")?$"#
)]
fn process_manager(w: &mut Generator, name: String, domain: String, output: String) {
    let component = w.component(&name, "PROCESS_MANAGER", &domain);
    component.domain = Some(domain);
    component.output_domain = (!output.is_empty()).then_some(output);
}

#[given(regex = r#"^a process manager "(\w+)" owning domain "([\w-]+)" with input "([^"]+)"$"#)]
fn process_manager_with_input(w: &mut Generator, name: String, domain: String, input: String) {
    process_manager(w, name.clone(), domain, String::new());
    w.input(&input, &name);
}

#[given(regex = r#"^an \(event\) entry for "([^"]+)" naming "(\w+)" with domain "([\w-]+)"$"#)]
fn event_entry(w: &mut Generator, event: String, component: String, domain: String) {
    let component = w.named(&component);
    w.protos
        .message(&full(&event))
        .events
        .push((component, Some(domain)));
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

#[given(regex = r#"^a \(command\) entry for "([^"]+)" naming "(\w+)" that sets starts$"#)]
fn starting_entry(w: &mut Generator, command: String, component: String) {
    w.input(&command, &component).starts = true;
}

#[given(regex = r#"^its \(command\) entry for "([^"]+)" (sets starts|does not)$"#)]
fn its_entry(w: &mut Generator, command: String, sets: String) {
    let component = last_component(w);
    w.input(&command, &component).starts = sets == "sets starts";
}

/// The component declared last, by the name the scenario gave it.
fn last_component(w: &Generator) -> String {
    w.last_component.clone().expect("a component")
}

#[given(
    regex = r#"^the input "([^"]+)" for "(\w+)" has \(command\)\.fact (true|false)( and \(command\)\.starts true)?$"#
)]
fn input_fact(w: &mut Generator, input: String, component: String, fact: String, starts: String) {
    let input = w.input(&input, &component);
    input.fact = fact == "true";
    input.starts = !starts.is_empty();
}

#[given(regex = r#"^its \(command\)\.refuses lists "([^"]+)"$"#)]
fn its_refuses(w: &mut Generator, refusal: String) {
    let name = w.last_input.clone().expect("an input");
    if let Some(input) = w.protos.message(&name).input.as_mut() {
        input.refuses.push(full(&refusal));
    }
}

#[given(
    regex = r#"^the value (\w+) of enum "([^"]+)", no refusal's Reason is marked \(refusal_status\) (\w+)$"#
)]
fn marked_elsewhere(w: &mut Generator, value: String, enumeration: String, status: String) {
    w.protos
        .enums
        .insert(full(&enumeration), vec![(value, Some(status))]);
}

#[given(
    regex = r#"^the zero value REASON_UNSPECIFIED of "([^"]+)"'s Reason is marked \(refusal_status\) (\w+)$"#
)]
fn marked_zero(w: &mut Generator, refusal: String, status: String) {
    let reasons = &mut w.protos.message(&full(&refusal)).reasons;
    reasons.push(("REASON_UNSPECIFIED".to_string(), Some(status)));
    reasons.push(("REASON_TAKEN".to_string(), None));
}

#[given(regex = r#"^an "([\w-]+)" aggregate component whose state message is "([^"]+)"$"#)]
fn aggregate_with_state(w: &mut Generator, domain: String, state: String) {
    w.names.insert(domain.clone(), state.clone());
    let message = w.protos.message(&state);
    message.component = Some(crate::protos::Component {
        kind: "AGGREGATE".to_string(),
        domain: Some(domain),
        ..Default::default()
    });
}

#[given(regex = r#"^the business binary is built as version "([^"]+)"$"#)]
fn built_as(w: &mut Generator, version: String) {
    w.version = Some(version);
}

#[given(regex = r#"^a (AGGREGATE|PROCESS_MANAGER|PROJECTOR) component "(\w+)"$"#)]
fn component_of_kind(w: &mut Generator, kind: String, name: String) {
    let domain = package(&name.to_lowercase());
    let component = w.component(&name, &kind, &domain);
    match kind.as_str() {
        "PROJECTOR" => component.input_domain = Some("order".to_string()),
        _ => component.domain = Some(domain),
    }
}

#[given(regex = r#"^no \(event\) entry for "([^"]+)" names "(\w+)"$"#)]
fn no_event_entry(w: &mut Generator, event: String, component: String) {
    let component = w.named(&component);
    let message = w.protos.message(&full(&event));
    message.events.retain(|(named, _)| *named != component);
}

#[given(regex = r#"^no \(command\) entry naming "(\w+)" sets starts$"#)]
fn none_starts(w: &mut Generator, component: String) {
    let component = w.named(&component);
    for message in w.protos.messages.values_mut() {
        if let Some(input) = message.input.as_mut().filter(|i| i.component == component) {
            input.starts = false;
        }
    }
}

#[given(
    regex = r#"^a (AGGREGATE|PROCESS_MANAGER|POLICY|PROJECTOR) component "(\w+)" that no \(command\) or \(event\) entry names$"#
)]
fn idle(w: &mut Generator, kind: String, name: String) {
    match kind.as_str() {
        // A policy declares both its domains (C-0582).
        "POLICY" => policy(w, name, "order".to_string(), "inventory".to_string()),
        _ => component_of_kind(w, kind, name),
    }
}
