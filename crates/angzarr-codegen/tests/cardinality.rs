//! Emits cardinality (decision 14.50): the handler's result is typed from
//! `CommandOptions.emits_cardinality`, ONE an event or a tuple, OPTIONAL
//! that or nothing, MANY any number of the component's events. The kinds
//! fixture (`kinds/v1/kinds.proto`) has an input of each; its scaffold and
//! adapters are pinned by `tests/refusals.rs`'s golden files.

mod common;

use angzarr_codegen::{CodegenError, adapters, markup_domain, scaffold};
use common::*;

const KINDS: &str = "kinds/v1/kinds.proto";

fn interface() -> String {
    scaffold(&descriptors(&[KINDS]), &[KINDS], &[])
        .unwrap()
        .into_iter()
        .find(|f| f.path == "ledger_state.rs")
        .expect("ledger_state.rs")
        .source
}

#[test]
fn test_each_cardinality_types_the_handlers_result() {
    let source = interface();
    for expected in [
        "fn open(&self, cmd: &Open, ctx: &Context) -> Result<(Posted, Noted), Failed<Rejection>>;",
        "    ) -> Result<(Posted, Noted, Cleared, Closed), Failed<Rejection>>;",
        "fn maybe(&self, cmd: &Maybe, ctx: &Context) -> Result<Option<Posted>, Failed<Rejection>>;",
        "    ) -> Result<Option<(Posted, Noted)>, Failed<Rejection>>;",
        "    ) -> Result<Vec<LedgerStateEvent>, Failed<PostRefused>>;",
        "fn raw(&self, cmd: &Raw, ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>>;",
        "fn arrived(&self, fact: &Arrived, ctx: &Context) -> Option<(Posted, Noted)>;",
        "fn swept(&self, fact: &Swept, ctx: &Context) -> Vec<LedgerStateEvent>;",
    ] {
        assert!(source.contains(expected), "{expected}:\n{source}");
    }
}

#[test]
fn test_the_adapter_hands_the_markup_each_cardinality() {
    let source = adapters(&descriptors(&[KINDS]), &[KINDS], &[], &|_| None).unwrap();
    let flat = source.replace([' ', '\n'], "");
    for expected in [
        // ONE and OPTIONAL of one type: the markup takes them as they are.
        "LedgerState::open(&self.0,&cmd,ctx).map(Emit::of)",
        "LedgerState::maybe(&self.0,&cmd,ctx).map(Emit::of)",
        // OPTIONAL of several: the tuple, or no event.
        "|events:Option<(Posted,Noted)>|matchevents{Some(events)=>Emit::of(events),None=>Emit::of(None::<Posted>),}",
        "|events:Option<(Posted,Noted)>|matchevents{Some(events)=>Record::of(events),None=>Record::of(None::<Posted>),}",
        // MANY: a step per event, a command's returning its own failure.
        "Ok::<_,Failed<Refuses<Self,Batch>>,>(event)",
        "Ok::<_,Failed<Rejection>,>(event)",
        "LedgerStateEvent::Noted(event)=>{emit.then(move|_:&Self,_:&Context|event)}",
        // A fact's closure is bound, then called: never called where it is
        // declared (clippy's redundant_closure_call).
        "letrecord=|events:Option<(Posted,Noted)>|",
        "record(crate::ledger_state::LedgerState::arrived(&self.0,&fact,ctx))",
    ] {
        assert!(flat.contains(expected), "{expected}:\n{source}");
    }
}

fn refusal(file: &str) -> CodegenError {
    markup_domain(&descriptors(&[file]), &[file], &[("bad.v1", "crate::wire")])
        .expect_err("refused")
}

#[test]
fn test_one_or_optional_without_emits_is_refused() {
    assert_eq!(
        refusal("bad/v1/one_without_emits.proto"),
        CodegenError::CardinalityWithoutEmits {
            message: "bad.v1.Poke".to_string(),
            optional: false,
        }
    );
    assert_eq!(
        refusal("bad/v1/optional_without_emits.proto"),
        CodegenError::CardinalityWithoutEmits {
            message: "bad.v1.Poke".to_string(),
            optional: true,
        }
    );
}

#[test]
fn test_a_tuple_wider_than_the_markups_is_refused() {
    assert_eq!(
        refusal("bad/v1/wide_tuple.proto"),
        CodegenError::EmitsTuple {
            message: "bad.v1.Poke".to_string(),
            count: 5,
        }
    );
}
