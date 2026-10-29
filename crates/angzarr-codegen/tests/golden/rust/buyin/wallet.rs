//! Wallet: the interface of `buyin.v1.WalletState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. It names only the domain types; the generated adapter
//! (`crate::adapters::Wallet`) carries the markup and calls it.

use crate::domain::*;

pub trait Wallet {
    fn deposit(&self, cmd: &Deposit) -> Result<Deposited, Rejection>;

    fn hold(&self, cmd: &Hold) -> Result<Held, InsufficientFunds>;

    fn apply_deposited(&mut self, event: &Deposited);

    fn apply_held(&mut self, event: &Held);
}
