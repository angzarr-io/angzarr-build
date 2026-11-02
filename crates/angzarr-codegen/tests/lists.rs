//! Every handling returns a list (decision 14.75; C-0797): a command's or
//! a fact's handler returns any number of its component's events, none
//! included, whatever its emits. The kinds fixture (`kinds/v1/kinds.proto`)
//! has commands and facts of one, two and four emits types, and one of
//! none; its domain layer (its interfaces in it), scaffold and adapters are
//! pinned by `tests/refusals.rs`'s golden files.

mod common;

use angzarr_codegen::{adapters, markup_domain};
use common::*;

const KINDS: &str = "kinds/v1/kinds.proto";

/// The kinds fixture's domain layer, the ledger's interface in it.
fn interface() -> String {
    markup_domain(
        &descriptors(&[KINDS]),
        &[KINDS],
        &[("kinds.v1", "crate::wire")],
    )
    .unwrap()
    .remove(0)
    .source
}

#[test]
fn test_every_handler_returns_a_list_of_its_components_events() {
    let source = interface();
    for expected in [
        "fn open(&self, cmd: &Open, ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>>;",
        "fn four(&self, cmd: &Four, ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>>;",
        "fn maybe(&self, cmd: &Maybe, ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>>;",
        "    ) -> Result<Vec<LedgerStateEvent>, Failed<PostRefused>>;",
        "fn raw(&self, cmd: &Raw, ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>>;",
        "fn arrived(&self, fact: &Arrived, ctx: &Context) -> Vec<LedgerStateEvent>;",
        "fn swept(&self, fact: &Swept, ctx: &Context) -> Vec<LedgerStateEvent>;",
    ] {
        assert!(source.contains(expected), "{expected}:\n{source}");
    }
    for shape in [
        "-> Option<",
        "-> Result<Option<",
        "-> Result<(",
        "-> (",
        "Result<Posted,",
    ] {
        assert!(!source.contains(shape), "{shape} in:\n{source}");
    }
}

#[test]
fn test_the_adapter_hands_the_markup_a_step_per_event() {
    let source = adapters(&descriptors(&[KINDS]), &[KINDS], &[]).unwrap();
    let flat = source.replace([' ', '\n'], "");
    for expected in [
        // A command's step returns its own failure.
        "Ok::<_,Failed<Refuses<Self,Batch>>,>(event)",
        "Ok::<_,Failed<Rejection>,>(event)",
        "LedgerStateEvent::Noted(event)=>{emit.then(move|_:&Self,_:&Context|event)}",
        // An empty list is no event.
        "events.into_iter().fold(Emit::of(None::<Posted>),",
        "events.into_iter().fold(Record::of(None::<Posted>),",
        // A fact's closure is bound, then called: never called where it is
        // declared (clippy's redundant_closure_call).
        "letrecord=|events:Vec<LedgerStateEvent>|",
        "record(crate::domain::LedgerState::arrived(&self.0,&fact,ctx))",
    ] {
        assert!(flat.contains(expected), "{expected}:\n{source}");
    }
    assert!(!flat.contains(".map(Emit::of)"), "{source}");
}
