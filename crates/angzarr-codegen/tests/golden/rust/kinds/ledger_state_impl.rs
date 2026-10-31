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

    fn apply_posted(&mut self, _event: &Posted) {
        todo!()
    }
}
