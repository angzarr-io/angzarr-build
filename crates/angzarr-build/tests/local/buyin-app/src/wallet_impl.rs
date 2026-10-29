//! WalletImpl: the scaffold of `Wallet`'s implementation, bodies written.

use crate::domain::*;
use crate::wallet::Wallet;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct WalletImpl {
    pub state: crate::wire::WalletState,
}

impl Wallet for WalletImpl {
    fn deposit(&self, cmd: &Deposit, _ctx: &Context) -> Result<Deposited, Failed<Rejection>> {
        Ok(Deposited { cents: cmd.cents })
    }

    fn hold(&self, cmd: &Hold, _ctx: &Context) -> Result<Held, Failed<InsufficientFunds>> {
        let available = self.state.balance - self.state.held;
        if cmd.cents > available {
            return Err(InsufficientFunds {
                short: cmd.cents - available,
            }
            .into());
        }
        Ok(Held {
            cents: cmd.cents,
            buy_in: cmd.buy_in.clone(),
        })
    }

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
