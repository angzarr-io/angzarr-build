//! What the descriptor set declares, as the emitters need it: components,
//! their commands and events, and every message and enum they reach. All
//! refusals happen here, so an emitter only writes.

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
/// `ComponentKind.COMPONENT_KIND_AGGREGATE`: the only kind emitted.
const AGGREGATE: i32 = 1;
/// The one well-known type the domain layer maps (to annals' `Timestamp`).
pub(crate) const TIMESTAMP: &str = "google.protobuf.Timestamp";
const WELL_KNOWN: &str = "google.protobuf";

/// One aggregate: its state message (the anchor) and what it handles.
pub(crate) struct Component {
    pub anchor: MessageDescriptor,
    /// The generated names' stem: `ComponentOptions.name`, else the anchor's.
    pub name: String,
    /// The stream domain it owns.
    pub domain: String,
    pub commands: Vec<MessageDescriptor>,
    pub events: Vec<MessageDescriptor>,
}

/// Everything generated into one package.
#[derive(Default)]
pub(crate) struct Package {
    pub components: Vec<Component>,
    /// Domain types, in the order they were reached.
    pub messages: Vec<MessageDescriptor>,
    pub enums: Vec<EnumDescriptor>,
}

/// Packages by name, each with a component.
pub(crate) fn build(
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<BTreeMap<String, Package>, CodegenError> {
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
        return Ok(BTreeMap::new());
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
        let kind = enum_field(&declared, "kind");
        if kind != AGGREGATE {
            return Err(CodegenError::UnsupportedKind {
                component: message.full_name().to_string(),
                kind,
            });
        }
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
            name,
            domain: string_field(&declared, "domain"),
            commands: Vec::new(),
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
            for emitted in list_field(&declared, "emits") {
                let Value::String(event) = emitted else {
                    continue;
                };
                if !component.events.iter().any(|e| e.full_name() == event) {
                    return Err(CodegenError::UnknownEmit {
                        command: message.full_name().to_string(),
                        event: event.clone(),
                    });
                }
            }
            component.commands.push(message.clone());
        }
    }

    let mut packages: BTreeMap<String, Package> = BTreeMap::new();
    for component in components {
        let mut reach = Reach::default();
        let roots = std::iter::once(&component.anchor)
            .chain(&component.commands)
            .chain(&component.events);
        for root in roots {
            reach.message(root)?;
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
