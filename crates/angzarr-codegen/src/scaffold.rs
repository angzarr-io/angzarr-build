//! What a project's protos give an angzarr-embedded application besides the
//! domain layer, split so the user's logic never names the runtime:
//!
//! - the user's, written once into the source tree by `angzarr scaffold
//!   rust` and completed with `--missing`: per component an interface (a
//!   trait over the domain types) and its implementation (every body
//!   `todo!()`), neither importing anything of angzarr-embedded directly:
//!   their context types, `Failed`, `BusinessStatus`, `RetryableStatus`
//!   and `Bug` are the generated domain layer's re-exports and its own
//!   (`Failed::retryable_bug(status, &bug)` and `Failed::immediate(&bug)`
//!   included);
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
use quote::{format_ident, quote};

use crate::common::{has_code_status, refusal_choice_name, snake};
use crate::error::CodegenError;
use crate::model::{self, ALL_KINDS, Cardinality, Component, ComponentKind, Input};
use crate::rust::{RUNTIME, ident, next, runtime_macros};
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
    let components = model::components(descriptor_set, files, ALL_KINDS)?;
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
    Ok(crate::headed_files(out))
}

/// `selection`'s policy is a policy, its command one its target decides,
/// and its fact a fact its source ingests.
pub(crate) fn check_translate(
    components: &[Component],
    selection: &Translate,
) -> Result<(), CodegenError> {
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
    /// An async method's output as its implementation's `async fn` returns
    /// it, the interface's `output` being the future's
    /// (`impl Future<Output = ..> + Send`).
    awaited: Option<String>,
}

impl Method {
    /// The signature as rustfmt lays it out at an indent of 4: on one line
    /// when it fits in 100 columns, else one parameter a line. `prefix`
    /// prefixes the parameters (`_` in an implementation, which writes an
    /// async method as an `async fn`: `implementation`).
    fn signature(&self, prefix: &str, end: &str, implementation: bool) -> String {
        let (asyncness, output) = match (&self.awaited, implementation) {
            (Some(awaited), true) => ("async ", Some(awaited)),
            _ => ("", self.output.as_ref()),
        };
        let output = output.map(|o| format!(" -> {o}")).unwrap_or_default();
        let params: Vec<String> = std::iter::once(self.receiver.to_string())
            .chain(self.params.iter().map(|(p, t)| format!("{prefix}{p}: {t}")))
            .collect();
        let line = format!(
            "    {asyncness}fn {}({}){output}{end}",
            self.name,
            params.join(", ")
        );
        if line.len() <= MAX_WIDTH {
            return line;
        }
        let params: String = params.iter().map(|p| format!("        {p},\n")).collect();
        format!(
            "    {asyncness}fn {}(\n{params}    ){output}{end}",
            self.name
        )
    }
}

/// rustfmt's default line width, which the scaffold is laid out to.
const MAX_WIDTH: usize = 100;

/// A method name for `message`: its snake case, raw where it is a keyword.
fn method(message: &str) -> String {
    ident(&snake(message)).to_string()
}

/// What a decision returns for `input` (decision 14.50): ONE its one event,
/// or a tuple of one of each type in order; OPTIONAL that or `None`; MANY
/// any number of the component's events, as its event enum.
fn decided(component: &Component, input: &Input) -> String {
    let one = match input.emits.as_slice() {
        [single] => single.name().to_string(),
        several => {
            let names: Vec<&str> = several.iter().map(|e| e.name()).collect();
            format!("({})", names.join(", "))
        }
    };
    match input.cardinality {
        Cardinality::One => one,
        Cardinality::Optional => format!("Option<{one}>"),
        Cardinality::Many => format!("Vec<{}Event>", component.name),
    }
}

fn interface_methods(
    component: &Component,
    pool: &[Component],
    translate: &[&Translate],
) -> Result<Vec<Method>, CodegenError> {
    let mut methods = Vec::new();
    if component.kind == ComponentKind::Projector {
        for event in &component.events {
            methods.push(Method {
                name: method(event.name()),
                receiver: "&mut self",
                params: vec![
                    ("event".to_string(), format!("&{}", event.name())),
                    ("ctx".to_string(), "&ProjectionContext".to_string()),
                ],
                output: Some(
                    "impl std::future::Future<Output = Result<(), Bug>> + Send".to_string(),
                ),
                awaited: Some("Result<(), Bug>".to_string()),
            });
        }
        return Ok(methods);
    }
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
                    ("ctx".to_string(), "&PolicyContext".to_string()),
                ],
                output: Some(format!(
                    "Result<Vec<(Vec<u8>, {}Command)>, Bug>",
                    target.name
                )),
                awaited: None,
            });
        }
        for selection in translate {
            let short = |full: &str| full.rsplit('.').next().unwrap_or(full).to_string();
            let (command, fact) = (short(&selection.command), short(&selection.fact));
            let mut params = vec![("command".to_string(), format!("&{command}"))];
            // The typed refusal: its one type, or the choice of several.
            if let Some(refusal) =
                target
                    .input(&selection.command)
                    .and_then(|i| match i.refuses.as_slice() {
                        [] => None,
                        [one] => Some(one.name().to_string()),
                        _ => Some(refusal_choice_name(&i.message)),
                    })
            {
                params.push(("refusal".to_string(), format!("Option<{refusal}>")));
            }
            params.push(("ctx".to_string(), "&CommandContext".to_string()));
            methods.push(Method {
                name: format!("{}_refused", snake(&command)),
                receiver: "&self",
                params,
                output: Some(format!("Result<{fact}, Bug>")),
                awaited: None,
            });
        }
        return Ok(methods);
    }
    for input in &component.inputs {
        let ty = input.message.name();
        let out = decided(component, input);
        if input.fact {
            methods.push(Method {
                name: method(ty),
                receiver: "&self",
                params: vec![
                    ("fact".to_string(), format!("&{ty}")),
                    ("ctx".to_string(), "&Context".to_string()),
                ],
                output: Some(out),
                awaited: None,
            });
            continue;
        }
        let error = match input.refuses.as_slice() {
            [] => "Failed<Rejection>".to_string(),
            [one] => format!("Failed<{}>", one.name()),
            _ => format!("Failed<{}>", refusal_choice_name(&input.message)),
        };
        methods.push(Method {
            name: method(ty),
            receiver: "&self",
            params: vec![
                ("cmd".to_string(), format!("&{ty}")),
                ("ctx".to_string(), "&Context".to_string()),
            ],
            output: Some(format!("Result<{out}, {error}>")),
            awaited: None,
        });
    }
    for event in &component.events {
        methods.push(Method {
            name: format!("apply_{}", snake(event.name())),
            receiver: "&mut self",
            params: vec![("event".to_string(), format!("&{}", event.name()))],
            output: None,
            awaited: None,
        });
    }
    Ok(methods)
}

fn interface_source(component: &Component, methods: &[Method]) -> String {
    let name = &component.name;
    let anchor = component.anchor.full_name();
    let mut out = if component.kind == ComponentKind::Projector {
        format!(
            "//! {name}: the interface of `{anchor}`, written once by\n\
             //! `angzarr scaffold rust` and yours from then on: `--missing` adds the\n\
             //! methods it lacks. One async method per projected event, of any of its\n\
             //! source domains (`ctx.stream()` says which), returning its `Bug` when\n\
             //! the projection fails. The generated adapter\n\
             //! (`crate::adapters::{name}`) carries the markup.\n\n"
        )
    } else if component.kind == ComponentKind::Policy {
        format!(
            "//! {name}: the interface of `{anchor}`, written once by\n\
             //! `angzarr scaffold rust` and yours from then on: `--missing` adds the\n\
             //! methods it lacks. Each reaction takes the source stream's root, the\n\
             //! event and its context, and returns the commands to send, each with its\n\
             //! target root, or its `Bug`: a defect, since a policy reacts to events the\n\
             //! application's own domains recorded. The generated adapter\n\
             //! (`crate::adapters::{name}`) carries the markup.\n\n"
        )
    } else {
        format!(
            "//! {name}: the interface of `{anchor}`, written once by\n\
             //! `angzarr scaffold rust` and yours from then on: `--missing` adds the\n\
             //! methods it lacks. It names only `crate::domain`'s types, its handlers'\n\
             //! `Context` among them; the generated adapter (`crate::adapters::{name}`)\n\
             //! carries the markup and calls it.\n\n"
        )
    };
    out.push_str("use crate::domain::*;\n\n");
    out.push_str(&format!("pub trait {name} {{\n"));
    let body: Vec<String> = methods
        .iter()
        .map(|m| format!("{}\n", m.signature("", ";", false)))
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
    } else if component.kind == ComponentKind::Projector {
        out.push_str(&format!(
            "/// The projection the methods update.\n\
             #[derive(Clone, Debug, Default)]\n\
             pub struct {name}Impl {{\n    pub state: crate::wire::{},\n}}\n\n",
            component.anchor.name()
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
        .map(|m| {
            format!(
                "{}\n        todo!()\n    }}\n",
                m.signature("_", " {", true)
            )
        })
        .collect();
    out.push_str(&body.join("\n"));
    out.push_str("}\n");
    out
}

/// `tokens`, a whole file, formatted.
pub(crate) fn file(tokens: TokenStream) -> String {
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
    // A file with neither has nothing to complete.
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

/// The generated adapters of every component declared in `files`: each
/// aggregate's, process manager's, policy's and projector's
/// angzarr-embedded markup, every handler delegating to the component's
/// interface on its implementation
/// (`crate::<component>_impl::<Component>Impl`). What a child process
/// serves is in `remote`, a domain by its stream domain, a policy or a
/// projector by its name in the application. A remote domain has no markup
/// here: it is its command set (`CommandTarget`) and its event set
/// (`EventSource`), so a policy can name it as its source or target; it is
/// never registered. A remote policy or projector has nothing here.
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
    let components = model::components(descriptor_set, files, ALL_KINDS)?;
    let next = next();
    let mut items = TokenStream::new();
    let mut markup = vec!["Context", "Emit"];
    let mut macros = vec![];
    let mut has_policy = false;
    for component in application(&components) {
        if component.kind != ComponentKind::Aggregate
            && component.kind != ComponentKind::ProcessManager
            && remote.contains(&component.component_name.as_str())
        {
            continue;
        }
        let is_remote = remote.contains(&component.domain.as_str());
        let tokens = match component.kind {
            ComponentKind::Projector => {
                markup.push("Report");
                macros.extend(["projector", "projects"]);
                projector_adapter(component)
            }
            ComponentKind::Policy => {
                has_policy = true;
                markup.extend(["Commands", "PolicyContext"]);
                macros.extend(["on_event", "policy"]);
                let source = interface(&format!("{}.rs", snake(&component.name)));
                let refused = source.as_deref().map(refused_methods).unwrap_or_default();
                if !refused.is_empty() {
                    markup.extend(["CommandContext", "Refusal"]);
                    macros.push("on_refused");
                }
                policy_adapter(component, &components, &refused)
            }
            _ if is_remote => {
                markup.extend(["CommandTarget", "EventSource"]);
                if component.inputs.iter().any(|i| !i.fact) {
                    markup.push("Decides");
                }
                if component.inputs.iter().any(|i| i.fact) {
                    markup.push("Ingests");
                }
                remote_domain(component)
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
    markup.sort();
    markup.dedup();
    macros.sort();
    macros.dedup();
    let markup: Vec<syn::Ident> = markup.iter().map(|m| format_ident!("{m}")).collect();
    let macros: Vec<syn::Ident> = macros.iter().map(|m| format_ident!("{m}")).collect();
    let macros_crate = runtime_macros();
    // A remote domain alone takes no macro: no import of none.
    let macros = if macros.is_empty() {
        quote!()
    } else {
        quote! {
            #[allow(unused_imports)]
            use ::#macros_crate::{#(#macros),*};
        }
    };
    let uuid = if has_policy {
        quote!(use #next::v1::Uuid;)
    } else {
        quote!()
    };
    let source = file(quote! {
        #[allow(unused_imports)]
        use #next::markup::{#(#markup),*};
        #uuid
        #macros
        use crate::domain::*;
        #items
    });
    Ok(crate::headed(
        ".rs",
        format!(
            "// @generated by angzarr-codegen from {}. Do not edit.\n//\n\
         // The adapter layer: each component's angzarr-embedded markup, every\n\
         // handler delegating to the component's interface (the user's trait) on its\n\
         // implementation. Regenerated every build; the interface and implementation\n\
         // are the user's and are never touched. A domain's handlers fail with the\n\
         // generated layer's `Failed`, whose size is not the application's to\n\
         // choose: its impl allows `clippy::result_large_err`.\n\n{source}",
            files.join(", ")
        ),
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

/// What a decision's `Ok` becomes for the markup, `Emit` for a command and
/// `Record` for a fact: ONE `of` it (the markup takes an event or a tuple
/// as is); OPTIONAL of one type the same (it takes an `Option`), of
/// several the tuple or no event; MANY one continuation step per event of
/// the event enum, each returning it (a command's step as `Ok` of the
/// handler's own failure, `failed`).
/// Whether the markup takes `input`'s decision as is, by `of`: ONE (an
/// event or a tuple), or OPTIONAL of one type (an `Option`).
fn is_of(input: &Input) -> bool {
    input.cardinality == Cardinality::One
        || (input.cardinality == Cardinality::Optional && input.emits.len() == 1)
}

fn emitted(component: &Component, input: &Input, failed: &TokenStream) -> TokenStream {
    let open = if input.fact {
        format_ident!("Record")
    } else {
        format_ident!("Emit")
    };
    let first = component
        .events
        .first()
        .map(|e| ident(e.name()))
        .unwrap_or_else(|| format_ident!("Rejection"));
    if is_of(input) {
        return quote!(#open::of);
    }
    match input.cardinality {
        // OPTIONAL of several types: the tuple, or no event.
        Cardinality::One | Cardinality::Optional => {
            let ty: syn::Type =
                syn::parse_str(&decided(component, input)).expect("a decided type parses");
            quote! {
                |events: #ty| match events {
                Some(events) => #open::of(events),
                    None => #open::of(None::<#first>),
                }
            }
        }
        Cardinality::Many => {
            let enum_name = format_ident!("{}Event", component.name);
            let arms = component.events.iter().map(|e| {
                let v = ident(e.name());
                if input.fact {
                    quote!(#enum_name::#v(event) => emit.then(move |_: &Self, _: &Context| event))
                } else {
                    quote!(#enum_name::#v(event) => emit.then(move |_: &Self, _: &Context| Ok::<_, #failed>(event)))
                }
            });
            quote! {
                |events: Vec<#enum_name>| {
                    events.into_iter().fold(#open::of(None::<#first>), |emit, event| match event {
                        #(#arms,)*
                    })
                }
            }
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
            let emit = emitted(component, input, &quote!());
            let call = quote!(#interface::#m(&self.0, &fact, ctx));
            let body = if is_of(input) {
                quote!(Record::of(#call))
            } else {
                // Bound first: a closure called where it is declared is
                // clippy's `redundant_closure_call`.
                quote! {
                    let record = #emit;
                    record(#call)
                }
            };
            methods.push(quote! {
                #[handles_fact(#ty)]
                fn #m(&self, fact: #ty, ctx: &Context) -> Record<Self> {
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
        match input.refuses.as_slice() {
            [] => {
                let emit = emitted(component, input, &quote!(Failed<Rejection>));
                methods.push(quote! {
                    #[handles_cmd(#ty #starts)]
                    fn #m(&self, cmd: #ty, ctx: &Context) -> Result<Emit<Self>, Failed<Rejection>> {
                        #interface::#m(&self.0, &cmd, ctx).map(#emit)
                    }
                });
            }
            refusals => {
                let emit = emitted(component, input, &quote!(Failed<Refuses<Self, #ty>>));
                // One type widens by its own `Into`; several, by their
                // domain-layer choice's `widen`. A refusal with a Reason is
                // refused under its code's status first (decision 14.52).
                let (refusal, widen) = match refusals {
                    [one] => (ident(one.name()), quote!(Failed::widen)),
                    _ => {
                        let choice = format_ident!("{}", refusal_choice_name(&input.message));
                        (choice.clone(), quote!(#choice::widen))
                    }
                };
                let widen = if has_code_status(input) {
                    quote!(map_err(#refusal::with_code_status).map_err(#widen))
                } else {
                    quote!(map_err(#widen))
                };
                let refusals: Vec<syn::Ident> = refusals.iter().map(|r| ident(r.name())).collect();
                methods.push(quote! {
                    #[handles_cmd(#ty #starts, refuses(#(#refusals),*))]
                    fn #m(&self, cmd: #ty, ctx: &Context) -> Result<Emit<Self>, Failed<Refuses<Self, #ty>>> {
                        #interface::#m(&self.0, &cmd, ctx).map(#emit).#widen
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
        #[allow(clippy::result_large_err)]
        impl #name {
            #(#methods)*
        }
    }
}

/// The stand-in of every aggregate and process manager declared in
/// `files` served by a child process, generated from its proto: what a
/// Rust policy names it by, its command set (`CommandTarget`, `Decides` of
/// each command) and its event set (`EventSource`, `Ingests` of each fact),
/// for a crate that hand-writes its policies and generates no adapter of
/// the domain. Its domain types are `crate::domain`'s.
pub fn stand_ins(descriptor_set: &[u8], files: &[&str]) -> Result<String, CodegenError> {
    let _ = model::components(descriptor_set, files, ALL_KINDS)?;
    Ok(String::new())
}

/// A remote domain (served by a child process) as a policy names it: its
/// command set (`CommandTarget`, `Decides` of each command) and its
/// event set (`EventSource`, `Ingests` of each fact), each type by its full
/// name. No aggregate stands in for it.
fn remote_domain(component: &Component) -> TokenStream {
    let name = ident(&component.name);
    let doc = format!(
        " `{}`, served by a child process: its commands and events, as a \
         policy names it; never registered.",
        component.anchor.full_name()
    );
    let domain = &component.domain;
    let full = |m: &prost_reflect::MessageDescriptor| m.full_name().to_string();
    let commands: Vec<String> = component
        .inputs
        .iter()
        .filter(|i| !i.fact)
        .map(|i| full(&i.message))
        .collect();
    let events: Vec<String> = component.events.iter().map(full).collect();
    let decides = component.inputs.iter().filter(|i| !i.fact).map(|i| {
        let ty = ident(i.message.name());
        quote!(impl Decides<#ty> for #name {})
    });
    let ingests = component.inputs.iter().filter(|i| i.fact).map(|i| {
        let ty = ident(i.message.name());
        quote!(impl Ingests<#ty> for #name {})
    });
    quote! {
        #[doc = #doc]
        #[derive(Clone, Debug, Default)]
        pub struct #name;

        impl CommandTarget for #name {
            const DOMAIN: &'static str = #domain;

            fn command_types() -> Vec<String> {
                vec![#(#commands.to_string()),*]
            }
        }

        #(#decides)*

        impl EventSource for #name {
            const DOMAIN: &'static str = #domain;

            fn event_types() -> Vec<String> {
                vec![#(#events.to_string()),*]
            }
        }

        #(#ingests)*
    }
}

/// A `<command>_refused` method of a policy's interface: its name, the
/// command it takes, whether it takes the typed refusal (beside the
/// refused command's context, which it always takes), and the fact its
/// `Result` answers.
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
                refusal: typed
                    .get(1)
                    .and_then(|t| last(t))
                    .filter(|t| t != "CommandContext"),
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
    let policy_name = &component.component_name;
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
            fn #m(
                &self,
                event: &#ty,
                ctx: &PolicyContext,
            ) -> ::core::result::Result<Commands<#target_ty>, Bug> {
                let orders = #interface::#m(&self.0, &ctx.source_root().value, event, ctx)?;
                ::core::result::Result::Ok(orders
                    .into_iter()
                    .fold(Commands::none(), |commands, (root, command)| {
                        let root = Uuid { value: root.into() };
                        match command {
                            #(#arms,)*
                        }
                    }))
            }
        }
    });
    let callbacks = refused.iter().map(|r| {
        let m = format_ident!("{}", r.name);
        let command = ident(&r.command);
        let fact = ident(&r.fact);
        // A command refusing several types: their choice, each type tried
        // in turn.
        let several = target
            .inputs
            .iter()
            .find(|i| i.message.name() == r.command)
            .filter(|i| i.refuses.len() > 1);
        let args = match (&r.refusal, several) {
            (Some(_), Some(input)) => {
                let choice = format_ident!("{}", refusal_choice_name(&input.message));
                let types: Vec<syn::Ident> =
                    input.refuses.iter().map(|t| ident(t.name())).collect();
                let (first, rest) = (&types[0], &types[1..]);
                quote! {
                    refusal.command(),
                    refusal.refusal::<#first>().map(#choice::#first)
                        #(.or_else(|| refusal.refusal::<#rest>().map(#choice::#rest)))*
                }
            }
            (Some(refusal), None) => {
                let refusal = ident(refusal);
                quote!(refusal.command(), refusal.refusal::<#refusal>())
            }
            (None, _) => quote!(refusal.command()),
        };
        quote! {
            #[on_refused(#command)]
            fn #m(
                &self,
                refusal: &Refusal<#command>,
                ctx: &CommandContext,
            ) -> ::core::result::Result<#fact, Bug> {
                #interface::#m(&self.0, #args, ctx)
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

fn projector_adapter(component: &Component) -> TokenStream {
    let name = ident(&component.name);
    let (interface, implementation) = user_paths(component);
    let doc = format!(
        " `{}`, delegating to `{}`.",
        component.anchor.full_name(),
        interface.to_string().replace(' ', "")
    );
    let projector_name = &component.component_name;
    let sources = &component.sources;
    let methods = component.events.iter().map(|event| {
        let ty = ident(event.name());
        let m = format_ident!("{}", method(event.name()));
        quote! {
            #[projects(#ty)]
            async fn #m(&mut self, event: &#ty, ctx: &ProjectionContext) -> Result<(), Report> {
                #interface::#m(&mut self.0, event, ctx)
                    .await
                    .map_err(|bug| Report::of(&bug))
            }
        }
    });
    quote! {
        #[doc = #doc]
        #[derive(Clone, Debug, Default)]
        pub struct #name(pub #implementation);

        #[projector(name = #projector_name, sources = [#(#sources),*])]
        impl #name {
            #(#methods)*
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
    registration(descriptor_set, files, &|c: &Component| {
        c.kind == ComponentKind::Policy || !remote.contains(&c.domain.as_str())
    })
}

/// The generated `register(&mut Registry)` of the components of `files`
/// named in `names` alone (by [`crate::component_name`]): what an
/// application's host config lists in process.
pub fn register_only(
    descriptor_set: &[u8],
    files: &[&str],
    names: &[&str],
) -> Result<String, CodegenError> {
    registration(descriptor_set, files, &|c: &Component| {
        names.contains(&c.component_name.as_str())
    })
}

fn registration(
    descriptor_set: &[u8],
    files: &[&str],
    keep: &dyn Fn(&Component) -> bool,
) -> Result<String, CodegenError> {
    let components = model::components(descriptor_set, files, ALL_KINDS)?;
    let registrations = application(&components)
        .into_iter()
        .filter(|c| keep(c))
        .map(|component| {
            let name = ident(&component.name);
            let ty = quote!(crate::adapters::#name);
            match component.kind {
                ComponentKind::Aggregate => quote!(registry.aggregate::<#ty>(#ty::default);),
                ComponentKind::ProcessManager => {
                    quote!(registry.process_manager::<#ty>(#ty::default);)
                }
                ComponentKind::Policy => quote!(registry.policy(#ty::default());),
                ComponentKind::Projector => quote!(registry.projector(#ty::default());),
            }
        })
        .collect::<Vec<_>>();
    // Every domain remote leaves the body empty: the registry is still
    // taken, for one signature, and bound so the build warns of nothing.
    let unused = registrations.is_empty().then(|| quote!(let _ = registry;));
    let next = next();
    let doc = [
        format!(" Registers every component of the project with {RUNTIME}, which dispatches"),
        " each through angzarr-router: an application calls this once, at startup.".to_string(),
    ];
    let source = file(quote! {
        #(#[doc = #doc])*
        pub fn register(registry: &mut #next::Registry) {
            #unused
            #(#registrations)*
        }
    });
    Ok(crate::headed(
        ".rs",
        format!(
            "// @generated by angzarr-codegen from {}. Do not edit.\n\n{source}",
            files.join(", ")
        ),
    ))
}

/// The domain layer for marked-up components: the domain types, their wire
/// conversions and each one's `DomainMessage` impl, and nothing the markup
/// generates itself (no payload enums, no `State` impl).
///
/// The wire types at each package's `wire` path are prost's, built with
/// angzarr's settings (`angzarr_build::wire`): a `bytes` field as
/// `Vec<u8>` (prost's default), a map as a `BTreeMap`
/// (`btree_map(["."])`), an enum field as its `i32`, a message field as an
/// `Option`, `prost::Name` on every message, and angzarr's own types
/// angzarr-proto's. The layer checks the first two where it is compiled: a
/// prost crate built otherwise fails with `angzarr-codegen-wire-bytes` or
/// `angzarr-codegen-wire-map` at the field.
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
