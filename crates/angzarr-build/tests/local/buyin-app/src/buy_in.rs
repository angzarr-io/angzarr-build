//! The buy-in: the scaffold of `buyin.v1.BuyInState`, bodies written.

use std::convert::Infallible;

use annals::next::markup::Context;
use annals_macros::{applies, handles_cmd, process_manager};

use crate::domain::*;

#[derive(Clone, Debug, Default)]
pub struct BuyIn {
    started: bool,
    held: i64,
    failed: bool,
}

impl BuyIn {
    pub fn held(&self) -> i64 {
        self.held
    }

    pub fn failed(&self) -> bool {
        self.failed
    }
}

#[process_manager(domain = "buyin")]
impl BuyIn {
    #[handles_cmd(StartBuyIn, starts)]
    fn start_buy_in(&self, cmd: StartBuyIn, ctx: &Context) -> Result<BuyInStarted, Infallible> {
        let _ = ctx;
        Ok(BuyInStarted {
            wallet: cmd.wallet,
            cents: cmd.cents,
        })
    }

    #[handles_cmd(RecordHeld)]
    fn record_held(&self, cmd: RecordHeld, ctx: &Context) -> Result<BuyInHeld, Infallible> {
        let _ = ctx;
        Ok(BuyInHeld { cents: cmd.cents })
    }

    #[handles_cmd(HoldLost)]
    fn hold_lost(&self, cmd: HoldLost, ctx: &Context) -> Result<BuyInFailed, Infallible> {
        let _ = ctx;
        Ok(BuyInFailed { short: cmd.short })
    }

    #[applies(BuyInStarted)]
    fn buy_in_started(&mut self, event: BuyInStarted) {
        let _ = event;
        self.started = true;
    }

    #[applies(BuyInHeld)]
    fn buy_in_held(&mut self, event: BuyInHeld) {
        self.held += event.cents;
    }

    #[applies(BuyInFailed)]
    fn buy_in_failed(&mut self, event: BuyInFailed) {
        let _ = event;
        self.failed = true;
    }
}
