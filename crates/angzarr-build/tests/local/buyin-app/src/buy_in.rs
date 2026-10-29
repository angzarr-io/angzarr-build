//! BuyIn: the interface of `buyin.v1.BuyInState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. It names only `crate::domain`'s types, its handlers'
//! `Context` among them; the generated adapter (`crate::adapters::BuyIn`)
//! carries the markup and calls it.

use crate::domain::*;

pub trait BuyIn {
    fn start_buy_in(&self, cmd: &StartBuyIn, ctx: &Context) -> Result<BuyInStarted, Rejection>;

    fn record_held(&self, cmd: &RecordHeld, ctx: &Context) -> Result<BuyInHeld, Rejection>;

    fn hold_lost(&self, fact: &HoldLost, ctx: &Context) -> BuyInFailed;

    fn apply_buy_in_started(&mut self, event: &BuyInStarted);

    fn apply_buy_in_held(&mut self, event: &BuyInHeld);

    fn apply_buy_in_failed(&mut self, event: &BuyInFailed);
}
