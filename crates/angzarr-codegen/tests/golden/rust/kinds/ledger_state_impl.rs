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
    fn post(&self, _cmd: &Post, _ctx: &Context) -> Result<Posted, Failed<PostRefuses>> {
        todo!()
    }

    fn open(&self, _cmd: &Open, _ctx: &Context) -> Result<(Posted, Noted), Failed<Rejection>> {
        todo!()
    }

    fn four(
        &self,
        _cmd: &Four,
        _ctx: &Context,
    ) -> Result<(Posted, Noted, Cleared, Closed), Failed<Rejection>> {
        todo!()
    }

    fn maybe(&self, _cmd: &Maybe, _ctx: &Context) -> Result<Option<Posted>, Failed<Rejection>> {
        todo!()
    }

    fn maybe_both(
        &self,
        _cmd: &MaybeBoth,
        _ctx: &Context,
    ) -> Result<Option<(Posted, Noted)>, Failed<Rejection>> {
        todo!()
    }

    fn batch(
        &self,
        _cmd: &Batch,
        _ctx: &Context,
    ) -> Result<Vec<LedgerStateEvent>, Failed<PostRefused>> {
        todo!()
    }

    fn raw(&self, _cmd: &Raw, _ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>> {
        todo!()
    }

    fn arrived(&self, _fact: &Arrived, _ctx: &Context) -> Option<(Posted, Noted)> {
        todo!()
    }

    fn swept(&self, _fact: &Swept, _ctx: &Context) -> Vec<LedgerStateEvent> {
        todo!()
    }

    fn apply_posted(&mut self, _event: &Posted) {
        todo!()
    }

    fn apply_noted(&mut self, _event: &Noted) {
        todo!()
    }

    fn apply_cleared(&mut self, _event: &Cleared) {
        todo!()
    }

    fn apply_closed(&mut self, _event: &Closed) {
        todo!()
    }
}
