//! WalletImpl: the implementation of `Wallet`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::domain::*;
use crate::wallet::Wallet;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct WalletImpl {
    pub state: crate::wire::WalletState,
}

impl Wallet for WalletImpl {
    fn deposit(&self, _cmd: &Deposit) -> Result<Deposited, Rejection> {
        todo!()
    }

    fn hold(&self, _cmd: &Hold) -> Result<Held, InsufficientFunds> {
        todo!()
    }

    fn apply_deposited(&mut self, _event: &Deposited) {
        todo!()
    }

    fn apply_held(&mut self, _event: &Held) {
        todo!()
    }
}
