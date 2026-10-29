//! Wallet: the interface of `buyin.v1.WalletState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. It names only `crate::domain`'s types, its handlers'
//! `Context` among them; the generated adapter (`crate::adapters::Wallet`)
//! carries the markup and calls it.

use crate::domain::*;

pub trait Wallet {
    fn deposit(&self, cmd: &Deposit, ctx: &Context) -> Result<Deposited, Rejection>;

    fn hold(&self, cmd: &Hold, ctx: &Context) -> Result<Held, InsufficientFunds>;

    fn apply_deposited(&mut self, event: &Deposited);

    fn apply_held(&mut self, event: &Held);
}
