//! WalletBuyInImpl: the scaffold of `WalletBuyIn`'s implementation, bodies
//! written. The buy-in's root is the one the Hold carried.

use crate::domain::*;

#[derive(Clone, Debug, Default)]
pub struct WalletBuyInImpl;

impl WalletBuyIn for WalletBuyInImpl {
    fn held(
        &self,
        _source: &[u8],
        event: &Held,
        _ctx: &PolicyContext,
    ) -> Result<Vec<(Vec<u8>, BuyInOutput)>, Bug> {
        Ok(vec![(
            event.buy_in.clone(),
            BuyInOutput::RecordHeld(RecordHeld { cents: event.cents }),
        )])
    }
}
