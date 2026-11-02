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

use crate::error::{CodegenError, MarkupError, ProtoError, UseError};
use crate::warnings::Warning;

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

    /// An aggregate or a process manager: a domain, owning a stream domain,
    /// deciding its inputs and folding its events.
    pub(crate) fn is_domain(self) -> bool {
        matches!(
            self,
            ComponentKind::Aggregate | ComponentKind::ProcessManager
        )
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

/// What one handling of an input returns (decision 14.50), for each
/// emitter to write in its own types.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Decided<'a> {
    /// One event of each type, in order: the one event, or a tuple.
    One(&'a [MessageDescriptor]),
    /// That, or nothing.
    Optional(&'a [MessageDescriptor]),
    /// Any number of events of these types, in any order: the input's
    /// emits or, when it lists none, any of its component's events.
    Many(&'a [MessageDescriptor]),
}

impl Input {
    /// What a handling of this input, of `component`, returns: by its
    /// cardinality, and an input listing no emits any number of the
    /// component's events.
    pub(crate) fn decided<'a>(&'a self, component: &'a Component) -> Decided<'a> {
        match self.cardinality {
            _ if self.emits.is_empty() => Decided::Many(&component.events),
            Cardinality::Many => Decided::Many(&self.emits),
            Cardinality::One => Decided::One(&self.emits),
            Cardinality::Optional => Decided::Optional(&self.emits),
        }
    }
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
    /// A policy's target: the aggregate or process manager of `components`
    /// owning its output domain, refused when none does.
    pub fn target<'a>(&self, components: &'a [Component]) -> Result<&'a Component, MarkupError> {
        owner(components, &self.output_domain).ok_or_else(|| MarkupError::UnownedDomain {
            policy: self.anchor.full_name().to_string(),
            domain: self.output_domain.clone(),
        })
    }

    /// A policy both of whose domains an aggregate or process manager of
    /// `components` owns: one an application of them runs.
    pub fn is_connected(&self, components: &[Component]) -> bool {
        owner(components, &self.input_domain).is_some()
            && owner(components, &self.output_domain).is_some()
    }

    /// The input whose message is `full_name`.
    pub fn input(&self, full_name: &str) -> Option<&Input> {
        self.inputs
            .iter()
            .find(|i| i.message.full_name() == full_name)
    }
}

/// The aggregate or process manager of `components` owning `domain`.
pub(crate) fn owner<'a>(components: &'a [Component], domain: &str) -> Option<&'a Component> {
    components
        .iter()
        .find(|c| c.kind.is_domain() && c.domain == domain)
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
                    return Err(ProtoError::RefusalChoiceName {
                        message: full.to_string(),
                        name: choice,
                        command: input.message.full_name().to_string(),
                    }
                    .into());
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
) -> Result<(), ProtoError> {
    for (full, local) in names {
        if RESERVED.contains(&local.as_str()) {
            return Err(ProtoError::ReservedName {
                message: full.to_string(),
                name: local,
            });
        }
    }
    Ok(())
}

/// `descriptor_set` decoded, refused when it is not a descriptor set.
pub(crate) fn decode(descriptor_set: &[u8]) -> Result<DescriptorPool, ProtoError> {
    DescriptorPool::decode(descriptor_set).map_err(|e| ProtoError::Descriptors {
        detail: e.to_string(),
    })
}

/// Every marked-up component of every file in the descriptor set, so a
/// scaffold finds the owners of a policy's domains declared outside the
/// files it scaffolds.
pub(crate) fn pool_components(descriptor_set: &[u8]) -> Result<Vec<Component>, CodegenError> {
    let pool = decode(descriptor_set)?;
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
    declare(descriptor_set, files, kinds, None)
}

/// [`components`] as a lint reads them: every rule of the declarations
/// held as generation holds it, but a command's `emits` naming an event its
/// component folds none of, which generation refuses
/// ([`MarkupError::UnknownEmit`]) and a lint warns of
/// ([`Warning::UnappliedEmit`]), that emit left out.
pub(crate) fn declarations(
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<(Vec<Component>, Vec<Warning>), CodegenError> {
    let mut unapplied = Vec::new();
    let components = declare(descriptor_set, files, ALL_KINDS, Some(&mut unapplied))?;
    Ok((components, unapplied))
}

/// The components declared in `files`, an unapplied emit refused, or kept
/// in `unapplied` when it is given.
fn declare(
    descriptor_set: &[u8],
    files: &[&str],
    kinds: &[ComponentKind],
    mut unapplied: Option<&mut Vec<Warning>>,
) -> Result<Vec<Component>, CodegenError> {
    let pool = decode(descriptor_set)?;
    let mut messages = Vec::new();
    for file in files {
        let found = pool
            .get_file_by_name(file)
            .ok_or_else(|| UseError::FileNotInSet {
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
    let Some(markup) = Markup::of(&pool) else {
        return Ok(Vec::new());
    };
    let mut declared = Vec::new();
    for message in &messages {
        declared.extend(markup.declared(message, kinds)?);
    }
    // Events first, so a command's `emits` can be checked against them.
    for message in &declaring {
        for consumer in markup.consumers(message) {
            attach_event(&mut declared, message, &consumer)?;
        }
    }
    for component in &declared {
        component.check_projected()?;
    }
    for message in &declaring {
        if let Some(command) = markup.command(message) {
            markup.attach_input(&mut declared, message, &command, unapplied.as_deref_mut())?;
        }
    }
    check_refusal_statuses(&markup, files, &declaring)?;
    Ok(declared.into_iter().map(|d| d.component).collect())
}

/// angzarr's declaration options as a descriptor set has them:
/// `(component)`, and `(command)`, `(event)` and `(state)` where its
/// options.proto declares them.
struct Markup<'a> {
    pool: &'a DescriptorPool,
    component: ExtensionDescriptor,
    command: Option<ExtensionDescriptor>,
    event: Option<ExtensionDescriptor>,
    state: Option<ExtensionDescriptor>,
}

impl<'a> Markup<'a> {
    /// `None` for a set that never imported options.proto.
    fn of(pool: &'a DescriptorPool) -> Option<Markup<'a>> {
        Some(Markup {
            pool,
            component: pool.get_extension_by_name(COMPONENT)?,
            command: pool.get_extension_by_name(COMMAND),
            event: pool.get_extension_by_name(EVENT),
            state: pool.get_extension_by_name(STATE),
        })
    }

    /// The `(component)` `message` declares.
    fn component(&self, message: &MessageDescriptor) -> Option<DynamicMessage> {
        values(message, Some(&self.component)).pop()
    }

    /// The `(command)` `message` declares: it is an input.
    fn command(&self, message: &MessageDescriptor) -> Option<DynamicMessage> {
        values(message, self.command.as_ref()).pop()
    }

    /// The `(event)` entries of `message`: each a component consuming it.
    fn consumers(&self, message: &MessageDescriptor) -> Vec<DynamicMessage> {
        values(message, self.event.as_ref())
    }

    /// The `(state)` `message` declares.
    fn state(&self, message: &MessageDescriptor) -> Option<DynamicMessage> {
        values(message, self.state.as_ref()).pop()
    }

    /// The component `message` declares, refusing a kind outside `kinds`;
    /// `None` when it declares none, or is a projector and `kinds` takes
    /// none.
    fn declared(
        &self,
        message: &MessageDescriptor,
        kinds: &[ComponentKind],
    ) -> Result<Option<Declared>, CodegenError> {
        let Some(declared) = self.component(message) else {
            return Ok(None);
        };
        let number = enum_field(&declared, "kind");
        // The markup is the write side: a projector beside it is skipped,
        // not refused.
        if number == PROJECTOR && !kinds.contains(&ComponentKind::Projector) {
            return Ok(None);
        }
        let Some(kind) = ComponentKind::of(number).filter(|k| kinds.contains(k)) else {
            return Err(MarkupError::UnsupportedKind {
                component: message.full_name().to_string(),
                kind: number,
            }
            .into());
        };
        let resolutions = self.resolutions(message)?;
        let name = match string_field(&declared, "name") {
            n if n.is_empty() => message.name().to_string(),
            n => n,
        };
        let (domain, input_domain, output_domain) = (
            string_field(&declared, "domain"),
            string_field(&declared, "input_domain"),
            string_field(&declared, "output_domain"),
        );
        match kind {
            ComponentKind::ProcessManager if !output_domain.is_empty() => {
                return Err(MarkupError::ProcessManagerOutputDomain {
                    component: message.full_name().to_string(),
                    domain: output_domain,
                }
                .into());
            }
            ComponentKind::Policy if output_domain.is_empty() => {
                return Err(MarkupError::NoOutputDomain {
                    component: message.full_name().to_string(),
                }
                .into());
            }
            _ => {}
        }
        let listed = projector_sources(message, kind, &declared, &input_domain)?;
        let component_name = crate::names::unchecked(&crate::names::NameOf {
            kind: kind.name_kind(),
            component_name: &string_field(&declared, "component_name"),
            domain: &domain,
            input_domain: &input_domain,
            output_domain: &output_domain,
            anchor: message.name(),
        });
        if !crate::names::well_formed(&component_name) {
            return Err(MarkupError::ComponentName {
                component: message.full_name().to_string(),
                name: component_name,
            }
            .into());
        }
        Ok(Some(Declared {
            listed: listed.is_some(),
            named: Vec::new(),
            component: Component {
                anchor: message.clone(),
                kind,
                name,
                component_name,
                domain,
                input_domain,
                output_domain,
                inputs: Vec::new(),
                events: Vec::new(),
                sources: listed.unwrap_or_default(),
                resolutions,
            },
        }))
    }

    /// A component's state's conflict resolutions (its `(state)` option's),
    /// refusing COMMUTATIVE, which is not generated yet.
    fn resolutions(&self, message: &MessageDescriptor) -> Result<Vec<i32>, MarkupError> {
        let commutative = self
            .pool
            .get_enum_by_name(RESOLUTION)
            .and_then(|e| e.get_value_by_name(COMMUTATIVE))
            .map(|v| v.number());
        let resolutions: Vec<i32> = self
            .state(message)
            .iter()
            .flat_map(|state| list_field(state, "conflict_resolutions"))
            .filter_map(|v| v.as_enum_number())
            .collect();
        if commutative.is_some_and(|c| resolutions.contains(&c)) {
            return Err(MarkupError::Commutative {
                component: message.full_name().to_string(),
            });
        }
        Ok(resolutions)
    }

    /// The input the `(command)` `message` declares (`declared`) added to
    /// the component it names, when that is generated here; refused when
    /// nothing declares it.
    fn attach_input(
        &self,
        components: &mut [Declared],
        message: &MessageDescriptor,
        declared: &DynamicMessage,
        unapplied: Option<&mut Vec<Warning>>,
    ) -> Result<(), CodegenError> {
        let named = string_field(declared, "component");
        if let Some(owner) = Declared::named(components, &named) {
            let input = command_input(self.pool, &owner.component, message, declared, unapplied)?;
            owner.component.inputs.push(input);
            return Ok(());
        }
        // Declared, but outside the files: generated elsewhere.
        match self.pool.get_message_by_name(&named) {
            Some(m) if m.options().has_extension(&self.component) => Ok(()),
            _ => Err(MarkupError::UnknownComponent {
                message: message.full_name().to_string(),
                component: named,
            }
            .into()),
        }
    }
}

/// The values `message` sets of the message option `ext`: one for a
/// singular option, each entry of a repeated one; none when it is unset, or
/// the set lacks the option.
fn values(message: &MessageDescriptor, ext: Option<&ExtensionDescriptor>) -> Vec<DynamicMessage> {
    let options = message.options();
    let Some(ext) = ext.filter(|ext| options.has_extension(ext)) else {
        return Vec::new();
    };
    match options.get_extension(ext).into_owned() {
        Value::Message(m) => vec![m],
        Value::List(entries) => entries
            .into_iter()
            .filter_map(|v| match v {
                Value::Message(m) => Some(m),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A component as it is declared: its `(event)` entries' source domains
/// held to the ones it lists, when it lists them.
struct Declared {
    component: Component,
    /// A projector listing its sources (`input_domains`, or its one
    /// `input_domain`): they are its `sources`, and every entry names one.
    listed: bool,
    /// The listed sources its entries named.
    named: Vec<String>,
}

impl Declared {
    /// The one of `components` whose anchor is `full_name`.
    fn named<'d>(components: &'d mut [Declared], full_name: &str) -> Option<&'d mut Declared> {
        components
            .iter_mut()
            .find(|d| d.component.anchor.full_name() == full_name)
    }

    /// `event`, of an `(event)` entry for this component naming `source`
    /// (empty: unset), added to its events, and `source` to its sources or
    /// held to its listed ones (an entry of one input_domain may leave it
    /// unset).
    fn attach(&mut self, event: &MessageDescriptor, source: String) -> Result<(), MarkupError> {
        let component = &mut self.component;
        if component.kind == ComponentKind::ProcessManager
            && !source.is_empty()
            && source != component.domain
        {
            return Err(MarkupError::ProcessManagerSubscribes {
                component: component.anchor.full_name().to_string(),
                event: event.full_name().to_string(),
                domain: source,
            });
        }
        if !component.events.contains(event) {
            component.events.push(event.clone());
        }
        if !self.listed {
            if !source.is_empty() && !component.sources.contains(&source) {
                component.sources.push(source);
            }
            return Ok(());
        }
        let unset_of_one = source.is_empty() && component.input_domain == component.sources[0];
        if !unset_of_one && !component.sources.contains(&source) {
            return Err(MarkupError::UnlistedSource {
                component: component.anchor.full_name().to_string(),
                event: event.full_name().to_string(),
                domain: source,
            });
        }
        self.named.push(source);
        Ok(())
    }

    /// Refuses a projector listing its sources in input_domains one of
    /// which no `(event)` entry for it names.
    fn check_projected(&self) -> Result<(), MarkupError> {
        let component = &self.component;
        if !self.listed || !component.input_domain.is_empty() {
            return Ok(());
        }
        match component.sources.iter().find(|d| !self.named.contains(d)) {
            Some(unnamed) => Err(MarkupError::UnprojectedSource {
                component: component.anchor.full_name().to_string(),
                domain: unnamed.clone(),
            }),
            None => Ok(()),
        }
    }
}

/// The `(event)` entry `consumer` of `event` attached to the component it
/// names. A consumer outside the set is a reference to a component
/// generated elsewhere (decision 14.50), skipped.
fn attach_event(
    components: &mut [Declared],
    event: &MessageDescriptor,
    consumer: &DynamicMessage,
) -> Result<(), MarkupError> {
    match Declared::named(components, &string_field(consumer, "component")) {
        Some(component) => component.attach(event, string_field(consumer, "domain")),
        None => Ok(()),
    }
}

/// The input the `(command)` `message` declares (`declared`) of
/// `component`: its emits, held to the component's events (one it folds
/// none of refused, or kept in `unapplied` and left out when that is
/// given), their cardinality, and its refusals, each a message of `pool`
/// in the command's own package.
fn command_input(
    pool: &DescriptorPool,
    component: &Component,
    message: &MessageDescriptor,
    declared: &DynamicMessage,
    mut unapplied: Option<&mut Vec<Warning>>,
) -> Result<Input, MarkupError> {
    let command = || message.full_name().to_string();
    let mut emits = Vec::new();
    for event in strings_field(declared, "emits") {
        match component.events.iter().find(|e| e.full_name() == event) {
            Some(found) => emits.push(found.clone()),
            None => match unapplied.as_deref_mut() {
                Some(warnings) => warnings.push(Warning::UnappliedEmit {
                    command: command(),
                    event,
                }),
                None => {
                    return Err(MarkupError::UnknownEmit {
                        command: command(),
                        event,
                    });
                }
            },
        }
    }
    let cardinality = cardinality(message, declared, emits.len())?;
    let fact = bool_field(declared, "fact");
    let refused = strings_field(declared, "refuses");
    if fact && !refused.is_empty() {
        return Err(MarkupError::RefusesOnFact { message: command() });
    }
    let refuses = refused
        .into_iter()
        .map(|refusal| {
            let found =
                pool.get_message_by_name(&refusal)
                    .ok_or_else(|| MarkupError::UnknownRefusal {
                        command: command(),
                        refusal: refusal.clone(),
                    })?;
            if found.package_name() != message.package_name() {
                return Err(MarkupError::ForeignRefusal {
                    command: command(),
                    refusal,
                });
            }
            Ok(found)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let starts = bool_field(declared, "starts");
    if starts && component.kind != ComponentKind::ProcessManager {
        return Err(MarkupError::StartsOnNonProcessManager {
            message: command(),
            component: component.anchor.full_name().to_string(),
        });
    }
    Ok(Input {
        message: message.clone(),
        emits,
        cardinality,
        refuses,
        starts,
        fact,
    })
}

/// A command's emits cardinality, given how many types it emits: refusing
/// ONE or OPTIONAL with none, and a tuple longer than the markup's.
fn cardinality(
    message: &MessageDescriptor,
    declared: &DynamicMessage,
    emits: usize,
) -> Result<Cardinality, MarkupError> {
    let cardinality = match enum_field(declared, "emits_cardinality") {
        CARDINALITY_MANY => Cardinality::Many,
        // With no emits, the raw form is a list: unset is MANY.
        0 if emits == 0 => Cardinality::Many,
        n @ (CARDINALITY_ONE | CARDINALITY_OPTIONAL) if emits == 0 => {
            return Err(MarkupError::CardinalityWithoutEmits {
                message: message.full_name().to_string(),
                optional: n == CARDINALITY_OPTIONAL,
            });
        }
        CARDINALITY_OPTIONAL => Cardinality::Optional,
        _ => Cardinality::One,
    };
    if cardinality != Cardinality::Many && emits > MAX_TUPLE {
        return Err(MarkupError::EmitsTuple {
            message: message.full_name().to_string(),
            count: emits,
        });
    }
    Ok(cardinality)
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
    markup: &Markup,
    files: &[&str],
    declaring: &[MessageDescriptor],
) -> Result<(), CodegenError> {
    let pool = markup.pool;
    let Some(status_ext) = pool.get_extension_by_name(REFUSAL_STATUS) else {
        return Ok(());
    };
    let refused: BTreeSet<String> = declaring
        .iter()
        .filter_map(|message| markup.command(message))
        .flat_map(|declared| strings_field(&declared, "refuses"))
        .collect();
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
                return Err(MarkupError::RefusalStatusMisplaced {
                    value: format!("{}.{}", e.full_name(), value.name()),
                }
                .into());
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
) -> Result<Option<Vec<String>>, MarkupError> {
    let component = || message.full_name().to_string();
    let domains = strings_field(declared, "input_domains");
    if !domains.is_empty() && kind != ComponentKind::Projector {
        return Err(MarkupError::InputDomainsOnNonProjector {
            component: component(),
        });
    }
    if !domains.is_empty() && !input_domain.is_empty() {
        return Err(MarkupError::InputDomainsWithInputDomain {
            component: component(),
        });
    }
    for (i, domain) in domains.iter().enumerate() {
        if domains[..i].contains(domain) {
            return Err(MarkupError::DuplicateSource {
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

/// The messages and enums reachable from a component's roots.
#[derive(Default)]
struct Reach {
    seen: BTreeSet<String>,
    messages: Vec<MessageDescriptor>,
    enums: Vec<EnumDescriptor>,
}

impl Reach {
    fn message(&mut self, message: &MessageDescriptor) -> Result<(), ProtoError> {
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
            return Err(ProtoError::NestedType {
                message: message.full_name().to_string(),
                nested,
            });
        }
        if let Some(oneof) = message.oneofs().find(|o| !o.is_synthetic()) {
            return Err(ProtoError::Oneof {
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
                    return Err(ProtoError::WellKnownType {
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

    fn enumeration(&mut self, e: &EnumDescriptor) -> Result<(), ProtoError> {
        if !self.seen.insert(e.full_name().to_string()) {
            return Ok(());
        }
        if let Some(zero) = e.values().find(|v| v.number() == 0)
            && !zero.name().ends_with("_UNSPECIFIED")
        {
            return Err(ProtoError::EnumZero {
                enum_name: e.full_name().to_string(),
                value: zero.name().to_string(),
            });
        }
        self.enums.push(e.clone());
        Ok(())
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

/// A repeated string field's values.
fn strings_field(message: &DynamicMessage, name: &str) -> Vec<String> {
    list_field(message, name)
        .into_iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s),
            _ => None,
        })
        .collect()
}

fn list_field(message: &DynamicMessage, name: &str) -> Vec<Value> {
    match message.get_field_by_name(name).as_deref() {
        Some(Value::List(values)) => values.clone(),
        _ => Vec::new(),
    }
}
