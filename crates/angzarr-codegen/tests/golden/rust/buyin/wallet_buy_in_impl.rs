//! WalletBuyInImpl: the implementation of `WalletBuyIn`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::domain::*;
use crate::wallet_buy_in::WalletBuyIn;

#[derive(Clone, Debug, Default)]
pub struct WalletBuyInImpl;

impl WalletBuyIn for WalletBuyInImpl {
    fn held(
        &self,
        _source: &[u8],
        _event: &Held,
        _ctx: &PolicyContext,
    ) -> Result<Vec<(Vec<u8>, BuyInCommand)>, Error> {
        todo!()
    }
}
