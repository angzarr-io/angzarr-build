//! What the descriptor set declares, as the emitters need it: components,
//! their inputs (commands and facts) and events, and every message and enum
//! they reach. Every refusal of the protos happens here, so an emitter only
//! writes; the scaffold's own checks (translate selections, the user's
//! markup) are the scaffold's.

use std::collections::{BTreeMap, BTreeSet};

use prost_reflect::{
    DescriptorPool, DynamicMessage, EnumDescriptor, EnumValueDescriptor, ExtensionDescriptor, Kind,
    MessageDescriptor, Value,
};

use crate::error::CodegenError;

const COMPONENT: &str = "io.angzarr.v1.component";
const COMMAND: &str = "io.angzarr.v1.command";
const EVENT: &str = "io.angzarr.v1.event";
const STATE: &str = "io.angzarr.v1.state";
const RESOLUTION: &str = "io.angzarr.v1.ConflictResolution";
const REFUSAL_STATUS: &str = "io.angzarr.v1.refusal_status";
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
    /// Only the descriptor's: the markup is the write side.
    Projector,
}

impl ComponentKind {
    fn of(number: i32) -> Option<ComponentKind> {
        match number {
            AGGREGATE => Some(ComponentKind::Aggregate),
            POLICY => Some(ComponentKind::Policy),
            PROCESS_MANAGER => Some(ComponentKind::ProcessManager),
            PROJECTOR => Some(ComponentKind::Projector),
            _ => None,
        }
    }

    /// The kind its name's default reads.
    pub(crate) fn name_kind(self) -> crate::names::NameKind {
        use crate::names::NameKind;
        match self {
            ComponentKind::Aggregate => NameKind::Aggregate,
            ComponentKind::Policy => NameKind::Policy,
            ComponentKind::ProcessManager => NameKind::ProcessManager,
            ComponentKind::Projector => NameKind::Projector,
        }
    }

    /// Its `ComponentKind` number (types.proto).
    pub(crate) fn number(self) -> i32 {
        match self {
            ComponentKind::Aggregate => AGGREGATE,
            ComponentKind::Policy => POLICY,
            ComponentKind::ProcessManager => PROCESS_MANAGER,
            ComponentKind::Projector => PROJECTOR,
        }
    }
}

/// The write side's kinds: what owns a policy's domains.
pub(crate) const WRITE_KINDS: &[ComponentKind] = &[
    ComponentKind::Aggregate,
    ComponentKind::Policy,
    ComponentKind::ProcessManager,
];
/// Every kind: what the Rust markup (scaffold, adapters, register, domain
/// layer), the Python scaffold and the descriptors are generated for.
pub(crate) const ALL_KINDS: &[ComponentKind] = &[
    ComponentKind::Aggregate,
    ComponentKind::Policy,
    ComponentKind::ProcessManager,
    ComponentKind::Projector,
];

/// How many events one handling of an input emits
/// (`CommandOptions.emits_cardinality`, decision 14.50).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Cardinality {
    /// One event of each emits type, in order: the one event, or a tuple.
    One,
    /// That, or nothing.
    Optional,
    /// Any number of events of the emits types, in any order.
    Many,
}

/// `EmitsCardinality`'s numbers (options.proto).
const CARDINALITY_ONE: i32 = 1;
const CARDINALITY_OPTIONAL: i32 = 2;
const CARDINALITY_MANY: i32 = 3;

/// The most emits a ONE or OPTIONAL input may list: the markup's widest
/// tuple.
pub(crate) const MAX_TUPLE: usize = 4;

/// One input of an aggregate or a process manager: a command, or a fact
/// (`CommandOptions.fact`).
pub(crate) struct Input {
    pub message: MessageDescriptor,
    pub emits: Vec<MessageDescriptor>,
    pub cardinality: Cardinality,
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
    /// Its name in an application (`ComponentOptions.component_name`, else
    /// its kind's default: [`crate::component_name`]).
    pub component_name: String,
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
    /// The source domains its `(event)` entries name, each once, in order;
    /// a projector's are its `input_domains` (or its one `input_domain`)
    /// when it lists them: its subscriptions.
    pub sources: Vec<String>,
    /// The conflict resolutions its state's `(state)` option lists.
    pub resolutions: Vec<i32>,
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

/// The markup domain layer's packages by name: every type the inputs,
/// events and refusals of each aggregate and process manager reach, and the
/// events each projector projects. The state is the implementation's (its
/// wire type), and a policy holds no domain type.
pub(crate) fn build_markup(
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<BTreeMap<String, Package>, CodegenError> {
    let components = components(descriptor_set, files, ALL_KINDS)?
        .into_iter()
        .filter(|c| c.kind != ComponentKind::Policy)
        .collect();
    let packages = packages(components, |c| {
        c.inputs
            .iter()
            .flat_map(|i| std::iter::once(&i.message).chain(&i.refuses))
            .chain(&c.events)
            .cloned()
            .collect()
    })?;
    for package in packages.values() {
        let names = package
            .messages
            .iter()
            .map(|m| (m.full_name(), m.name().to_string()))
            .chain(
                package
                    .enums
                    .iter()
                    .map(|e| (e.full_name(), crate::common::enum_name(e))),
            );
        refuse_reserved(names.clone())?;
        // A command refusing several types refuses with a choice the
        // domain layer names after it: no domain type may take that name.
        for component in &package.components {
            for input in component.inputs.iter().filter(|i| i.refuses.len() > 1) {
                let choice = crate::common::refusal_choice_name(&input.message);
                if let Some((full, _)) = names.clone().find(|(_, local)| *local == choice) {
                    return Err(CodegenError::RefusalChoiceName {
                        message: full.to_string(),
                        name: choice,
                        command: input.message.full_name().to_string(),
                    });
                }
            }
        }
    }
    Ok(packages)
}

/// The names the generated domain layers give the runtime's types, which no
/// domain type may take: a handler's `Context`, a policy reaction's
/// `PolicyContext`, a refused command's `CommandContext`, a projector's
/// `ProjectionContext`, the two classes
/// a handler fails with, the business `Rejection` and the defect `Bug`, and
/// Rust's `Failed` of either, with its `BusinessStatus` and its
/// `RetryableStatus`.
pub(crate) const RESERVED: &[&str] = &[
    "Context",
    "PolicyContext",
    "CommandContext",
    "ProjectionContext",
    "Rejection",
    "Bug",
    "Failed",
    "BusinessStatus",
    "RetryableStatus",
];

/// The first of `names` (each a type's full name and the name the domain
/// layer gives it) that is [`RESERVED`], refused: the generated code would
/// collide, and nothing is renamed silently.
pub(crate) fn refuse_reserved<'a>(
    names: impl IntoIterator<Item = (&'a str, String)>,
) -> Result<(), CodegenError> {
    for (full, local) in names {
        if RESERVED.contains(&local.as_str()) {
            return Err(CodegenError::ReservedName {
                message: full.to_string(),
                name: local,
            });
        }
    }
    Ok(())
}

/// Every marked-up component of every file in the descriptor set, so a
/// scaffold finds the owners of a policy's domains declared outside the
/// files it scaffolds.
pub(crate) fn pool_components(descriptor_set: &[u8]) -> Result<Vec<Component>, CodegenError> {
    let pool = DescriptorPool::decode(descriptor_set).map_err(|e| CodegenError::Descriptors {
        detail: e.to_string(),
    })?;
    let names: Vec<String> = pool.files().map(|f| f.name().to_string()).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    components(descriptor_set, &names, WRITE_KINDS)
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
    // A component's inputs and events are wherever the set declares them:
    // the files' messages first, then every other file's.
    let mut declaring = messages.clone();
    for file in pool.files() {
        for message in file.messages() {
            if !declaring.contains(&message) {
                declaring.push(message);
            }
        }
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
    // Each component's listed sources (a projector's input_domains or
    // input_domain), by index: what its (event) entries are held to.
    let mut listed: Vec<Option<Vec<String>>> = Vec::new();
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
        if number == PROJECTOR && !kinds.contains(&ComponentKind::Projector) {
            continue;
        }
        let Some(kind) = ComponentKind::of(number).filter(|k| kinds.contains(k)) else {
            return Err(CodegenError::UnsupportedKind {
                component: message.full_name().to_string(),
                kind: number,
            });
        };
        let resolutions: Vec<i32> = pool
            .get_extension_by_name(STATE)
            .and_then(|state_ext| message_value(&options, &state_ext))
            .map(|state| list_field(&state, "conflict_resolutions"))
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.as_enum_number())
            .collect();
        if commutative.is_some_and(|c| resolutions.contains(&c)) {
            return Err(CodegenError::Commutative {
                component: message.full_name().to_string(),
            });
        }
        let name = match string_field(&declared, "name") {
            n if n.is_empty() => message.name().to_string(),
            n => n,
        };
        let (domain, input_domain, output_domain) = (
            string_field(&declared, "domain"),
            string_field(&declared, "input_domain"),
            string_field(&declared, "output_domain"),
        );
        let input_domains = projector_sources(message, kind, &declared, &input_domain)?;
        let component_name = crate::names::unchecked(&crate::names::NameOf {
            kind: kind.name_kind(),
            component_name: &string_field(&declared, "component_name"),
            domain: &domain,
            input_domain: &input_domain,
            output_domain: &output_domain,
            anchor: message.name(),
        });
        if !crate::names::well_formed(&component_name) {
            return Err(CodegenError::ComponentName {
                component: message.full_name().to_string(),
                name: component_name,
            });
        }
        components.push(Component {
            anchor: message.clone(),
            kind,
            name,
            component_name,
            domain,
            input_domain,
            output_domain,
            inputs: Vec::new(),
            events: Vec::new(),
            sources: input_domains.clone().unwrap_or_default(),
            resolutions,
        });
        listed.push(input_domains);
    }

    // Events first, so a command's `emits` can be checked against them.
    let mut seen: Vec<Vec<String>> = vec![Vec::new(); components.len()];
    if let Some(event_ext) = &event_ext {
        for message in &declaring {
            let options = message.options();
            if !options.has_extension(event_ext) {
                continue;
            }
            for consumer in options.get_extension(event_ext).as_list().unwrap_or(&[]) {
                let Value::Message(consumer) = consumer else {
                    continue;
                };
                let named = string_field(consumer, "component");
                // A consumer outside the set is a reference to a component
                // generated elsewhere (decision 14.50), skipped.
                let Some(index) = components
                    .iter()
                    .position(|c| c.anchor.full_name() == named)
                else {
                    continue;
                };
                let component = &mut components[index];
                if !component.events.contains(message) {
                    component.events.push(message.clone());
                }
                let source = string_field(consumer, "domain");
                match &listed[index] {
                    // A projector listing its sources: each entry names one
                    // of them (an entry of one input_domain may leave it
                    // unset).
                    Some(list) => {
                        let unset_of_one = source.is_empty() && component.input_domain == list[0];
                        if !unset_of_one && !list.contains(&source) {
                            return Err(CodegenError::UnlistedSource {
                                component: component.anchor.full_name().to_string(),
                                event: message.full_name().to_string(),
                                domain: source,
                            });
                        }
                        seen[index].push(source);
                    }
                    None => {
                        if !source.is_empty() && !component.sources.contains(&source) {
                            component.sources.push(source);
                        }
                    }
                }
            }
        }
    }
    for ((component, list), seen) in components.iter().zip(&listed).zip(&seen) {
        let Some(list) = list.as_ref().filter(|_| component.input_domain.is_empty()) else {
            continue;
        };
        if let Some(unnamed) = list.iter().find(|d| !seen.contains(d)) {
            return Err(CodegenError::UnprojectedSource {
                component: component.anchor.full_name().to_string(),
                domain: unnamed.clone(),
            });
        }
    }
    if let Some(command_ext) = &command_ext {
        for message in &declaring {
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
            let cardinality = match enum_field(&declared, "emits_cardinality") {
                CARDINALITY_MANY => Cardinality::Many,
                // With no emits, the raw form is a list: unset is MANY.
                0 if emits.is_empty() => Cardinality::Many,
                n @ (CARDINALITY_ONE | CARDINALITY_OPTIONAL) if emits.is_empty() => {
                    return Err(CodegenError::CardinalityWithoutEmits {
                        message: message.full_name().to_string(),
                        optional: n == CARDINALITY_OPTIONAL,
                    });
                }
                CARDINALITY_OPTIONAL => Cardinality::Optional,
                _ => Cardinality::One,
            };
            if cardinality != Cardinality::Many && emits.len() > MAX_TUPLE {
                return Err(CodegenError::EmitsTuple {
                    message: message.full_name().to_string(),
                    count: emits.len(),
                });
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
                cardinality,
                refuses,
                starts: bool_field(&declared, "starts"),
                fact,
            });
        }
    }
    check_refusal_statuses(&pool, files, &declaring, command_ext.as_ref())?;
    Ok(components)
}

/// The status a refusal Reason's value marks (decision 14.52), its
/// `google.rpc.Code` number: 0 where unmarked, or where the set's
/// options.proto has no `refusal_status`.
pub(crate) fn refusal_status(value: &EnumValueDescriptor) -> i32 {
    let Some(ext) = value
        .parent_enum()
        .parent_pool()
        .get_extension_by_name(REFUSAL_STATUS)
    else {
        return 0;
    };
    let options = value.options();
    if !options.has_extension(&ext) {
        return 0;
    }
    options.get_extension(&ext).as_enum_number().unwrap_or(0)
}

/// Every enum of `files`, nested ones too, each value marked with a
/// `refusal_status` checked (C-0746): it must be a non-zero value of the
/// nested `Reason` of a message some command of the set refuses with.
fn check_refusal_statuses(
    pool: &DescriptorPool,
    files: &[&str],
    declaring: &[MessageDescriptor],
    command_ext: Option<&ExtensionDescriptor>,
) -> Result<(), CodegenError> {
    let Some(status_ext) = pool.get_extension_by_name(REFUSAL_STATUS) else {
        return Ok(());
    };
    let mut refused: BTreeSet<String> = BTreeSet::new();
    if let Some(command_ext) = command_ext {
        for message in declaring {
            if let Some(declared) = message_value(&message.options(), command_ext) {
                refused.extend(list_field(&declared, "refuses").into_iter().filter_map(
                    |v| match v {
                        Value::String(s) => Some(s),
                        _ => None,
                    },
                ));
            }
        }
    }
    fn nested(message: MessageDescriptor, out: &mut Vec<EnumDescriptor>) {
        out.extend(message.child_enums());
        for child in message.child_messages() {
            nested(child, out);
        }
    }
    let mut enums = Vec::new();
    for file in files {
        if let Some(file) = pool.get_file_by_name(file) {
            enums.extend(file.enums());
            for message in file.messages() {
                nested(message, &mut enums);
            }
        }
    }
    for e in enums {
        let codes = e.parent_message().filter(|m| {
            refused.contains(m.full_name()) && crate::common::refusal_reason(m).as_ref() == Some(&e)
        });
        for value in e.values() {
            let marked = value.options().has_extension(&status_ext);
            if marked && (codes.is_none() || value.number() == 0) {
                return Err(CodegenError::RefusalStatusMisplaced {
                    value: format!("{}.{}", e.full_name(), value.name()),
                });
            }
        }
    }
    Ok(())
}

/// A projector's listed sources (decision 14.50): its `input_domains`, or
/// its one `input_domain`; `None` for a projector listing neither (its
/// entries name its sources) and for every other kind. Refused: both set,
/// a domain listed twice, and `input_domains` on another kind.
fn projector_sources(
    message: &MessageDescriptor,
    kind: ComponentKind,
    declared: &DynamicMessage,
    input_domain: &str,
) -> Result<Option<Vec<String>>, CodegenError> {
    let component = || message.full_name().to_string();
    let domains: Vec<String> = list_field(declared, "input_domains")
        .into_iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s),
            _ => None,
        })
        .collect();
    if !domains.is_empty() && kind != ComponentKind::Projector {
        return Err(CodegenError::InputDomainsOnNonProjector {
            component: component(),
        });
    }
    if !domains.is_empty() && !input_domain.is_empty() {
        return Err(CodegenError::InputDomainsWithInputDomain {
            component: component(),
        });
    }
    for (i, domain) in domains.iter().enumerate() {
        if domains[..i].contains(domain) {
            return Err(CodegenError::DuplicateSource {
                component: component(),
                domain: domain.clone(),
            });
        }
    }
    Ok(match (kind, domains.is_empty(), input_domain.is_empty()) {
        (ComponentKind::Projector, false, _) => Some(domains),
        (ComponentKind::Projector, true, false) => Some(vec![input_domain.to_string()]),
        _ => None,
    })
}

/// The component a `(command)` names: `Some` when it is generated here,
/// `None` when it is declared but outside `files`, and a refusal when
/// nothing declares it.
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
        // A nested enum is mapped (as `<Message><Enum>`); a nested message
        // is not yet.
        let nested = message
            .child_messages()
            .filter(|m| !m.is_map_entry())
            .map(|m| m.full_name().to_string())
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
