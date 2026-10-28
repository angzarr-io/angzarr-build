//! What a project's protos give angzarr-local (angzarr-embedded) besides the domain
//! layer: the scaffolded markup impls (angzarr-embedded-macros), written once into the
//! user's source tree, and the generated `register`, which registers every
//! component of the project with angzarr-embedded's `Registry`, so an application
//! calls one function.
//!
//! Pure, as the rest of the crate: descriptors (and, for [`complete`], the
//! user's current file) in, source out. Reading and writing files is
//! angzarr-build's.

use proc_macro2::TokenStream;
use prost_reflect::MessageDescriptor;
use quote::{format_ident, quote};

use crate::error::CodegenError;
use crate::model::{self, Component, ComponentKind, MARKUP_KINDS};
use crate::rust::{RUNTIME, ident, next, runtime, runtime_macros, snake};
use crate::{Generated, generate};

/// One component's scaffold: the user's file, written once and never
/// overwritten.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScaffoldFile {
    /// The component's anchor message, e.g. `buyin.v1.WalletState`.
    pub component: String,
    /// Where it goes, relative to the crate's `src`: `wallet.rs`.
    pub path: String,
    pub source: String,
}

/// angzarr-cli's refusal switch (backlog OB-7), set to translate for one
/// command of one policy: the policy's scaffold gets an
/// `#[on_refused(command)]` callback stub returning `fact`, a fact of its
/// source. Every command without one is passed through, the default.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Translate {
    /// The policy's anchor message, e.g. `buyin.v1.BuyInWalletPolicy`.
    pub policy: String,
    /// A command of the policy's target, e.g. `buyin.v1.Hold`.
    pub command: String,
    /// A command or fact of the policy's source, e.g. `buyin.v1.HoldLost`.
    pub fact: String,
}

/// The scaffold of every component declared in `files`, one file each:
/// the struct and its decorated impl (`#[aggregate]`, `#[process_manager]`
/// or `#[policy]`), one stub per input and event, `todo!()` bodies.
///
/// A policy's source and target are the aggregates or process managers
/// owning its `input_domain` and `output_domain`, declared in `files`.
pub fn scaffold(
    descriptor_set: &[u8],
    files: &[&str],
    translate: &[Translate],
) -> Result<Vec<ScaffoldFile>, CodegenError> {
    let components = model::components(descriptor_set, files, MARKUP_KINDS)?;
    for selection in translate {
        check_translate(&components, selection)?;
    }
    components
        .iter()
        .map(|component| {
            let source = match component.kind {
                ComponentKind::Policy => policy_source(component, &components, translate)?,
                ComponentKind::Aggregate | ComponentKind::ProcessManager => {
                    domain_source(component)
                }
            };
            Ok(ScaffoldFile {
                component: component.anchor.full_name().to_string(),
                path: format!("{}.rs", snake(&component.name)),
                source,
            })
        })
        .collect()
}

/// `selection`'s policy is a policy, its command one its target decides,
/// and its fact a fact its source ingests.
fn check_translate(components: &[Component], selection: &Translate) -> Result<(), CodegenError> {
    let policy = components
        .iter()
        .find(|c| c.kind == ComponentKind::Policy && c.anchor.full_name() == selection.policy)
        .ok_or_else(|| CodegenError::TranslateUnknownPolicy {
            policy: selection.policy.clone(),
        })?;
    let takes = owner(components, &policy.output_domain)
        .and_then(|target| target.input(&selection.command))
        .is_some_and(|input| !input.fact);
    if !takes {
        return Err(CodegenError::TranslateNotATargetCommand {
            policy: selection.policy.clone(),
            command: selection.command.clone(),
        });
    }
    match owner(components, &policy.input_domain).and_then(|source| source.input(&selection.fact)) {
        None => Err(CodegenError::TranslateNotASourceFact {
            policy: selection.policy.clone(),
            fact: selection.fact.clone(),
        }),
        Some(input) if !input.fact => Err(CodegenError::TranslateNotAFact {
            policy: selection.policy.clone(),
            fact: selection.fact.clone(),
        }),
        Some(_) => Ok(()),
    }
}

/// The aggregate or process manager owning `domain`.
fn owner<'a>(components: &'a [Component], domain: &str) -> Option<&'a Component> {
    components
        .iter()
        .find(|c| c.kind != ComponentKind::Policy && c.domain == domain)
}

/// One method stub of a scaffold: its marker and its signature.
struct Stub {
    /// The marker's text, `handles_cmd(Hold, refuses(InsufficientFunds))`.
    marker: String,
    receiver: &'static str,
    name: String,
    params: Vec<String>,
    output: Option<String>,
}

impl Stub {
    /// The stub as rustfmt lays it out in an impl: the signature on one
    /// line when it fits in 100 columns, else one parameter a line.
    fn render(&self) -> String {
        let output = self
            .output
            .as_ref()
            .map(|o| format!(" -> {o}"))
            .unwrap_or_default();
        let params: Vec<&str> = std::iter::once(self.receiver)
            .chain(self.params.iter().map(String::as_str))
            .collect();
        let line = format!("    fn {}({}){output} {{", self.name, params.join(", "));
        let signature = if line.len() <= MAX_WIDTH {
            line
        } else {
            let params: String = params.iter().map(|p| format!("        {p},\n")).collect();
            format!("    fn {}(\n{params}    ){output} {{", self.name)
        };
        format!(
            "    #[{}]\n{signature}\n        todo!()\n    }}\n",
            self.marker
        )
    }
}

/// rustfmt's default line width, which the scaffold is laid out to.
const MAX_WIDTH: usize = 100;

/// A method name for `message`: its snake case, raw where it is a keyword.
fn method(message: &str) -> String {
    ident(&snake(message)).to_string()
}

/// An aggregate's or a process manager's scaffold.
fn domain_source(component: &Component) -> String {
    let markup = match component.kind {
        ComponentKind::ProcessManager => "process_manager",
        _ => "aggregate",
    };
    let mut markup_items = vec!["Context"];
    let mut macros = vec![markup, "applies"];
    let mut infallible = false;
    let mut stubs = Vec::new();
    for input in &component.inputs {
        let ty = input.message.name();
        let starts = if input.starts { ", starts" } else { "" };
        if input.fact {
            macros.push("handles_fact");
            stubs.push(Stub {
                marker: format!("handles_fact({ty}{starts})"),
                receiver: "&self",
                name: method(ty),
                params: vec![format!("fact: {ty}"), "ctx: &Context".to_string()],
                output: Some(shape(&input.emits, "Record", &mut markup_items)),
            });
            continue;
        }
        macros.push("handles_cmd");
        let output = shape(&input.emits, "Emit", &mut markup_items);
        let (refuses, error) = if input.refuses.is_empty() {
            infallible = true;
            (String::new(), "Infallible".to_string())
        } else {
            markup_items.push("Refuses");
            let refusals: Vec<&str> = input.refuses.iter().map(|r| r.name()).collect();
            (
                format!(", refuses({})", refusals.join(", ")),
                format!("Refuses<Self, {ty}>"),
            )
        };
        stubs.push(Stub {
            marker: format!("handles_cmd({ty}{starts}{refuses})"),
            receiver: "&self",
            name: method(ty),
            params: vec![format!("cmd: {ty}"), "ctx: &Context".to_string()],
            output: Some(format!("Result<{output}, {error}>")),
        });
    }
    for event in &component.events {
        let ty = event.name();
        stubs.push(Stub {
            marker: format!("applies({ty})"),
            receiver: "&mut self",
            name: method(ty),
            params: vec![format!("event: {ty}")],
            output: None,
        });
    }
    let mut out = header(component);
    if infallible {
        out.push_str("use std::convert::Infallible;\n\n");
    }
    out.push_str(&uses(&markup_items, &macros));
    out.push_str("\nuse crate::domain::*;\n\n");
    out.push_str(&format!(
        "#[derive(Clone, Debug, Default)]\npub struct {} {{}}\n\n",
        component.name
    ));
    out.push_str(&format!("#[{markup}(domain = {:?})]\n", component.domain));
    out.push_str(&decorated(&component.name, &stubs));
    out
}

/// A handler's output for `emits`: the one event, a tuple of up to four,
/// else `open<Self>` (`Emit` or `Record`), which takes any.
fn shape(
    emits: &[MessageDescriptor],
    open: &'static str,
    markup_items: &mut Vec<&'static str>,
) -> String {
    let events: Vec<&str> = emits.iter().map(|e| e.name()).collect();
    match events.as_slice() {
        [one] => one.to_string(),
        several if (2..=4).contains(&several.len()) => format!("({})", several.join(", ")),
        _ => {
            markup_items.push(open);
            format!("{open}<Self>")
        }
    }
}

/// `impl name { stubs }`, a blank line between stubs.
fn decorated(name: &str, stubs: &[Stub]) -> String {
    if stubs.is_empty() {
        return format!("impl {name} {{}}\n");
    }
    let body: Vec<String> = stubs.iter().map(Stub::render).collect();
    format!("impl {name} {{\n{}}}\n", body.join("\n"))
}

/// A policy's scaffold: a stub per event it reacts to, and an
/// `#[on_refused]` callback stub per command `translate` selects for it.
fn policy_source(
    component: &Component,
    components: &[Component],
    translate: &[Translate],
) -> Result<String, CodegenError> {
    let owned = |domain: &str| {
        owner(components, domain).ok_or_else(|| CodegenError::UnownedDomain {
            policy: component.anchor.full_name().to_string(),
            domain: domain.to_string(),
        })
    };
    let source = owned(&component.input_domain)?;
    let target = owned(&component.output_domain)?;
    let mut markup_items = vec!["Commands", "PolicyContext"];
    let mut macros = vec!["on_event", "policy"];
    let mut stubs = Vec::new();
    for event in &component.events {
        let ty = event.name();
        stubs.push(Stub {
            marker: format!("on_event({ty})"),
            receiver: "&self",
            name: method(ty),
            params: vec![format!("event: &{ty}"), "ctx: &PolicyContext".to_string()],
            output: Some(format!("Commands<{}>", target.name)),
        });
    }
    let selected = translate
        .iter()
        .filter(|t| t.policy == component.anchor.full_name());
    for selection in selected {
        let short = |full: &str| full.rsplit('.').next().unwrap_or(full).to_string();
        let (command, fact) = (short(&selection.command), short(&selection.fact));
        stubs.push(Stub {
            marker: format!("on_refused({command})"),
            receiver: "&self",
            name: format!("{}_refused", snake(&command)),
            params: vec![format!("refusal: &Refusal<{command}>")],
            output: Some(fact),
        });
        markup_items.push("Refusal");
        macros.push("on_refused");
    }
    let mut crate_uses = vec![
        format!("use crate::{}::{};", snake(&source.name), source.name),
        format!("use crate::{}::{};", snake(&target.name), target.name),
        "use crate::domain::*;".to_string(),
    ];
    crate_uses.sort();
    crate_uses.dedup();
    let mut out = header(component);
    out.push_str(&uses(&markup_items, &macros));
    out.push('\n');
    for line in crate_uses {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!("\npub struct {};\n\n", component.name));
    out.push_str(&format!(
        "#[policy(name = {:?}, source = {}, target = {})]\n",
        snake(&component.name).replace('_', "-"),
        source.name,
        target.name
    ));
    out.push_str(&decorated(&component.name, &stubs));
    Ok(out)
}

/// The scaffold's opening doc comment and the blank line after it.
fn header(component: &Component) -> String {
    format!(
        "//! {}: the scaffold of `{}`, written once by\n\
         //! `angzarr scaffold rust` and yours from then on: `--missing` adds the\n\
         //! stubs it lacks.\n\n",
        component.name,
        component.anchor.full_name()
    )
}

/// The `use` items for the runtime's markup types and macros, sorted.
fn uses(markup_items: &[&str], macros: &[&str]) -> String {
    let line = |path: String, names: &[&str]| {
        let mut names = names.to_vec();
        names.sort();
        names.dedup();
        match names.as_slice() {
            [one] => format!("use {path}::{one};\n"),
            _ => format!("use {path}::{{{}}};\n", names.join(", ")),
        }
    };
    line(format!("{}::next::markup", runtime()), markup_items)
        + &line(runtime_macros().to_string(), macros)
}

/// `tokens`, a whole file, formatted.
fn file(tokens: TokenStream) -> String {
    let file: syn::File = syn::parse2(tokens).expect("the emitter writes valid Rust");
    prettyplease::unparse(&file)
}

/// `existing`, the user's file for `fresh`'s component, with the stubs of
/// `fresh` that it lacks inserted into its decorated impl, and nothing else
/// changed; `None` when it lacks none. A stub is present when the impl has
/// any marker naming its type. A `#[handles_cmd]` on a fact, or a
/// `#[handles_fact]` on a command, is refused, the file left as it is.
///
/// The stubs are inserted in one place, before the impl's closing brace; a
/// stub needing a marker or a type the file does not import yet compiles
/// once the user adds the import.
pub fn complete(fresh: &ScaffoldFile, existing: &str) -> Result<Option<String>, CodegenError> {
    let not_found = || CodegenError::ScaffoldImplNotFound {
        path: fresh.path.clone(),
        component: fresh.component.clone(),
    };
    let fresh_file = syn::parse_file(&fresh.source).expect("a scaffold parses");
    let fresh_impl = decorated_impl(&fresh_file, None).expect("a scaffold holds its impl");
    let user_file = syn::parse_file(existing).map_err(|_| not_found())?;
    let user_impl = decorated_impl(&user_file, Some(fresh_impl)).ok_or_else(not_found)?;

    let expected = markers(fresh_impl);
    let found = markers(user_impl);
    let package = fresh
        .component
        .rsplit_once('.')
        .map_or("", |(package, _)| package);
    for (marker, ty, _) in &found {
        let contradicted = expected
            .iter()
            .find(|(m, t, _)| t == ty && m != marker && is_input(m) && is_input(marker));
        if let Some((expected, _, _)) = contradicted {
            return Err(CodegenError::MarkerMismatch {
                path: fresh.path.clone(),
                message: format!("{package}.{ty}"),
                expected: expected.clone(),
                found: marker.clone(),
            });
        }
    }
    let missing: Vec<&syn::ImplItemFn> = expected
        .iter()
        .filter(|(_, ty, _)| !found.iter().any(|(_, t, _)| t == ty))
        .map(|(_, _, method)| *method)
        .collect();
    if missing.is_empty() {
        return Ok(None);
    }

    // The stubs as the scaffold lays them out, a blank line before each.
    let mut stubs = String::new();
    for method in missing {
        let first = method
            .attrs
            .first()
            .map_or(method.sig.fn_token.span, |a| a.pound_token.span);
        let start = line_start(&fresh.source, first.start().line);
        let end = line_start(
            &fresh.source,
            method.block.brace_token.span.close().end().line + 1,
        );
        stubs.push('\n');
        stubs.push_str(&fresh.source[start..end]);
    }

    let close = user_impl.brace_token.span.close().start();
    let line = line_start(existing, close.line);
    let brace = line
        + existing[line..]
            .char_indices()
            .nth(close.column)
            .map_or(0, |(i, _)| i);
    let at = if existing[line..brace].trim().is_empty() {
        line
    } else {
        stubs.insert(0, '\n');
        brace
    };
    let mut completed = existing.to_string();
    completed.insert_str(at, &stubs);
    Ok(Some(completed))
}

/// The byte offset where 1-based line `line` of `text` starts (the length
/// of `text` past its last line).
fn line_start(text: &str, line: usize) -> usize {
    text.split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum()
}

/// The markers that name an input's handler.
fn is_input(marker: &str) -> bool {
    marker == "handles_cmd" || marker == "handles_fact"
}

/// The markup macros on a component's impl.
const IMPL_MARKUP: &[&str] = &["aggregate", "process_manager", "policy"];
/// The markers on its methods.
const MARKERS: &[&str] = &[
    "handles_cmd",
    "handles_fact",
    "applies",
    "on_event",
    "on_refused",
];

/// The inherent impl carrying a markup macro: the first in `file`, or, with
/// `like`, the one of the same type and the same macro.
fn decorated_impl<'a>(
    file: &'a syn::File,
    like: Option<&syn::ItemImpl>,
) -> Option<&'a syn::ItemImpl> {
    let shape = |imp: &syn::ItemImpl| {
        let markup = imp
            .attrs
            .iter()
            .filter_map(|a| a.path().get_ident().map(|i| i.to_string()))
            .find(|name| IMPL_MARKUP.contains(&name.as_str()))?;
        let ty = &imp.self_ty;
        Some((markup, quote!(#ty).to_string()))
    };
    let wanted = like.map(shape);
    file.items.iter().find_map(|item| match item {
        syn::Item::Impl(imp) if imp.trait_.is_none() => {
            let found = shape(imp)?;
            match &wanted {
                Some(wanted) if wanted.as_ref() != Some(&found) => None,
                _ => Some(imp),
            }
        }
        _ => None,
    })
}

/// Each marker on the impl's methods: its name, the type it names (the
/// last segment of its first argument), and its method.
fn markers(imp: &syn::ItemImpl) -> Vec<(String, String, &syn::ImplItemFn)> {
    let mut found = Vec::new();
    for item in &imp.items {
        let syn::ImplItem::Fn(method) = item else {
            continue;
        };
        for attr in &method.attrs {
            let Some(name) = attr.path().get_ident().map(|i| i.to_string()) else {
                continue;
            };
            if !MARKERS.contains(&name.as_str()) {
                continue;
            }
            let Ok(list) = attr.meta.require_list() else {
                continue;
            };
            let first: TokenStream = list
                .tokens
                .clone()
                .into_iter()
                .take_while(
                    |t| !matches!(t, proc_macro2::TokenTree::Punct(p) if p.as_char() == ','),
                )
                .collect();
            if let Ok(path) = syn::parse2::<syn::Path>(first)
                && let Some(last) = path.segments.last()
            {
                found.push((name, last.ident.to_string(), method));
            }
        }
    }
    found
}

/// The generated `register(&mut Registry)`: every component declared in
/// `files`, registered with the runtime, each at the path its scaffold
/// gives it.
pub fn register(descriptor_set: &[u8], files: &[&str]) -> Result<String, CodegenError> {
    let components = model::components(descriptor_set, files, MARKUP_KINDS)?;
    let registrations = components.iter().map(|component| {
        let module = format_ident!("{}", snake(&component.name));
        let name = ident(&component.name);
        let ty = quote!(crate::#module::#name);
        match component.kind {
            ComponentKind::Aggregate => quote!(registry.aggregate::<#ty>(#ty::default);),
            ComponentKind::ProcessManager => {
                quote!(registry.process_manager::<#ty>(#ty::default);)
            }
            ComponentKind::Policy => quote!(registry.policy(#ty);),
        }
    });
    let next = next();
    let doc = [
        format!(" Registers every component of the project with {RUNTIME}, which dispatches"),
        " each through angzarr-router: an application calls this once, at startup.".to_string(),
    ];
    let source = file(quote! {
        #(#[doc = #doc])*
        pub fn register(registry: &mut #next::Registry) {
            #(#registrations)*
        }
    });
    Ok(format!(
        "// @generated by angzarr-codegen from {}. Do not edit.\n\n{source}",
        files.join(", ")
    ))
}

/// The domain layer for marked-up components: the domain types, their wire
/// conversions and each one's `DomainMessage` impl, and nothing the markup
/// generates itself (no payload enums, no `State` impl).
pub fn markup_domain(
    descriptor_set: &[u8],
    files: &[&str],
    wire: &[(&str, &str)],
) -> Result<Vec<Generated>, CodegenError> {
    generate(
        model::build_markup(descriptor_set, files)?,
        wire,
        crate::rust::markup_package,
    )
}
