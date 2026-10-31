//! The dispatch (`_gen/.../<component>_angzarr.py`): the router binding's
//! wire calls converted to calls on the logic, by kind.

use std::collections::BTreeSet;

use prost_reflect::MessageDescriptor;

use crate::common::snake;
use crate::model::ComponentKind;

use super::convert::{converter_pair, converters, pb2_class, table};
use super::layout::{call, head, wrap};
use super::logic::logic_name;
use super::{GENERATED, PythonApp, Unit, alias, pb2_module, reached};

/// The dispatch's table type, by kind: the client's public dispatch.
fn dispatch_type(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Aggregate => "AggregateDispatch",
        ComponentKind::ProcessManager => "ProcessManagerDispatch",
        ComponentKind::Policy => "PolicyDispatch",
        ComponentKind::Projector => "ProjectorDispatch",
    }
}

/// What a dispatch does, for its doc, by kind.
fn dispatch_doc(unit: &Unit, package: &str, logic: &str) -> String {
    let name = &unit.component.name;
    let what = match unit.kind() {
        ComponentKind::Aggregate | ComponentKind::ProcessManager => {
            "and converts what it returns back: a decision's events (one, an iterable, \
             or None) into the event book, or its typed refusal into the client's \
             ``Refused``, a fact's flags beside it into its record, a fold's next state \
             into the state."
        }
        ComponentKind::Policy => {
            "and converts what it returns back: each command, with its target root, \
             into a command book to the policy's target domain."
        }
        ComponentKind::Projector => {
            "runs its coroutine on the dispatch's event loop, and converts the \
             projection it returns back."
        }
    };
    let doc = wrap(&format!(
        "\"\"\"The {name}'s dispatch for the router binding, the client's public \
         ``{}``: each entry decodes the wire payload, converts it to the plain types \
         (:mod:`{package}.domain`), calls ``{logic}``, {what} A hand-written dispatch \
         built on the same public API may take its place.",
        dispatch_type(unit.kind()),
    ));
    format!("{doc}\n\"\"\"\n")
}

/// The events a domain's decisions return, on the wire.
const WIRE_EVENTS: &str = "\n\ndef _wire(result) -> list:\n    \
    if result is None:\n        events = ()\n    \
    elif dataclasses.is_dataclass(result):\n        events = (result,)\n    \
    else:\n        events = result\n    \
    return [_EVENTS[type(event)](event) for event in events]\n\n\n\
    def _book(result) -> _t.EventBook:\n    \
    book = _t.EventBook()\n    \
    for wire in _wire(result):\n        \
    book.pages.add().payload.CopyFrom(_az.pack(wire))\n    \
    return book\n";

/// A decision's result refused, when it is a typed refusal: under its
/// code's status (decision 14.52), the client's `Refused` being
/// FAILED_PRECONDITION unless its public `grpc` says otherwise.
const DECIDED: &str = "\n\ndef _decided(result) -> _t.EventBook:\n    \
    to_wire = _REFUSALS.get(type(result))\n    \
    if to_wire is not None:\n        \
    refused = _az.Refused(to_wire(result))\n        \
    refused.grpc = result.status()\n        \
    raise refused\n    \
    return _book(result)\n";

/// The event loop a projector's coroutines run on.
const EVENT_LOOP: &str = "\n\n# The logic's coroutines run on one event loop, on a thread of its own\n\
    # started with the first: every call shares it, so what the logic opens on\n\
    # it (a connection, a client) stays usable from one event to the next.\n\
    _LOOP = None\n_LOOP_LOCK = threading.Lock()\n\n\n\
    def _run(coroutine):\n    \
    \"\"\"``coroutine`` run to its end on the dispatch's event loop.\"\"\"\n    \
    global _LOOP\n    \
    with _LOOP_LOCK:\n        \
    if _LOOP is None:\n            \
    _LOOP = asyncio.new_event_loop()\n            \
    threading.Thread(\n                \
    target=_LOOP.run_forever, name=\"angzarr-logic\", daemon=True\n            \
    ).start()\n    \
    return asyncio.run_coroutine_threadsafe(coroutine, _LOOP).result()\n";

pub(super) fn dispatch(app: &PythonApp, unit: &Unit) -> String {
    let component = &unit.component;
    let stem = snake(&component.name);
    let package = app.package();
    let logic = logic_name(app, unit);
    let (messages, _) = reached(unit);
    let mut modules = BTreeSet::new();
    for m in &messages {
        modules.insert((pb2_module(app, m), alias(m)));
    }
    let stdlib = match unit.kind() {
        ComponentKind::Aggregate | ComponentKind::ProcessManager => "import dataclasses\n\n",
        ComponentKind::Policy => "",
        ComponentKind::Projector => "import asyncio\nimport threading\n\n",
    };
    let types = if unit.kind() == ComponentKind::Projector {
        ""
    } else {
        "from angzarr_client.proto.io.angzarr.v1 import types_pb2 as _t\n"
    };
    let mut out = format!(
        "{GENERATED}{}\nfrom __future__ import annotations\n\n\
         {stdlib}import angzarr_client.router as _az\n{types}\n\
         from {package} import domain as _d\n",
        dispatch_doc(unit, &package, &logic),
    );
    for (module, alias) in &modules {
        let (parent, leaf) = module.rsplit_once('.').unwrap_or(("", module));
        out.push_str(&format!("from {parent} import {leaf} as {alias}\n"));
    }
    out.push_str(&format!("from {package}.logic import {logic}\n"));
    out.push_str(
        "\n\ndef _decode(message, payload):\n    try:\n        message.ParseFromString(payload.value)\n    except Exception as exc:\n        raise _az.any_decode_error(payload.type_url, exc)\n    return message\n",
    );
    for m in &messages {
        out.push_str(&converter_pair(m));
    }
    let returns = format!("_az.{}", dispatch_type(unit.kind()));
    let new = head(
        0,
        &format!("def new_{stem}_dispatch"),
        &[format!("logic: {logic}")],
        &returns,
        "",
    );
    match unit.kind() {
        ComponentKind::Aggregate | ComponentKind::ProcessManager => {
            domain_dispatch(&mut out, unit, &new)
        }
        ComponentKind::Policy => policy_dispatch(&mut out, unit, &new),
        ComponentKind::Projector => projector_dispatch(&mut out, unit, &new),
    }
    out.push_str("    return dispatch\n");
    out
}

/// An aggregate's or a process manager's dispatch: its rebuilder of folds,
/// a decision per command, a record per fact.
fn domain_dispatch(out: &mut String, unit: &Unit, new: &str) {
    let component = &unit.component;
    out.push_str(&table(
        "Each event's plain type, and its converter to the wire.",
        "_EVENTS",
        &component.events,
    ));
    out.push_str(WIRE_EVENTS);
    let refusals: Vec<MessageDescriptor> = component
        .inputs
        .iter()
        .flat_map(|i| i.refuses.iter().cloned())
        .fold(Vec::new(), |mut all, r| {
            if !all.contains(&r) {
                all.push(r);
            }
            all
        });
    if !refusals.is_empty() {
        out.push_str(&table(
            "Each typed refusal's plain type, and its converter to the wire.",
            "_REFUSALS",
            &refusals,
        ));
        out.push_str(DECIDED);
    }
    let (state_to, state_from) = converters(&component.anchor);
    out.push_str(&format!(
        "\n\n{new}\n    rebuilder = _az.Rebuilder({})\n",
        pb2_class(&component.anchor)
    ));
    for event in &component.events {
        let method = format!("apply_{}", snake(event.name()));
        let (event_to, _) = converters(event);
        out.push_str(&format!(
            "\n    def _{method}(state, payload, ctx):\n        \
             event = {event_to}(_decode({}(), payload))\n        \
             folded = logic.{method}({state_to}(state), event)\n        \
             state.CopyFrom({state_from}(folded))\n\n",
            pb2_class(event),
        ));
        out.push_str(&call(
            4,
            "rebuilder.apply_with_context",
            &format!("\"{}\", _{method}", event.full_name()),
        ));
    }
    out.push('\n');
    out.push_str(&call(
        4,
        &format!("dispatch = _az.{}", dispatch_type(component.kind)),
        &format!("\"{}\", \"{}\", rebuilder", unit.name, component.domain),
    ));
    for input in &component.inputs {
        let method = snake(input.message.name());
        let (input_to, _) = converters(&input.message);
        let class = pb2_class(&input.message);
        if input.fact {
            out.push_str(&format!(
                "\n    def _on_{method}(payload, state, cctx):\n        \
                 wire = _decode({class}(), payload)\n        \
                 flags = logic.{method}({input_to}(wire), {state_to}(state), cctx)\n        \
                 return _az.FactRecord(wire, tuple(_wire(flags)))\n\n",
            ));
            out.push_str(&call(
                4,
                "dispatch.on_fact",
                &format!("\"{}\", _on_{method}", input.message.full_name()),
            ));
            continue;
        }
        let answer = if input.refuses.is_empty() {
            "_book"
        } else {
            "_decided"
        };
        out.push_str(&format!(
            "\n    def _on_{method}(payload, state, cctx):\n        \
             cmd = {input_to}(_decode({class}(), payload))\n        \
             return {answer}(logic.{method}(cmd, {state_to}(state), cctx))\n\n",
        ));
        out.push_str(&call(
            4,
            "dispatch.on_command",
            &format!("\"{}\", _on_{method}", input.message.full_name()),
        ));
    }
}

/// A policy's dispatch: a reaction per event, its commands addressed to
/// the target domain.
fn policy_dispatch(out: &mut String, unit: &Unit, new: &str) {
    let component = &unit.component;
    out.push_str(&table(
        "Each command's plain type, and its converter to the wire.",
        "_COMMANDS",
        &unit.commands,
    ));
    out.push_str(&format!(
        "\n\ndef _commands(result) -> list:\n    \
         \"\"\"A reaction's commands, each to its root of the {target} domain.\"\"\"\n    \
         books = []\n    \
         for root, command in result or ():\n        \
         book = _t.CommandBook(cover=_t.Cover(domain=\"{target}\"))\n        \
         book.cover.root.value = root\n        \
         wire = _COMMANDS[type(command)](command)\n        \
         book.page.command.payload.CopyFrom(_az.pack(wire))\n        \
         books.append(book)\n    \
         return books\n",
        target = component.output_domain,
    ));
    out.push_str(&format!("\n\n{new}\n"));
    out.push_str(&call(
        4,
        "dispatch = _az.PolicyDispatch",
        &format!(
            "\"{}\", \"{}\", \"{}\"",
            unit.name, component.input_domain, component.output_domain
        ),
    ));
    for event in &component.events {
        let method = snake(event.name());
        let (event_to, _) = converters(event);
        out.push_str(&format!(
            "\n    def _{method}(payload, ctx):\n        \
             event = {event_to}(_decode({}(), payload))\n        \
             source = ctx.cover.root.value if ctx.cover is not None else b\"\"\n        \
             return _commands(logic.{method}(event, source, ctx))\n\n",
            pb2_class(event),
        ));
        out.push_str(&call(
            4,
            "dispatch.on_event_with_context",
            &format!("\"{}\", _{method}", event.full_name()),
        ));
    }
}

/// A projector's dispatch: its sources, and a projection per event, each
/// the logic's coroutine run on the dispatch's event loop.
fn projector_dispatch(out: &mut String, unit: &Unit, new: &str) {
    let component = &unit.component;
    out.push_str(EVENT_LOOP);
    let (projection_to, projection_from) = converters(&component.anchor);
    out.push_str(&format!("\n\n{new}\n"));
    out.push_str(&call(
        4,
        "dispatch = _az.ProjectorDispatch",
        &format!("\"{}\", {}", unit.name, pb2_class(&component.anchor)),
    ));
    let sources: Vec<String> = unit.sources.iter().map(|s| format!("{s:?}")).collect();
    out.push_str(&call(4, "dispatch.for_domains", &sources.join(", ")));
    for event in &component.events {
        let method = format!("project_{}", snake(event.name()));
        let (event_to, _) = converters(event);
        out.push_str(&format!(
            "\n    def _{method}(projection, payload, ctx):\n        \
             event = {event_to}(_decode({}(), payload))\n",
            pb2_class(event),
        ));
        out.push_str(&call(
            8,
            "projected = _run",
            &format!("logic.{method}({projection_to}(projection), event, ctx)"),
        ));
        out.push_str(&format!(
            "        projection.CopyFrom({projection_from}(projected))\n\n"
        ));
        out.push_str(&call(
            4,
            "dispatch.on_event_with_context",
            &format!("\"{}\", _{method}", event.full_name()),
        ));
    }
}
