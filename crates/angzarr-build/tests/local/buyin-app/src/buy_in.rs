//! BuyIn: the interface of `buyin.v1.BuyInState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. It names only the domain types; the generated adapter
//! (`crate::adapters::BuyIn`) carries the markup and calls it.

use crate::domain::*;

pub trait BuyIn {
    fn start_buy_in(&self, cmd: &StartBuyIn) -> Result<BuyInStarted, Rejection>;

    fn record_held(&self, cmd: &RecordHeld) -> Result<BuyInHeld, Rejection>;

    fn hold_lost(&self, fact: &HoldLost) -> BuyInFailed;

    fn apply_buy_in_started(&mut self, event: &BuyInStarted);

    fn apply_buy_in_held(&mut self, event: &BuyInHeld);

    fn apply_buy_in_failed(&mut self, event: &BuyInFailed);
}
