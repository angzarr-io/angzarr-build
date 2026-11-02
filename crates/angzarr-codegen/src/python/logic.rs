//! The logic interface (`logic.py`, a `typing.Protocol` over the plain
//! types) and its implementation (`logic_impl.py`, every method a TODO):
//! the user's files, written once.

use std::collections::BTreeSet;

use prost_reflect::MessageDescriptor;

use crate::common::snake;
use crate::model::{Component, ComponentKind, Decided, Input};

use super::layout::{head, wrap};
use super::{PythonApp, Refusable, Unit};

/// One method of the logic interface.
struct Method {
    name: String,
    /// A coroutine: `async def`.
    asynchronous: bool,
    /// `(param, type)`, after `self`.
    params: Vec<(String, String)>,
    returns: String,
    todo: String,
}

/// The interface's name: a domain's, camel-cased, then `Logic` (`player`
/// as `PlayerLogic`); a policy's or a projector's code name, then `Logic`
/// (`LedgerProjectorLogic`).
pub(super) fn logic_name(app: &PythonApp, unit: &Unit) -> String {
    if !unit.is_domain() {
        return format!("{}Logic", unit.component.name);
    }
    let camel: String = app
        .domain
        .split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars
                .next()
                .map(|c| c.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect();
    format!("{camel}Logic")
}

/// What a decision of `input` returns: its events by its cardinality (one
/// event, a tuple of several, that or None, or a list of any of them; an
/// input declaring no emits, a list of any of `component`'s events, None
/// when it has none), or one of its typed refusals.
fn decided(component: &Component, input: &Input) -> String {
    let names = |events: &[MessageDescriptor], by: &str| {
        let names: Vec<&str> = events.iter().map(|e| e.name()).collect();
        names.join(by)
    };
    let events = match input.decided(component) {
        Decided::Many([]) => "None".to_string(),
        Decided::Many(events) => format!("list[{}]", names(events, " | ")),
        Decided::Optional([one]) => format!("{} | None", one.name()),
        Decided::Optional(events) => format!("tuple[{}] | None", names(events, ", ")),
        Decided::One([one]) => one.name().to_string(),
        Decided::One(events) => format!("tuple[{}]", names(events, ", ")),
    };
    std::iter::once(events)
        .chain(input.refuses.iter().map(|r| r.name().to_string()))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// A refusal method's typed refusal parameter, when `refusable` refuses
/// with types: `refusal`, one of them or None.
fn refusal_param(refusable: &Refusable) -> Option<(String, String)> {
    if refusable.refuses.is_empty() {
        return None;
    }
    let types: Vec<&str> = refusable.refuses.iter().map(|r| r.name()).collect();
    Some((
        "refusal".to_string(),
        format!("{} | None", types.join(" | ")),
    ))
}

/// What a domain's handler of a refusal returns, as a fact handler with no
/// emits does: a list of any of its events, None when it has none.
fn any_events(component: &Component) -> String {
    let any: Vec<&str> = component.events.iter().map(|e| e.name()).collect();
    if any.is_empty() {
        "None".to_string()
    } else {
        format!("list[{}]", any.join(" | "))
    }
}

/// The interface's methods, by kind. A domain's: a decision per command
/// (or fact) from the state and its context (the harness's
/// `CommandContext`, re-exported by `domain.py`), returning what it emits
/// or a typed refusal, a handler per command its policies send that
/// declares a `refuses` type, of that command's pass-through refusal
/// (`on_refused_<command>`: the command, its
/// typed refusal or None, the state and the context, returning the events
/// recording it) unless the refusal switch translates it, and a fold per
/// event from the state to the next state. A policy's: a reaction per event
/// from the event, its source root and its page's context, returning the
/// target's commands and facts, each with its root, and a translating callback per
/// command the refusal switch selects (`<command>_refused`: the command, its
/// typed refusal or None and the refused command's context, returning the
/// fact). A projector's: a coroutine per event from the projection, the
/// event and its page's context, returning the next projection. All of it
/// in the plain types (`domain.py`).
fn methods(unit: &Unit) -> Vec<Method> {
    match unit.kind() {
        ComponentKind::Aggregate | ComponentKind::ProcessManager => domain_methods(unit),
        ComponentKind::Policy => policy_methods(unit),
        ComponentKind::Projector => projector_methods(unit),
    }
}

/// A `(name, type)` parameter.
fn param(name: &str, ty: &str) -> (String, String) {
    (name.to_string(), ty.to_string())
}

/// An aggregate's or a process manager's methods (see [`methods`]).
fn domain_methods(unit: &Unit) -> Vec<Method> {
    let component = &unit.component;
    let state = component.anchor.name();
    let decisions = component.inputs.iter().map(|input| {
        let ty = input.message.name();
        let (param_name, todo) = if input.fact {
            ("fact", "record")
        } else {
            ("cmd", "decide")
        };
        Method {
            name: snake(ty),
            asynchronous: false,
            params: vec![
                param(param_name, ty),
                param("state", state),
                param("ctx", "CommandContext"),
            ],
            returns: decided(component, input),
            todo: format!("{todo} {ty}"),
        }
    });
    let refusals = unit
        .refusable
        .iter()
        .filter(|r| unit.translation(&r.command).is_none())
        .map(|refusable| {
            let ty = refusable.command.name();
            let mut params = vec![param("command", ty)];
            params.extend(refusal_param(refusable));
            params.push(param("state", state));
            params.push(param("ctx", "CommandContext"));
            Method {
                name: format!("on_refused_{}", snake(ty)),
                asynchronous: false,
                params,
                returns: any_events(component),
                todo: format!("record the refusal of {ty}"),
            }
        });
    let folds = component.events.iter().map(|event| {
        let ty = event.name();
        Method {
            name: format!("apply_{}", snake(ty)),
            asynchronous: false,
            params: vec![param("state", state), param("event", ty)],
            returns: state.to_string(),
            todo: format!("fold {ty} into the state"),
        }
    });
    decisions.chain(refusals).chain(folds).collect()
}

/// A policy's methods (see [`methods`]).
fn policy_methods(unit: &Unit) -> Vec<Method> {
    let commands: Vec<&str> = unit
        .commands
        .iter()
        .chain(&unit.sent_facts)
        .map(|c| c.name())
        .collect();
    let command = match commands.as_slice() {
        [] => "object".to_string(),
        _ => commands.join(" | "),
    };
    let reactions = unit.component.events.iter().map(|event| {
        let ty = event.name();
        Method {
            name: snake(ty),
            asynchronous: false,
            params: vec![
                param("event", ty),
                param("source", "bytes"),
                param("ctx", "PageContext"),
            ],
            returns: format!("list[tuple[bytes, {command}]]"),
            todo: format!("react to {ty}"),
        }
    });
    let translations = unit.refusable.iter().filter_map(|refusable| {
        let fact = unit.translation(&refusable.command)?;
        let ty = refusable.command.name();
        let mut params = vec![param("command", ty)];
        params.extend(refusal_param(refusable));
        params.push(param("ctx", "CommandContext"));
        Some(Method {
            name: format!("{}_refused", snake(ty)),
            asynchronous: false,
            params,
            returns: fact.name().to_string(),
            todo: format!("translate the refusal of {ty}"),
        })
    });
    reactions.chain(translations).collect()
}

/// A projector's methods (see [`methods`]).
fn projector_methods(unit: &Unit) -> Vec<Method> {
    let projection = unit.component.anchor.name();
    let projections = unit.component.events.iter().map(|event| {
        let ty = event.name();
        Method {
            name: format!("project_{}", snake(ty)),
            asynchronous: true,
            params: vec![
                param("projection", projection),
                param("event", ty),
                param("ctx", "PageContext"),
            ],
            returns: projection.to_string(),
            todo: format!("project {ty}"),
        }
    });
    projections.collect()
}

/// The import of the plain types the interface names.
fn interface_imports(app: &PythonApp, unit: &Unit) -> String {
    let component = &unit.component;
    let mut names = BTreeSet::new();
    if component.kind != ComponentKind::Policy {
        names.insert(component.anchor.name().to_string());
    }
    if unit.is_domain() {
        if !component.inputs.is_empty() {
            names.insert("CommandContext".to_string());
        }
    } else if !component.events.is_empty() {
        names.insert("PageContext".to_string());
    }
    for input in &component.inputs {
        names.insert(input.message.name().to_string());
        names.extend(input.emits.iter().map(|e| e.name().to_string()));
        names.extend(input.refuses.iter().map(|r| r.name().to_string()));
    }
    names.extend(component.events.iter().map(|e| e.name().to_string()));
    if !component.events.is_empty() {
        names.extend(
            unit.commands
                .iter()
                .chain(&unit.sent_facts)
                .map(|c| c.name().to_string()),
        );
    }
    // A refusal method's: the refused command, its typed refusals, the
    // refused command's context and, translated, the fact.
    for refusable in &unit.refusable {
        let translation = unit.translation(&refusable.command);
        if unit.is_domain() == translation.is_some() {
            continue;
        }
        names.insert(refusable.command.name().to_string());
        names.extend(refusable.refuses.iter().map(|r| r.name().to_string()));
        names.insert("CommandContext".to_string());
        names.extend(translation.map(|fact| fact.name().to_string()));
    }
    let mut out = format!("from {}.domain import (\n", app.package());
    for name in names {
        out.push_str(&format!("    {name},\n"));
    }
    out.push_str(")\n");
    out
}

/// `def name(self, a: A, b: B) -> R:` laid out as black lays it out at
/// `indent`, followed by `tail` (` ...` or nothing).
fn signature(indent: usize, method: &Method, tail: &str) -> String {
    let params: Vec<String> = std::iter::once("self".to_string())
        .chain(method.params.iter().map(|(p, t)| format!("{p}: {t}")))
        .collect();
    let def = if method.asynchronous {
        "async def"
    } else {
        "def"
    };
    head(
        indent,
        &format!("{def} {}", method.name),
        &params,
        &method.returns,
        tail,
    )
}

/// What the logic does, for the interface's doc, by kind.
fn duties(unit: &Unit) -> &'static str {
    match unit.kind() {
        ComponentKind::Aggregate | ComponentKind::ProcessManager => {
            "each decision its ``CommandContext`` too, returning its events or one of \
             its typed refusals, or raising ``Rejection`` to refuse its command \
             (or ``Bug`` for a defect),"
        }
        ComponentKind::Policy => {
            "each reaction its event's source root and ``PageContext`` too, returning \
             what it sends, each with its target root: a command, or a fact its target \
             declares, which the generated dispatch sends in the arm the target \
             declares (raising ``Bug`` for a defect),"
        }
        ComponentKind::Projector => {
            "each projection a coroutine, its event's ``PageContext`` too (the source \
             domain is its cover's), returning the next projection (raising ``Bug`` \
             for a defect),"
        }
    }
}

pub(super) fn interface(app: &PythonApp, unit: &Unit) -> String {
    let component = &unit.component;
    let name = logic_name(app, unit);
    let owner = match unit.kind() {
        ComponentKind::Aggregate | ComponentKind::ProcessManager => {
            format!("the {} domain's logic", app.domain)
        }
        ComponentKind::Policy => format!("the {} policy's logic", unit.name),
        ComponentKind::Projector => format!("the {} projector's logic", unit.name),
    };
    let doc = wrap(&format!(
        "\"\"\"{name}: {owner}, for ``{}``. Written once by \
         ``angzarr scaffold python`` and yours from then on: ``--missing`` adds the methods \
         it lacks. It takes and returns the plain types (:mod:`.domain`), {} \
         and imports nothing of angzarr directly; the generated dispatch \
         (``_gen/.../{}_angzarr.py``) calls it.",
        component.anchor.full_name(),
        duties(unit),
        snake(&component.name),
    ));
    let mut out = format!(
        "{doc}\n\"\"\"\n\nfrom __future__ import annotations\n\nfrom typing import Protocol\n\n"
    );
    out.push_str(&interface_imports(app, unit));
    out.push_str(&format!("\n\nclass {name}(Protocol):\n"));
    let body: Vec<String> = methods(unit)
        .iter()
        .map(|m| format!("{}\n", signature(4, m, " ...")))
        .collect();
    out.push_str(&body.join("\n"));
    out
}

pub(super) fn implementation(app: &PythonApp, unit: &Unit) -> String {
    let name = logic_name(app, unit);
    let mut out = format!(
        "\"\"\"{name}Impl: the implementation of ``{name}``, written once by\n\
         ``angzarr scaffold python`` and yours from then on: ``--missing`` adds the\n\
         methods it lacks, each a TODO.\n\"\"\"\n\n\
         from __future__ import annotations\n\n"
    );
    out.push_str(&interface_imports(app, unit));
    out.push_str(&format!(
        "from {}.logic import {name}\n\n\nclass {name}Impl({name}):\n",
        app.package(),
    ));
    let body: Vec<String> = methods(unit).iter().map(stub).collect();
    out.push_str(&body.join("\n"));
    out
}

/// An implementation method: its signature and a TODO body.
fn stub(method: &Method) -> String {
    format!(
        "{}\n        raise NotImplementedError  # TODO: {}\n",
        signature(4, method, ""),
        method.todo
    )
}
