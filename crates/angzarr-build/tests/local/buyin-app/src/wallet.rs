//! The wallet: the scaffold of `buyin.v1.WalletState`, bodies written.

use std::convert::Infallible;

use annals::next::markup::{Context, Refuses};
use annals_macros::{aggregate, applies, handles_cmd};

use crate::domain::*;

#[derive(Clone, Debug, Default)]
pub struct Wallet {
    balance: i64,
    held: i64,
}

impl Wallet {
    pub fn balance(&self) -> i64 {
        self.balance
    }

    pub fn held(&self) -> i64 {
        self.held
    }
}

#[aggregate(domain = "wallet")]
impl Wallet {
    #[handles_cmd(Deposit)]
    fn deposit(&self, cmd: Deposit, ctx: &Context) -> Result<Deposited, Infallible> {
        let _ = ctx;
        Ok(Deposited { cents: cmd.cents })
    }

    #[handles_cmd(Hold, refuses(InsufficientFunds))]
    fn hold(&self, cmd: Hold, ctx: &Context) -> Result<Held, Refuses<Self, Hold>> {
        let _ = ctx;
        let available = self.balance - self.held;
        if cmd.cents > available {
            return Err(InsufficientFunds {
                short: cmd.cents - available,
            }
            .into());
        }
        Ok(Held { cents: cmd.cents })
    }

    #[applies(Deposited)]
    fn deposited(&mut self, event: Deposited) {
        self.balance += event.cents;
    }

    #[applies(Held)]
    fn held(&mut self, event: Held) {
        self.held += event.cents;
    }
}
