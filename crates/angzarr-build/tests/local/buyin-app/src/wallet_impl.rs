//! WalletImpl: the scaffold of `Wallet`'s implementation, bodies written.

use crate::domain::*;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct WalletImpl {
    pub state: crate::wire::WalletState,
}

impl Wallet for WalletImpl {
    fn deposit(
        &self,
        cmd: &Deposit,
        _ctx: &Context,
    ) -> Result<Vec<WalletEvent>, Failed<DepositRefused>> {
        Ok(vec![Deposited { cents: cmd.cents }.into()])
    }

    fn hold(
        &self,
        cmd: &Hold,
        _ctx: &Context,
    ) -> Result<Vec<WalletEvent>, Failed<InsufficientFunds>> {
        let available = self.state.balance - self.state.held;
        if cmd.cents > available {
            return Err(InsufficientFunds {
                short: cmd.cents - available,
            }
            .into());
        }
        Ok(vec![Held {
            cents: cmd.cents,
            buy_in: cmd.buy_in.clone(),
        }
        .into()])
    }

    fn record_buy_in_refused(
        &self,
        fact: &RecordBuyInRefused,
        _ctx: &Context,
    ) -> Vec<WalletEvent> {
        vec![BuyInRefusalRecorded {
            reason: fact.reason.clone(),
        }
        .into()]
    }

    fn apply_buy_in_refusal_recorded(&mut self, _event: &BuyInRefusalRecorded) {}

    fn apply_deposited(&mut self, event: &Deposited) {
        self.state.balance += event.cents;
    }

    fn apply_held(&mut self, event: &Held) {
        self.state.held += event.cents;
    }
}

impl WalletImpl {
    pub fn balance(&self) -> i64 {
        self.state.balance
    }

    pub fn held(&self) -> i64 {
        self.state.held
    }
}
