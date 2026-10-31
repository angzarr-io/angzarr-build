//! Each component's whole `ComponentDescriptor` (types.proto), generated
//! at build time from its protos (decision 14.42): its name in an
//! application (decision 14.50) and kind, the
//! domains it subscribes to, its state's type and resolved conflict
//! resolutions, a process manager's starting type names, a policy's output
//! domain, and the build's version as its rules version. The business
//! process answers `ComponentService.Describe` with it unchanged; the
//! coordinator validates it.
//!
//! It is built as a dynamic message of the descriptor set's own
//! `io.angzarr.v1.ComponentDescriptor` (angzarr's protos are always in the
//! set: options.proto imports types.proto) and kept encoded, so each
//! language's generated code holds the same bytes whatever version of the
//! wire types it is built against.

use prost_reflect::prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, ReflectMessage, Value};
use quote::quote;

use crate::error::CodegenError;
use crate::model::{self, ALL_KINDS, Component, ComponentKind};

const COMPONENT_DESCRIPTOR: &str = "io.angzarr.v1.ComponentDescriptor";
/// `ConflictResolution`'s implicit base (types.proto): STRICT and
/// AGGREGATE_HANDLES, which every state supports.
const BASE_RESOLUTIONS: [i32; 2] = [2, 3];

/// One component's generated descriptor.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Descriptor {
    /// The component's anchor message, e.g. `orders.v1.OrderState`.
    pub component: String,
    /// `ComponentDescriptor.name`: the component's name in an application
    /// (`component_name`, else its kind's default, decision 14.50).
    pub name: String,
    /// The `ComponentDescriptor`, encoded.
    pub encoded: Vec<u8>,
}

/// The descriptor of every component declared in `files`, in declaration
/// order, `rules_version` the build's version.
pub fn descriptors(
    descriptor_set: &[u8],
    files: &[&str],
    rules_version: &str,
) -> Result<Vec<Descriptor>, CodegenError> {
    Ok(build(descriptor_set, files, rules_version)?
        .into_iter()
        .map(|(_, d)| d)
        .collect())
}

/// Each component's descriptor beside the component.
pub(crate) fn build(
    descriptor_set: &[u8],
    files: &[&str],
    rules_version: &str,
) -> Result<Vec<(Component, Descriptor)>, CodegenError> {
    let components = model::components(descriptor_set, files, ALL_KINDS)?;
    let pool = DescriptorPool::decode(descriptor_set).map_err(|e| CodegenError::Descriptors {
        detail: e.to_string(),
    })?;
    components
        .into_iter()
        .map(|component| {
            let encoded = encode(&pool, &component, rules_version)?;
            let descriptor = Descriptor {
                component: component.anchor.full_name().to_string(),
                name: component.component_name.clone(),
                encoded,
            };
            Ok((component, descriptor))
        })
        .collect()
}

/// `component`'s `ComponentDescriptor`, encoded.
fn encode(
    pool: &DescriptorPool,
    component: &Component,
    rules_version: &str,
) -> Result<Vec<u8>, CodegenError> {
    let mut descriptor = new(pool, COMPONENT_DESCRIPTOR)?;
    set(
        &mut descriptor,
        "name",
        Value::String(component.component_name.clone()),
    )?;
    set(
        &mut descriptor,
        "kind",
        Value::EnumNumber(component.kind.number()),
    )?;
    let mut inputs = Vec::new();
    for domain in subscriptions(component) {
        let mut target = new(pool, "io.angzarr.v1.Target")?;
        set(&mut target, "domain", Value::String(domain))?;
        inputs.push(Value::Message(target));
    }
    set(&mut descriptor, "inputs", Value::List(inputs))?;
    set(
        &mut descriptor,
        "rules_version",
        Value::String(rules_version.to_string()),
    )?;
    if matches!(
        component.kind,
        ComponentKind::Aggregate | ComponentKind::ProcessManager
    ) {
        let mut options = new(pool, "io.angzarr.v1.StateOptions")?;
        let mut resolutions: Vec<i32> = BASE_RESOLUTIONS
            .into_iter()
            .chain(component.resolutions.iter().copied())
            .collect();
        resolutions.sort_unstable();
        resolutions.dedup();
        set(
            &mut options,
            "conflict_resolutions",
            Value::List(resolutions.into_iter().map(Value::EnumNumber).collect()),
        )?;
        let mut state = new(pool, "io.angzarr.v1.StateDescriptor")?;
        set(
            &mut state,
            "type_url",
            Value::String(format!("/{}", component.anchor.full_name())),
        )?;
        set(&mut state, "options", Value::Message(options))?;
        set(&mut descriptor, "state", Value::Message(state))?;
    }
    let starting = component
        .inputs
        .iter()
        .filter(|i| i.starts)
        .map(|i| Value::String(i.message.full_name().to_string()))
        .collect();
    set(&mut descriptor, "starting_types", Value::List(starting))?;
    set(
        &mut descriptor,
        "output_domain",
        Value::String(component.output_domain.clone()),
    )?;
    Ok(descriptor.encode_to_vec())
}

/// The domains a component subscribes to: its `input_domain`, else its
/// sources (a projector's `input_domains`, else the domains of its
/// `(event)` entries, in order). An aggregate or a process manager has
/// neither.
fn subscriptions(component: &Component) -> Vec<String> {
    if component.input_domain.is_empty() {
        component.sources.clone()
    } else {
        vec![component.input_domain.clone()]
    }
}

/// An empty message of the set's `name`.
fn new(pool: &DescriptorPool, name: &str) -> Result<DynamicMessage, CodegenError> {
    pool.get_message_by_name(name)
        .map(DynamicMessage::new)
        .ok_or_else(|| CodegenError::Descriptors {
            detail: format!("the set declares no {name}"),
        })
}

/// `message.field = value`, refused when the set's message has no such
/// field: angzarr's protos in the set are older than this generator.
fn set(message: &mut DynamicMessage, field: &str, value: Value) -> Result<(), CodegenError> {
    message
        .try_set_field_by_name(field, value)
        .map_err(|e| CodegenError::Descriptors {
            detail: format!("{}: {e}", message.descriptor().full_name()),
        })
}

/// The Rust build layer's descriptors (`angzarr-descriptors.rs`): every
/// component's in `files` but those of the `remote` domains, whose
/// business process (a child) has its own.
pub fn descriptors_rs(
    descriptor_set: &[u8],
    files: &[&str],
    remote: &[&str],
    rules_version: &str,
) -> Result<String, CodegenError> {
    let entries = build(descriptor_set, files, rules_version)?
        .into_iter()
        .filter(|(c, _)| !remote.contains(&c.domain.as_str()))
        .map(|(_, d)| {
            let name = d.name;
            let bytes = proc_macro2::Literal::byte_string(&d.encoded);
            quote!((#name, #bytes))
        });
    let doc = [
        " Each component's `ComponentDescriptor`, encoded, by name: generated from".to_string(),
        format!(" its protos with this build's version, {rules_version:?}, as its"),
        " rules_version (decision 14.42). It is answered to `Describe` unchanged.".to_string(),
    ];
    let source = crate::scaffold::file(quote! {
        #(#[doc = #doc])*
        pub const DESCRIPTORS: &[(&str, &[u8])] = &[#(#entries),*];
    });
    Ok(crate::headed(
        ".rs",
        format!(
            "// @generated by angzarr-codegen from {}. Do not edit.\n\n{source}",
            files.join(", ")
        ),
    ))
}
