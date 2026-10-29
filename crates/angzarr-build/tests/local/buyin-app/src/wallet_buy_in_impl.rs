//! WalletBuyInImpl: the scaffold of `WalletBuyIn`'s implementation, bodies
//! written. The buy-in's root is the one the Hold carried.

use crate::domain::*;
use crate::wallet_buy_in::WalletBuyIn;

#[derive(Clone, Debug, Default)]
pub struct WalletBuyInImpl;

impl WalletBuyIn for WalletBuyInImpl {
    fn held(&self, _source: &[u8], event: &Held) -> Vec<(Vec<u8>, BuyInCommand)> {
        vec![(
            event.buy_in.clone(),
            BuyInCommand::RecordHeld(RecordHeld { cents: event.cents }),
        )]
    }
}
