//! BuyInWalletImpl: the implementation of `BuyInWallet`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::buy_in_wallet::BuyInWallet;
use crate::domain::*;

#[derive(Clone, Debug, Default)]
pub struct BuyInWalletImpl;

impl BuyInWallet for BuyInWalletImpl {
    fn buy_in_started(
        &self,
        _source: &[u8],
        _event: &BuyInStarted,
        _ctx: &PolicyContext,
    ) -> Result<Vec<(Vec<u8>, WalletCommand)>, Bug> {
        todo!()
    }
}
