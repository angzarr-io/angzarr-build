//! What the descriptor set declares, as the emitters need it: components,
//! their inputs (commands and facts) and events, and every message and enum
//! they reach. Every refusal of the protos happens here, so an emitter only
//! writes; the scaffold's own checks (translate selections, the user's
//! markup) are the scaffold's.

use std::collections::{BTreeMap, BTreeSet};

use prost_reflect::{
    DescriptorPool, DynamicMessage, EnumDescriptor, ExtensionDescriptor, Kind, MessageDescriptor,
    Value,
};

use crate::error::CodegenError;

const COMPONENT: &str = "io.angzarr.v1.component";
const COMMAND: &str = "io.angzarr.v1.command";
const EVENT: &str = "io.angzarr.v1.event";
const STATE: &str = "io.angzarr.v1.state";
const RESOLUTION: &str = "io.angzarr.v1.ConflictResolution";
const COMMUTATIVE: &str = "CONFLICT_RESOLUTION_COMMUTATIVE";
/// The one well-known type the domain layer maps (to angzarr-embedded's `Timestamp`).
pub(crate) const TIMESTAMP: &str = "google.protobuf.Timestamp";
const WELL_KNOWN: &str = "google.protobuf";

/// `ComponentKind`'s numbers (types.proto).
const AGGREGATE: i32 = 1;
const POLICY: i32 = 2;
const PROCESS_MANAGER: i32 = 3;
const PROJECTOR: i32 = 4;

/// A component kind a generator emits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ComponentKind {
    Aggregate,
    Policy,
    ProcessManager,
}

impl ComponentKind {
    fn of(number: i32) -> Option<ComponentKind> {
        match number {
            AGGREGATE => Some(ComponentKind::Aggregate),
            POLICY => Some(ComponentKind::Policy),
            PROCESS_MANAGER => Some(ComponentKind::ProcessManager),
            _ => None,
        }
    }
}

/// The kinds the domain layer emits.
pub(crate) const DOMAIN_KINDS: &[ComponentKind] = &[ComponentKind::Aggregate];
/// The kinds the markup (scaffold, register, markup domain layer) emits.
pub(crate) const MARKUP_KINDS: &[ComponentKind] = &[
    ComponentKind::Aggregate,
    ComponentKind::Policy,
    ComponentKind::ProcessManager,
];

/// One input of an aggregate or a process manager: a command, or a fact
/// (`CommandOptions.fact`).
pub(crate) struct Input {
    pub message: MessageDescriptor,
    pub emits: Vec<MessageDescriptor>,
    /// Its typed refusals; always empty for a fact.
    pub refuses: Vec<MessageDescriptor>,
    pub starts: bool,
    pub fact: bool,
}

/// One component: its anchor message and what it handles.
pub(crate) struct Component {
    pub anchor: MessageDescriptor,
    pub kind: ComponentKind,
    /// The generated names' stem: `ComponentOptions.name`, else the anchor's.
    pub name: String,
    /// The stream domain it owns (an aggregate's or a process manager's).
    pub domain: String,
    /// A policy's source domain.
    pub input_domain: String,
    /// A policy's target domain.
    pub output_domain: String,
    pub inputs: Vec<Input>,
    /// An aggregate's or a process manager's events (folded), or the events
    /// a policy reacts to.
    pub events: Vec<MessageDescriptor>,
}

impl Component {
    /// The input whose message is `full_name`.
    pub fn input(&self, full_name: &str) -> Option<&Input> {
        self.inputs
            .iter()
            .find(|i| i.message.full_name() == full_name)
    }
}

/// Everything generated into one package.
#[derive(Default)]
pub(crate) struct Package {
    pub components: Vec<Component>,
    /// Domain types, in the order they were reached.
    pub messages: Vec<MessageDescriptor>,
    pub enums: Vec<EnumDescriptor>,
}

/// The domain layer's packages by name, each with a component: its
/// aggregates, every type their state, commands and events reach.
pub(crate) fn build(
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<BTreeMap<String, Package>, CodegenError> {
    let components = components(descriptor_set, files, DOMAIN_KINDS)?;
    packages(components, |c| {
        std::iter::once(&c.anchor)
            .chain(c.inputs.iter().map(|i| &i.message))
            .chain(&c.events)
            .cloned()
            .collect()
    })
}

/// The markup domain layer's packages by name: every type the inputs,
/// events and refusals of each aggregate and process manager reach. The
/// state is the markup's, and a policy holds no domain type.
pub(crate) fn build_markup(
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<BTreeMap<String, Package>, CodegenError> {
    let components = components(descriptor_set, files, MARKUP_KINDS)?
        .into_iter()
        .filter(|c| c.kind != ComponentKind::Policy)
        .collect();
    packages(components, |c| {
        c.inputs
            .iter()
            .flat_map(|i| std::iter::once(&i.message).chain(&i.refuses))
            .chain(&c.events)
            .cloned()
            .collect()
    })
}

/// `components` grouped by their anchor's package, with every type `roots`
/// reaches.
fn packages(
    components: Vec<Component>,
    roots: impl Fn(&Component) -> Vec<MessageDescriptor>,
) -> Result<BTreeMap<String, Package>, CodegenError> {
    let mut packages: BTreeMap<String, Package> = BTreeMap::new();
    for component in components {
        let mut reach = Reach::default();
        for root in roots(&component) {
            reach.message(&root)?;
        }
        let package = packages
            .entry(component.anchor.package_name().to_string())
            .or_default();
        for message in reach.messages {
            if !package.messages.contains(&message) {
                package.messages.push(message);
            }
        }
        for e in reach.enums {
            if !package.enums.contains(&e) {
                package.enums.push(e);
            }
        }
        package.components.push(component);
    }
    Ok(packages)
}

/// Every component declared in `files`, in declaration order, refusing a
/// kind outside `kinds`.
pub(crate) fn components(
    descriptor_set: &[u8],
    files: &[&str],
    kinds: &[ComponentKind],
) -> Result<Vec<Component>, CodegenError> {
    let pool = DescriptorPool::decode(descriptor_set).map_err(|e| CodegenError::Descriptors {
        detail: e.to_string(),
    })?;
    let mut messages = Vec::new();
    for file in files {
        let found = pool
            .get_file_by_name(file)
            .ok_or_else(|| CodegenError::FileNotInSet {
                file: file.to_string(),
            })?;
        messages.extend(found.messages());
    }
    // A set that never imported options.proto declares no component.
    let Some(component_ext) = pool.get_extension_by_name(COMPONENT) else {
        return Ok(Vec::new());
    };
    let command_ext = pool.get_extension_by_name(COMMAND);
    let event_ext = pool.get_extension_by_name(EVENT);
    let commutative = pool
        .get_enum_by_name(RESOLUTION)
        .and_then(|e| e.get_value_by_name(COMMUTATIVE))
        .map(|v| v.number());

    let mut components: Vec<Component> = Vec::new();
    for message in &messages {
        let options = message.options();
        if !options.has_extension(&component_ext) {
            continue;
        }
        let Some(declared) = message_value(&options, &component_ext) else {
            continue;
        };
        let number = enum_field(&declared, "kind");
        // The markup is the write side: a projector beside it is skipped,
        // not refused.
        if number == PROJECTOR && kinds.contains(&ComponentKind::Policy) {
            continue;
        }
        let Some(kind) = ComponentKind::of(number).filter(|k| kinds.contains(k)) else {
            return Err(CodegenError::UnsupportedKind {
                component: message.full_name().to_string(),
                kind: number,
            });
        };
        if let (Some(state_ext), Some(commutative)) =
            (pool.get_extension_by_name(STATE), commutative)
            && let Some(state) = message_value(&options, &state_ext)
            && list_field(&state, "conflict_resolutions")
                .iter()
                .any(|v| matches!(v, Value::EnumNumber(n) if *n == commutative))
        {
            return Err(CodegenError::Commutative {
                component: message.full_name().to_string(),
            });
        }
        let name = match string_field(&declared, "name") {
            n if n.is_empty() => message.name().to_string(),
            n => n,
        };
        components.push(Component {
            anchor: message.clone(),
            kind,
            name,
            domain: string_field(&declared, "domain"),
            input_domain: string_field(&declared, "input_domain"),
            output_domain: string_field(&declared, "output_domain"),
            inputs: Vec::new(),
            events: Vec::new(),
        });
    }

    // Events first, so a command's `emits` can be checked against them.
    if let Some(event_ext) = &event_ext {
        for message in &messages {
            let options = message.options();
            if !options.has_extension(event_ext) {
                continue;
            }
            for consumer in options.get_extension(event_ext).as_list().unwrap_or(&[]) {
                let Value::Message(consumer) = consumer else {
                    continue;
                };
                let named = string_field(consumer, "component");
                let component = owner(&mut components, &pool, &component_ext, message, &named)?;
                if let Some(component) = component
                    && !component.events.contains(message)
                {
                    component.events.push(message.clone());
                }
            }
        }
    }
    if let Some(command_ext) = &command_ext {
        for message in &messages {
            let options = message.options();
            if !options.has_extension(command_ext) {
                continue;
            }
            let Some(declared) = message_value(&options, command_ext) else {
                continue;
            };
            let named = string_field(&declared, "component");
            let Some(component) = owner(&mut components, &pool, &component_ext, message, &named)?
            else {
                continue;
            };
            let mut emits = Vec::new();
            for emitted in list_field(&declared, "emits") {
                let Value::String(event) = emitted else {
                    continue;
                };
                let Some(event) = component.events.iter().find(|e| e.full_name() == event) else {
                    return Err(CodegenError::UnknownEmit {
                        command: message.full_name().to_string(),
                        event: event.clone(),
                    });
                };
                emits.push(event.clone());
            }
            let fact = bool_field(&declared, "fact");
            let refused = list_field(&declared, "refuses");
            if fact && !refused.is_empty() {
                return Err(CodegenError::RefusesOnFact {
                    message: message.full_name().to_string(),
                });
            }
            let mut refuses = Vec::new();
            for refusal in refused {
                let Value::String(refusal) = refusal else {
                    continue;
                };
                let found = pool.get_message_by_name(&refusal).ok_or_else(|| {
                    CodegenError::UnknownRefusal {
                        command: message.full_name().to_string(),
                        refusal: refusal.clone(),
                    }
                })?;
                refuses.push(found);
            }
            component.inputs.push(Input {
                message: message.clone(),
                emits,
                refuses,
                starts: bool_field(&declared, "starts"),
                fact,
            });
        }
    }
    Ok(components)
}

/// The component a `(command)` or `(event)` names: `Some` when it is
/// generated here, `None` when it is declared but outside `files`, and a
/// refusal when nothing declares it.
fn owner<'a>(
    components: &'a mut [Component],
    pool: &DescriptorPool,
    component_ext: &ExtensionDescriptor,
    message: &MessageDescriptor,
    named: &str,
) -> Result<Option<&'a mut Component>, CodegenError> {
    if let Some(i) = components
        .iter()
        .position(|c| c.anchor.full_name() == named)
    {
        return Ok(components.get_mut(i));
    }
    match pool.get_message_by_name(named) {
        Some(m) if m.options().has_extension(component_ext) => Ok(None),
        _ => Err(CodegenError::UnknownComponent {
            message: message.full_name().to_string(),
            component: named.to_string(),
        }),
    }
}

/// The messages and enums reachable from a component's roots.
#[derive(Default)]
struct Reach {
    seen: BTreeSet<String>,
    messages: Vec<MessageDescriptor>,
    enums: Vec<EnumDescriptor>,
}

impl Reach {
    fn message(&mut self, message: &MessageDescriptor) -> Result<(), CodegenError> {
        if !self.seen.insert(message.full_name().to_string()) {
            return Ok(());
        }
        let nested = message
            .child_messages()
            .filter(|m| !m.is_map_entry())
            .map(|m| m.full_name().to_string())
            .chain(message.child_enums().map(|e| e.full_name().to_string()))
            .next();
        if let Some(nested) = nested {
            return Err(CodegenError::NestedType {
                message: message.full_name().to_string(),
                nested,
            });
        }
        if let Some(oneof) = message.oneofs().find(|o| !o.is_synthetic()) {
            return Err(CodegenError::Oneof {
                message: message.full_name().to_string(),
                oneof: oneof.name().to_string(),
            });
        }
        self.messages.push(message.clone());
        for field in message.fields() {
            let kind = match field.kind() {
                Kind::Message(entry) if entry.is_map_entry() => {
                    entry.map_entry_value_field().kind()
                }
                kind => kind,
            };
            match kind {
                Kind::Message(m) if m.full_name() == TIMESTAMP => {}
                Kind::Message(m) if m.package_name() == WELL_KNOWN => {
                    return Err(CodegenError::WellKnownType {
                        message: message.full_name().to_string(),
                        field: field.name().to_string(),
                        type_name: m.full_name().to_string(),
                    });
                }
                Kind::Message(m) => self.message(&m)?,
                Kind::Enum(e) => self.enumeration(&e)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn enumeration(&mut self, e: &EnumDescriptor) -> Result<(), CodegenError> {
        if !self.seen.insert(e.full_name().to_string()) {
            return Ok(());
        }
        if let Some(zero) = e.values().find(|v| v.number() == 0)
            && !zero.name().ends_with("_UNSPECIFIED")
        {
            return Err(CodegenError::EnumZero {
                enum_name: e.full_name().to_string(),
                value: zero.name().to_string(),
            });
        }
        self.enums.push(e.clone());
        Ok(())
    }
}

/// A message-typed option's value, as a dynamic message.
fn message_value(options: &DynamicMessage, ext: &ExtensionDescriptor) -> Option<DynamicMessage> {
    match options.get_extension(ext).into_owned() {
        Value::Message(m) => Some(m),
        _ => None,
    }
}

fn string_field(message: &DynamicMessage, name: &str) -> String {
    match message.get_field_by_name(name).as_deref() {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn bool_field(message: &DynamicMessage, name: &str) -> bool {
    matches!(
        message.get_field_by_name(name).as_deref(),
        Some(Value::Bool(true))
    )
}

fn enum_field(message: &DynamicMessage, name: &str) -> i32 {
    match message.get_field_by_name(name).as_deref() {
        Some(Value::EnumNumber(n)) => *n,
        _ => 0,
    }
}

fn list_field(message: &DynamicMessage, name: &str) -> Vec<Value> {
    match message.get_field_by_name(name).as_deref() {
        Some(Value::List(values)) => values.clone(),
        _ => Vec::new(),
    }
}
