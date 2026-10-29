//! BuyInImpl: the implementation of `BuyIn`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::buy_in::BuyIn;
use crate::domain::*;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct BuyInImpl {
    pub state: crate::wire::BuyInState,
}

impl BuyIn for BuyInImpl {
    fn start_buy_in(
        &self,
        _cmd: &StartBuyIn,
        _ctx: &Context,
    ) -> Result<BuyInStarted, Failed<Rejection>> {
        todo!()
    }

    fn record_held(
        &self,
        _cmd: &RecordHeld,
        _ctx: &Context,
    ) -> Result<BuyInHeld, Failed<Rejection>> {
        todo!()
    }

    fn hold_lost(&self, _fact: &HoldLost, _ctx: &Context) -> BuyInFailed {
        todo!()
    }

    fn apply_buy_in_started(&mut self, _event: &BuyInStarted) {
        todo!()
    }

    fn apply_buy_in_held(&mut self, _event: &BuyInHeld) {
        todo!()
    }

    fn apply_buy_in_failed(&mut self, _event: &BuyInFailed) {
        todo!()
    }
}
