//! What a project's protos give an angzarr-embedded application besides the
//! domain layer, split so the user's logic never names the runtime:
//!
//! - the user's, written once into the source tree by `angzarr scaffold
//!   rust` and completed with `--missing`: per component an interface (a
//!   trait over the domain types) and its implementation (every body
//!   `todo!()`), neither naming angzarr-embedded or its markup;
//! - the CLI's, regenerated into `OUT_DIR` every build: the adapters
//!   ([`adapters`]), each component's angzarr-embedded markup delegating to
//!   its interface, and [`register`], which registers every adapter with
//!   angzarr-embedded's `Registry`, so an application calls one function.
//!
//! A change to the runtime or to the scaffold touches only the generated
//! layer. Pure, as the rest of the crate: descriptors (and, for
//! [`complete`], the user's current file) in, source out. Reading and
//! writing files is angzarr-build's.

use proc_macro2::TokenStream;
use prost_reflect::MessageDescriptor;
use quote::{format_ident, quote};

use crate::error::CodegenError;
use crate::model::{self, Component, ComponentKind, MARKUP_KINDS};
use crate::rust::{RUNTIME, ident, next, runtime_macros, snake};
use crate::{Generated, generate};

/// One file of a scaffold.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScaffoldFile {
    /// The component's anchor message, e.g. `buyin.v1.WalletState`.
    pub component: String,
    /// Where it goes, relative to the crate's `src` (`wallet.rs`) or the
    /// project's root.
    pub path: String,
    pub source: String,
    pub owner: Owner,
}

/// Who owns a scaffolded file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owner {
    /// The user's: written once when absent, then only ever completed with
    /// `--missing`.
    User,
    /// The CLI's: rewritten on every run, never edited by hand.
    Generated,
}

/// angzarr-cli's refusal switch (backlog OB-7), set to translate for one
/// command of one policy: the policy's interface gets a `<command>_refused`
/// method returning `fact`, a fact of its source. Every command without one
/// is passed through, the default.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Translate {
    /// The policy's anchor message, e.g. `buyin.v1.BuyInWalletPolicy`.
    pub policy: String,
    /// A command of the policy's target, e.g. `buyin.v1.Hold`.
    pub command: String,
    /// A command or fact of the policy's source, e.g. `buyin.v1.HoldLost`.
    pub fact: String,
}

/// The scaffold of every component declared in `files`, two files each:
/// its interface (`<component>.rs`, a trait) and its implementation
/// (`<component>_impl.rs`), the user's.
///
/// A policy is scaffolded when an aggregate or a process manager of the
/// descriptor set owns each of its `input_domain` and `output_domain`.
pub fn scaffold(
    descriptor_set: &[u8],
    files: &[&str],
    translate: &[Translate],
) -> Result<Vec<ScaffoldFile>, CodegenError> {
    let components = model::components(descriptor_set, files, MARKUP_KINDS)?;
    let pool = model::pool_components(descriptor_set)?;
    for selection in translate {
        check_translate(&pool, selection)?;
    }
    let mut out = Vec::new();
    for component in &components {
        if component.kind == ComponentKind::Policy
            && (owner(&pool, &component.input_domain).is_none()
                || owner(&pool, &component.output_domain).is_none())
        {
            continue;
        }
        let stem = snake(&component.name);
        let anchor = component.anchor.full_name().to_string();
        let selected: Vec<&Translate> = translate.iter().filter(|t| t.policy == anchor).collect();
        let methods = interface_methods(component, &pool, &selected)?;
        out.push(ScaffoldFile {
            component: anchor.clone(),
            path: format!("{stem}.rs"),
            source: interface_source(component, &methods),
            owner: Owner::User,
        });
        out.push(ScaffoldFile {
            component: anchor,
            path: format!("{stem}_impl.rs"),
            source: impl_source(component, &methods),
            owner: Owner::User,
        });
    }
    Ok(out)
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

/// One method of a component's interface: its signature, the message it
/// takes (by name) and the parameter it takes it as.
struct Method {
    name: String,
    receiver: &'static str,
    /// `(param, type)`, after the receiver.
    params: Vec<(String, String)>,
    output: Option<String>,
}

impl Method {
    /// The signature as rustfmt lays it out at an indent of 4: on one line
    /// when it fits in 100 columns, else one parameter a line. `params`
    /// names the parameters (prefixed `_` in an implementation).
    fn signature(&self, prefix: &str, end: &str) -> String {
        let output = self
            .output
            .as_ref()
            .map(|o| format!(" -> {o}"))
            .unwrap_or_default();
        let params: Vec<String> = std::iter::once(self.receiver.to_string())
            .chain(self.params.iter().map(|(p, t)| format!("{prefix}{p}: {t}")))
            .collect();
        let line = format!("    fn {}({}){output}{end}", self.name, params.join(", "));
        if line.len() <= MAX_WIDTH {
            return line;
        }
        let params: String = params.iter().map(|p| format!("        {p},\n")).collect();
        format!("    fn {}(\n{params}    ){output}{end}", self.name)
    }
}

/// rustfmt's default line width, which the scaffold is laid out to.
const MAX_WIDTH: usize = 100;

/// A method name for `message`: its snake case, raw where it is a keyword.
fn method(message: &str) -> String {
    ident(&snake(message)).to_string()
}

/// What a decision returns for `emits`: the one event, a tuple of two to
/// four in declaration order, else the component's event enum, any number
/// of any of its events. The user may change the first two to any shape
/// angzarr-embedded takes (`Option<E>`, `Vec<E>`); the adapter takes them
/// all.
fn decided(component: &Component, emits: &[MessageDescriptor]) -> String {
    let events: Vec<&str> = emits.iter().map(|e| e.name()).collect();
    match events.as_slice() {
        [one] => one.to_string(),
        several if (2..=4).contains(&several.len()) => format!("({})", several.join(", ")),
        _ => format!("Vec<{}Event>", component.name),
    }
}

/// Whether [`decided`] is the event enum.
fn is_open(emits: &[MessageDescriptor]) -> bool {
    !(1..=4).contains(&emits.len())
}

fn interface_methods(
    component: &Component,
    pool: &[Component],
    translate: &[&Translate],
) -> Result<Vec<Method>, CodegenError> {
    let mut methods = Vec::new();
    if component.kind == ComponentKind::Policy {
        let target =
            owner(pool, &component.output_domain).ok_or_else(|| CodegenError::UnownedDomain {
                policy: component.anchor.full_name().to_string(),
                domain: component.output_domain.clone(),
            })?;
        for event in &component.events {
            methods.push(Method {
                name: method(event.name()),
                receiver: "&self",
                params: vec![
                    ("source".to_string(), "&[u8]".to_string()),
                    ("event".to_string(), format!("&{}", event.name())),
                ],
                output: Some(format!("Vec<(Vec<u8>, {}Command)>", target.name)),
            });
        }
        for selection in translate {
            let short = |full: &str| full.rsplit('.').next().unwrap_or(full).to_string();
            let (command, fact) = (short(&selection.command), short(&selection.fact));
            let mut params = vec![("command".to_string(), format!("&{command}"))];
            if let Some(refusal) =
                target
                    .input(&selection.command)
                    .and_then(|i| match i.refuses.as_slice() {
                        [one] => Some(one.name().to_string()),
                        _ => None,
                    })
            {
                params.push(("refusal".to_string(), format!("Option<{refusal}>")));
            }
            methods.push(Method {
                name: format!("{}_refused", snake(&command)),
                receiver: "&self",
                params,
                output: Some(fact),
            });
        }
        return Ok(methods);
    }
    for input in &component.inputs {
        let ty = input.message.name();
        let out = decided(component, &input.emits);
        if input.fact {
            methods.push(Method {
                name: method(ty),
                receiver: "&self",
                params: vec![("fact".to_string(), format!("&{ty}"))],
                output: Some(out),
            });
            continue;
        }
        let error = match input.refuses.as_slice() {
            [] => "Rejection".to_string(),
            [one] => one.name().to_string(),
            _ => {
                return Err(CodegenError::SeveralRefusals {
                    command: input.message.full_name().to_string(),
                });
            }
        };
        methods.push(Method {
            name: method(ty),
            receiver: "&self",
            params: vec![("cmd".to_string(), format!("&{ty}"))],
            output: Some(format!("Result<{out}, {error}>")),
        });
    }
    for event in &component.events {
        methods.push(Method {
            name: format!("apply_{}", snake(event.name())),
            receiver: "&mut self",
            params: vec![("event".to_string(), format!("&{}", event.name()))],
            output: None,
        });
    }
    Ok(methods)
}

fn interface_source(component: &Component, methods: &[Method]) -> String {
    let name = &component.name;
    let anchor = component.anchor.full_name();
    let mut out = if component.kind == ComponentKind::Policy {
        format!(
            "//! {name}: the interface of `{anchor}`, written once by\n\
             //! `angzarr scaffold rust` and yours from then on: `--missing` adds the\n\
             //! methods it lacks. Each reaction takes the source stream's root and the\n\
             //! event, and returns the commands to send, each with its target root; the\n\
             //! generated adapter (`crate::adapters::{name}`) carries the markup.\n\n"
        )
    } else {
        format!(
            "//! {name}: the interface of `{anchor}`, written once by\n\
             //! `angzarr scaffold rust` and yours from then on: `--missing` adds the\n\
             //! methods it lacks. It names only the domain types; the generated adapter\n\
             //! (`crate::adapters::{name}`) carries the markup and calls it.\n\n"
        )
    };
    out.push_str("use crate::domain::*;\n\n");
    out.push_str(&format!("pub trait {name} {{\n"));
    let body: Vec<String> = methods
        .iter()
        .map(|m| format!("{}\n", m.signature("", ";")))
        .collect();
    out.push_str(&body.join("\n"));
    out.push_str("}\n");
    out
}

fn impl_source(component: &Component, methods: &[Method]) -> String {
    let name = &component.name;
    let mut out = format!(
        "//! {name}Impl: the implementation of `{name}`, written once by\n\
         //! `angzarr scaffold rust` and yours from then on: `--missing` adds the\n\
         //! methods it lacks, each `todo!()`.\n\n"
    );
    let mut uses = [
        "use crate::domain::*;".to_string(),
        format!("use crate::{}::{name};", snake(name)),
    ];
    uses.sort();
    out.push_str(&uses.join("\n"));
    out.push_str("\n\n");
    if component.kind == ComponentKind::Policy {
        out.push_str(&format!(
            "#[derive(Clone, Debug, Default)]\npub struct {name}Impl;\n\n"
        ));
    } else {
        out.push_str(&format!(
            "/// The state the `apply_` methods fold.\n\
             #[derive(Clone, Debug, Default)]\n\
             pub struct {name}Impl {{\n    pub state: crate::wire::{},\n}}\n\n",
            component.anchor.name()
        ));
    }
    out.push_str(&format!("impl {name} for {name}Impl {{\n"));
    let body: Vec<String> = methods
        .iter()
        .map(|m| format!("{}\n        todo!()\n    }}\n", m.signature("_", " {")))
        .collect();
    out.push_str(&body.join("\n"));
    out.push_str("}\n");
    out
}

/// `tokens`, a whole file, formatted.
fn file(tokens: TokenStream) -> String {
    let file: syn::File = syn::parse2(tokens).expect("the emitter writes valid Rust");
    prettyplease::unparse(&file)
}

// ------------------------------------------------------------- --missing

/// `existing`, the user's file for `fresh`'s component, with the methods
/// of `fresh` that it lacks inserted into its trait (an interface) or its
/// trait impl (an implementation), and nothing else changed; `None` when it
/// lacks none. A method is present when the user's has one taking the same
/// message; one taking an input as `cmd` where the proto declares a fact
/// (`fact`), or the reverse, is refused, the file left as it is.
///
/// The methods are inserted in one place, before the closing brace; one
/// needing a type the file does not import yet compiles once the user adds
/// the import.
pub fn complete(fresh: &ScaffoldFile, existing: &str) -> Result<Option<String>, CodegenError> {
    // A Python project's files (and its pyproject.toml, which has no stubs
    // to complete).
    if !fresh.path.ends_with(".rs") {
        return crate::python::complete(fresh, existing);
    }
    let not_found = || CodegenError::ScaffoldImplNotFound {
        path: fresh.path.clone(),
        component: fresh.component.clone(),
    };
    let fresh_file = syn::parse_file(&fresh.source).expect("a scaffold parses");
    // A file with neither (the startup's build.rs and main.rs) has nothing
    // to complete.
    let Some(fresh_block) = block(&fresh_file, None) else {
        return Ok(None);
    };
    let user_file = syn::parse_file(existing).map_err(|_| not_found())?;
    let user_block = block(&user_file, Some(&fresh_block)).ok_or_else(not_found)?;

    let package = fresh
        .component
        .rsplit_once('.')
        .map_or("", |(package, _)| package);
    let mut missing = Vec::new();
    for method in &fresh_block.methods {
        match user_block.methods.iter().find(|m| m.ty == method.ty) {
            Some(found)
                if is_input(&found.param)
                    && is_input(&method.param)
                    && found.param != method.param =>
            {
                return Err(CodegenError::InputMismatch {
                    path: fresh.path.clone(),
                    message: format!("{package}.{}", method.ty),
                    expected: method.param.clone(),
                    found: found.param.clone(),
                });
            }
            Some(_) => {}
            None => missing.push(method),
        }
    }
    if missing.is_empty() {
        return Ok(None);
    }

    // The methods as the scaffold lays them out, a blank line before each.
    let mut stubs = String::new();
    for method in missing {
        let start = line_start(&fresh.source, method.lines.0);
        let end = line_start(&fresh.source, method.lines.1 + 1);
        stubs.push('\n');
        stubs.push_str(&fresh.source[start..end]);
    }

    let close = user_block.close;
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

/// The parameters that name an input.
fn is_input(param: &str) -> bool {
    param == "cmd" || param == "fact"
}

/// The trait of an interface, or the trait impl of an implementation.
struct Block {
    /// The trait's name, and for an impl its self type.
    key: (String, Option<String>),
    methods: Vec<BlockMethod>,
    close: proc_macro2::LineColumn,
}

/// One method of a [`Block`]: the message it takes (the last segment of
/// its first parameter's type that is not the source root) and the
/// parameter it takes it as (a leading `_` dropped), and its first and
/// last lines.
struct BlockMethod {
    ty: String,
    param: String,
    lines: (usize, usize),
}

/// The first trait or trait impl of `file` (with `like`, the one of the
/// same trait and self type).
fn block(file: &syn::File, like: Option<&Block>) -> Option<Block> {
    file.items.iter().find_map(|item| {
        let (key, sigs, close): (_, Vec<(&syn::Signature, usize, usize)>, _) = match item {
            syn::Item::Trait(t) => (
                (t.ident.to_string(), None),
                t.items
                    .iter()
                    .filter_map(|i| match i {
                        syn::TraitItem::Fn(f) => Some((
                            &f.sig,
                            first_line(&f.attrs, f.sig.fn_token.span),
                            last_line(f.semi_token.map(|s| s.span), f.default.as_ref()),
                        )),
                        _ => None,
                    })
                    .collect(),
                t.brace_token.span.close().start(),
            ),
            syn::Item::Impl(imp) => {
                let (path, _) = imp.trait_.as_ref()?;
                let ty = &imp.self_ty;
                (
                    (
                        path.segments.last()?.ident.to_string(),
                        Some(quote!(#ty).to_string()),
                    ),
                    imp.items
                        .iter()
                        .filter_map(|i| match i {
                            syn::ImplItem::Fn(f) => Some((
                                &f.sig,
                                first_line(&f.attrs, f.sig.fn_token.span),
                                f.block.brace_token.span.close().end().line,
                            )),
                            _ => None,
                        })
                        .collect(),
                    imp.brace_token.span.close().start(),
                )
            }
            _ => return None,
        };
        if let Some(like) = like
            && like.key != key
        {
            return None;
        }
        let methods = sigs
            .into_iter()
            .filter_map(|(sig, first, last)| {
                let (param, ty) = first_message(sig)?;
                Some(BlockMethod {
                    ty,
                    param,
                    lines: (first, last),
                })
            })
            .collect();
        Some(Block {
            key,
            methods,
            close,
        })
    })
}

fn first_line(attrs: &[syn::Attribute], fn_span: proc_macro2::Span) -> usize {
    attrs
        .first()
        .map_or(fn_span, |a| a.pound_token.span)
        .start()
        .line
}

fn last_line(semi: Option<proc_macro2::Span>, body: Option<&syn::Block>) -> usize {
    match (semi, body) {
        (_, Some(body)) => body.brace_token.span.close().end().line,
        (Some(semi), None) => semi.end().line,
        (None, None) => 0,
    }
}

/// A signature's first parameter naming a message, as `(param, type)`:
/// the source root (`&[u8]`) is passed over.
fn first_message(sig: &syn::Signature) -> Option<(String, String)> {
    sig.inputs.iter().find_map(|input| {
        let syn::FnArg::Typed(typed) = input else {
            return None;
        };
        let syn::Pat::Ident(pat) = &*typed.pat else {
            return None;
        };
        let mut ty = &*typed.ty;
        if let syn::Type::Reference(r) = ty {
            ty = &r.elem;
        }
        let syn::Type::Path(path) = ty else {
            return None;
        };
        let last = path.path.segments.last()?;
        let name = last.ident.to_string();
        // `Option<Refusal>` and the like are not what a method takes first.
        if !matches!(last.arguments, syn::PathArguments::None) {
            return None;
        }
        Some((
            pat.ident.to_string().trim_start_matches('_').to_string(),
            name,
        ))
    })
}

// -------------------------------------------------------- generated layer

/// The components of an application in `files`, with `remote` the domains
/// served by a child process: every aggregate and process manager, and
/// every policy whose two domains are owned among them.
fn application(components: &[Component]) -> Vec<&Component> {
    components
        .iter()
        .filter(|c| {
            c.kind != ComponentKind::Policy
                || (owner(components, &c.input_domain).is_some()
                    && owner(components, &c.output_domain).is_some())
        })
        .collect()
}

/// The generated adapters of every component declared in `files`, the
/// `remote` domains (served by a child process) as contracts: each
/// aggregate's, process manager's and policy's angzarr-embedded markup,
/// every handler delegating to the component's interface on its
/// implementation (`crate::<component>_impl::<Component>Impl`). A remote
/// domain's contract is its markup with no logic, so a policy can name it
/// as its source or target; it is never registered.
///
/// `interface` reads the user's interface of a policy (its path relative to
/// `src`, `buy_in_wallet.rs`): each `<command>_refused` method there is the
/// refusal switch set to translate that command, and gets its
/// `#[on_refused]` callback.
pub fn adapters(
    descriptor_set: &[u8],
    files: &[&str],
    remote: &[&str],
    interface: &dyn Fn(&str) -> Option<String>,
) -> Result<String, CodegenError> {
    let components = model::components(descriptor_set, files, MARKUP_KINDS)?;
    let next = next();
    let mut items = TokenStream::new();
    let mut markup = vec!["Context", "Emit"];
    let mut macros = vec![];
    let mut has_policy = false;
    for component in application(&components) {
        let is_remote = remote.contains(&component.domain.as_str());
        let tokens = match component.kind {
            ComponentKind::Policy => {
                has_policy = true;
                markup.extend(["Commands", "PolicyContext"]);
                macros.extend(["on_event", "policy"]);
                let source = interface(&format!("{}.rs", snake(&component.name)));
                let refused = source.as_deref().map(refused_methods).unwrap_or_default();
                if !refused.is_empty() {
                    markup.push("Refusal");
                    macros.push("on_refused");
                }
                policy_adapter(component, &components, &refused)
            }
            _ if is_remote => {
                macros.extend(["applies", "handles_cmd", "handles_fact"]);
                markup.push("Record");
                macros.push(if component.kind == ComponentKind::ProcessManager {
                    "process_manager"
                } else {
                    "aggregate"
                });
                contract(component)
            }
            _ => {
                macros.extend(["applies", "handles_cmd", "handles_fact"]);
                markup.extend(["Record", "Refuses"]);
                macros.push(if component.kind == ComponentKind::ProcessManager {
                    "process_manager"
                } else {
                    "aggregate"
                });
                domain_adapter(component)
            }
        };
        items.extend(tokens);
    }
    if has_policy {
        items.extend(quote! {
            /// The root of the stream a policy's trigger was recorded on.
            fn source_root(ctx: &PolicyContext) -> Vec<u8> {
                ctx.trigger()
                    .stream
                    .as_ref()
                    .and_then(|cover| cover.root.as_ref())
                    .map(|root| root.value.to_vec())
                    .unwrap_or_default()
            }
        });
    }
    markup.sort();
    markup.dedup();
    macros.sort();
    macros.dedup();
    let markup: Vec<syn::Ident> = markup.iter().map(|m| format_ident!("{m}")).collect();
    let macros: Vec<syn::Ident> = macros.iter().map(|m| format_ident!("{m}")).collect();
    let macros_crate = runtime_macros();
    let uuid = if has_policy {
        quote!(use #next::v1::Uuid;)
    } else {
        quote!()
    };
    let source = file(quote! {
        #[allow(unused_imports)]
        use #next::markup::{#(#markup),*};
        #uuid
        #[allow(unused_imports)]
        use ::#macros_crate::{#(#macros),*};
        use crate::domain::*;
        #items
    });
    Ok(format!(
        "// @generated by angzarr-codegen from {}. Do not edit.\n//\n\
         // The adapter layer: each component's angzarr-embedded markup, every\n\
         // handler delegating to the component's interface (the user's trait) on its\n\
         // implementation. Regenerated every build; the interface and implementation\n\
         // are the user's and are never touched.\n\n{source}",
        files.join(", ")
    ))
}

/// The paths of a component's interface trait and implementation type.
fn user_paths(component: &Component) -> (TokenStream, TokenStream) {
    let stem = format_ident!("{}", snake(&component.name));
    let stem_impl = format_ident!("{}_impl", snake(&component.name));
    let name = ident(&component.name);
    let name_impl = format_ident!("{}Impl", component.name);
    (
        quote!(crate::#stem::#name),
        quote!(crate::#stem_impl::#name_impl),
    )
}

/// What a decision's `Ok` becomes for the markup: `Emit::of` it, or, for
/// the component's event enum, one continuation step per event.
fn emitted(component: &Component, emits: &[MessageDescriptor], open: &str) -> TokenStream {
    let open = format_ident!("{open}");
    if !is_open(emits) {
        return quote!(#open::of);
    }
    let first = component
        .events
        .first()
        .map(|e| ident(e.name()))
        .unwrap_or_else(|| format_ident!("Rejection"));
    let enum_name = format_ident!("{}Event", component.name);
    let arms = component.events.iter().map(|e| {
        let v = ident(e.name());
        quote!(#enum_name::#v(event) => emit.then(move |_: &Self, _: &Context| Ok::<_, Rejection>(event)))
    });
    quote! {
        |events: Vec<#enum_name>| {
            events.into_iter().fold(#open::of(None::<#first>), |emit, event| match event {
                #(#arms,)*
            })
        }
    }
}

fn domain_adapter(component: &Component) -> TokenStream {
    let name = ident(&component.name);
    let (interface, implementation) = user_paths(component);
    let doc = format!(
        " `{}`, delegating to `{}`.",
        component.anchor.full_name(),
        interface.to_string().replace(' ', "")
    );
    let markup = if component.kind == ComponentKind::ProcessManager {
        format_ident!("process_manager")
    } else {
        format_ident!("aggregate")
    };
    let domain = &component.domain;
    let mut methods = Vec::new();
    for input in &component.inputs {
        let ty = ident(input.message.name());
        let m = format_ident!("{}", method(input.message.name()));
        if input.fact {
            let emit = emitted(component, &input.emits, "Record");
            let body = if is_open(&input.emits) {
                quote!((#emit)(#interface::#m(&self.0, &fact)))
            } else {
                quote!(Record::of(#interface::#m(&self.0, &fact)))
            };
            methods.push(quote! {
                #[handles_fact(#ty)]
                fn #m(&self, fact: #ty, _ctx: &Context) -> Record<Self> {
                    #body
                }
            });
            continue;
        }
        let starts = if input.starts {
            quote!(, starts)
        } else {
            quote!()
        };
        let emit = emitted(component, &input.emits, "Emit");
        match input.refuses.as_slice() {
            [] => methods.push(quote! {
                #[handles_cmd(#ty #starts)]
                fn #m(&self, cmd: #ty, _ctx: &Context) -> Result<Emit<Self>, Rejection> {
                    #interface::#m(&self.0, &cmd).map(#emit)
                }
            }),
            refusals => {
                let refusals: Vec<syn::Ident> = refusals.iter().map(|r| ident(r.name())).collect();
                methods.push(quote! {
                    #[handles_cmd(#ty #starts, refuses(#(#refusals),*))]
                    fn #m(&self, cmd: #ty, _ctx: &Context) -> Result<Emit<Self>, Refuses<Self, #ty>> {
                        #interface::#m(&self.0, &cmd).map(#emit).map_err(Into::into)
                    }
                });
            }
        }
    }
    for event in &component.events {
        let ty = ident(event.name());
        let m = format_ident!("{}", method(event.name()));
        let apply = format_ident!("apply_{}", snake(event.name()));
        methods.push(quote! {
            #[applies(#ty)]
            fn #m(&mut self, event: #ty) {
                #interface::#apply(&mut self.0, &event);
            }
        });
    }
    quote! {
        #[doc = #doc]
        #[derive(Clone, Debug, Default)]
        pub struct #name(pub #implementation);

        impl std::ops::Deref for #name {
            type Target = #implementation;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[#markup(domain = #domain)]
        impl #name {
            #(#methods)*
        }
    }
}

/// A remote domain's contract: its markup, every handler unreachable (the
/// child serves it), every applier a no-op.
fn contract(component: &Component) -> TokenStream {
    let name = ident(&component.name);
    let doc = format!(
        " `{}`, served by a child process: the contract a policy names, never \
         registered.",
        component.anchor.full_name()
    );
    let markup = if component.kind == ComponentKind::ProcessManager {
        format_ident!("process_manager")
    } else {
        format_ident!("aggregate")
    };
    let domain = &component.domain;
    let served = format!("the {domain} domain is served by its child process");
    let mut methods = Vec::new();
    for input in &component.inputs {
        let ty = ident(input.message.name());
        let m = format_ident!("{}", method(input.message.name()));
        if input.fact {
            methods.push(quote! {
                #[handles_fact(#ty)]
                fn #m(&self, _fact: #ty, _ctx: &Context) -> Record<Self> {
                    unreachable!(#served)
                }
            });
        } else {
            methods.push(quote! {
                #[handles_cmd(#ty)]
                fn #m(&self, _cmd: #ty, _ctx: &Context) -> Result<Emit<Self>, Rejection> {
                    unreachable!(#served)
                }
            });
        }
    }
    for event in &component.events {
        let ty = ident(event.name());
        let m = format_ident!("{}", method(event.name()));
        methods.push(quote! {
            #[applies(#ty)]
            fn #m(&mut self, _event: #ty) {}
        });
    }
    quote! {
        #[doc = #doc]
        #[derive(Clone, Debug, Default)]
        pub struct #name;

        #[#markup(domain = #domain)]
        impl #name {
            #(#methods)*
        }
    }
}

/// A `<command>_refused` method of a policy's interface: its name, the
/// command it takes, whether it takes the typed refusal, and the fact it
/// returns.
struct Refused {
    name: String,
    command: String,
    refusal: Option<String>,
    fact: String,
}

/// The `<command>_refused` methods of a policy's interface `source`.
fn refused_methods(source: &str) -> Vec<Refused> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let last = |ty: &syn::Type| -> Option<String> {
        let mut ty = ty;
        if let syn::Type::Reference(r) = ty {
            ty = &r.elem;
        }
        let syn::Type::Path(p) = ty else { return None };
        let seg = p.path.segments.last()?;
        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            && let syn::Type::Path(inner) = inner
        {
            return Some(inner.path.segments.last()?.ident.to_string());
        }
        Some(seg.ident.to_string())
    };
    let mut out = Vec::new();
    for item in &file.items {
        let syn::Item::Trait(t) = item else { continue };
        for item in &t.items {
            let syn::TraitItem::Fn(f) = item else {
                continue;
            };
            let name = f.sig.ident.to_string();
            if !name.ends_with("_refused") {
                continue;
            }
            let typed: Vec<&syn::Type> = f
                .sig
                .inputs
                .iter()
                .filter_map(|i| match i {
                    syn::FnArg::Typed(t) => Some(&*t.ty),
                    _ => None,
                })
                .collect();
            let syn::ReturnType::Type(_, output) = &f.sig.output else {
                continue;
            };
            let (Some(command), Some(fact)) = (typed.first().and_then(|t| last(t)), last(output))
            else {
                continue;
            };
            out.push(Refused {
                name,
                command,
                refusal: typed.get(1).and_then(|t| last(t)),
                fact,
            });
        }
        break;
    }
    out
}

fn policy_adapter(
    component: &Component,
    components: &[Component],
    refused: &[Refused],
) -> TokenStream {
    let name = ident(&component.name);
    let (interface, implementation) = user_paths(component);
    let doc = format!(
        " `{}`, delegating to `{}`.",
        component.anchor.full_name(),
        interface.to_string().replace(' ', "")
    );
    let (Some(source), Some(target)) = (
        owner(components, &component.input_domain),
        owner(components, &component.output_domain),
    ) else {
        return quote!();
    };
    let policy_name = format!(
        "policy-{}-{}",
        component.input_domain, component.output_domain
    );
    let source_ty = ident(&source.name);
    let target_ty = ident(&target.name);
    let command_enum = format_ident!("{}Command", target.name);
    let arms: Vec<TokenStream> = target
        .inputs
        .iter()
        .filter(|i| !i.fact)
        .map(|i| {
            let v = ident(i.message.name());
            quote!(#command_enum::#v(command) => commands.to(root, command))
        })
        .collect();
    let methods = component.events.iter().map(|event| {
        let ty = ident(event.name());
        let m = format_ident!("{}", method(event.name()));
        quote! {
            #[on_event(#ty)]
            fn #m(&self, event: &#ty, ctx: &PolicyContext) -> Commands<#target_ty> {
                let orders = #interface::#m(&self.0, &source_root(ctx), event);
                orders
                    .into_iter()
                    .fold(Commands::none(), |commands, (root, command)| {
                        let root = Uuid { value: root.into() };
                        match command {
                            #(#arms,)*
                        }
                    })
            }
        }
    });
    let callbacks = refused.iter().map(|r| {
        let m = format_ident!("{}", r.name);
        let command = ident(&r.command);
        let fact = ident(&r.fact);
        let args = match &r.refusal {
            Some(refusal) => {
                let refusal = ident(refusal);
                quote!(refusal.command(), refusal.refusal::<#refusal>())
            }
            None => quote!(refusal.command()),
        };
        quote! {
            #[on_refused(#command)]
            fn #m(&self, refusal: &Refusal<#command>) -> #fact {
                #interface::#m(&self.0, #args)
            }
        }
    });
    quote! {
        #[doc = #doc]
        #[derive(Clone, Debug, Default)]
        pub struct #name(pub #implementation);

        #[policy(name = #policy_name, source = #source_ty, target = #target_ty)]
        impl #name {
            #(#methods)*
            #(#callbacks)*
        }
    }
}

/// The generated `register(&mut Registry)`: every adapter of the
/// application declared in `files` registered with the runtime, the
/// `remote` domains (served by a child process) left to the host.
pub fn register(
    descriptor_set: &[u8],
    files: &[&str],
    remote: &[&str],
) -> Result<String, CodegenError> {
    let components = model::components(descriptor_set, files, MARKUP_KINDS)?;
    let registrations = application(&components)
        .into_iter()
        .filter(|c| c.kind == ComponentKind::Policy || !remote.contains(&c.domain.as_str()))
        .map(|component| {
            let name = ident(&component.name);
            let ty = quote!(crate::adapters::#name);
            match component.kind {
                ComponentKind::Aggregate => quote!(registry.aggregate::<#ty>(#ty::default);),
                ComponentKind::ProcessManager => {
                    quote!(registry.process_manager::<#ty>(#ty::default);)
                }
                ComponentKind::Policy => quote!(registry.policy(#ty::default());),
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
