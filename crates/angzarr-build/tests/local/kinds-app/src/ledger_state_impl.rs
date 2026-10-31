//! LedgerStateImpl: the implementation of `LedgerState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::domain::*;
use crate::ledger_state::LedgerState;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct LedgerStateImpl {
    pub state: crate::wire::LedgerState,
}

impl LedgerState for LedgerStateImpl {
    /// Always refused: over the limit, with every kind of arg set.
    fn post(&self, _cmd: &Post, _ctx: &Context) -> Result<Posted, Failed<PostRefuses>> {
        Err(Failed::rejected(PostRefused {
            reason: PostRefusedReason::OverLimit,
            small: -1,
            big: 2,
            count: 3,
            total: 4,
            text: "t".to_string(),
            data: vec![0xab, 0x01],
            flag: true,
            ratio: 0.5,
            share: 1.5,
            tone: Tone::Loud,
            limit: Some(9),
            maybe: None,
            note: Note {
                text: "n".to_string(),
            },
            list: vec![1],
            table: Default::default(),
        }))
    }

    fn open(&self, _cmd: &Open, _ctx: &Context) -> Result<(Posted, Noted), Failed<Rejection>> {
        Ok((Posted {}, Noted {}))
    }

    fn four(
        &self,
        _cmd: &Four,
        _ctx: &Context,
    ) -> Result<(Posted, Noted, Cleared, Closed), Failed<Rejection>> {
        Ok((Posted {}, Noted {}, Cleared {}, Closed {}))
    }

    fn maybe(&self, _cmd: &Maybe, _ctx: &Context) -> Result<Option<Posted>, Failed<Rejection>> {
        Ok(None)
    }

    fn maybe_both(
        &self,
        _cmd: &MaybeBoth,
        _ctx: &Context,
    ) -> Result<Option<(Posted, Noted)>, Failed<Rejection>> {
        Ok(Some((Posted {}, Noted {})))
    }

    fn batch(
        &self,
        _cmd: &Batch,
        _ctx: &Context,
    ) -> Result<Vec<LedgerStateEvent>, Failed<PostRefused>> {
        Ok(vec![Noted {}.into(), Posted {}.into()])
    }

    fn raw(&self, _cmd: &Raw, _ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>> {
        Ok(Vec::new())
    }

    fn arrived(&self, _fact: &Arrived, _ctx: &Context) -> Option<(Posted, Noted)> {
        None
    }

    fn swept(&self, _fact: &Swept, _ctx: &Context) -> Vec<LedgerStateEvent> {
        vec![Closed {}.into()]
    }

    fn apply_posted(&mut self, _event: &Posted) {}

    fn apply_noted(&mut self, _event: &Noted) {}

    fn apply_cleared(&mut self, _event: &Cleared) {}

    fn apply_closed(&mut self, _event: &Closed) {}
}
