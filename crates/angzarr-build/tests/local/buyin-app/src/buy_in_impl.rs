//! BuyInImpl: the scaffold of `BuyIn`'s implementation, bodies written.

use crate::buy_in::BuyIn;
use crate::domain::*;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct BuyInImpl {
    pub state: crate::wire::BuyInState,
}

impl BuyIn for BuyInImpl {
    fn start_buy_in(&self, cmd: &StartBuyIn) -> Result<BuyInStarted, Rejection> {
        Ok(BuyInStarted {
            wallet: cmd.wallet.clone(),
            cents: cmd.cents,
        })
    }

    fn record_held(&self, cmd: &RecordHeld) -> Result<BuyInHeld, Rejection> {
        Ok(BuyInHeld { cents: cmd.cents })
    }

    fn hold_lost(&self, fact: &HoldLost) -> BuyInFailed {
        BuyInFailed { short: fact.short }
    }

    fn apply_buy_in_started(&mut self, _event: &BuyInStarted) {
        self.state.started = true;
    }

    fn apply_buy_in_held(&mut self, event: &BuyInHeld) {
        self.state.held += event.cents;
    }

    fn apply_buy_in_failed(&mut self, _event: &BuyInFailed) {
        self.state.failed = true;
    }
}

impl BuyInImpl {
    pub fn held(&self) -> i64 {
        self.state.held
    }

    pub fn failed(&self) -> bool {
        self.state.failed
    }
}
