//! WalletBuyIn: the scaffold of `buyin.v1.WalletBuyInPolicy`, bodies
//! written. The buy-in's root is the correlation.

use annals::next::markup::{Commands, PolicyContext};
use annals_macros::{on_event, policy};

use crate::buy_in::BuyIn;
use crate::domain::*;
use crate::wallet::Wallet;

pub struct WalletBuyIn;

#[policy(name = "wallet-buy-in", source = Wallet, target = BuyIn)]
impl WalletBuyIn {
    #[on_event(Held)]
    fn held(&self, event: &Held, ctx: &PolicyContext) -> Commands<BuyIn> {
        Commands::none().to(
            ctx.correlation_id().clone(),
            RecordHeld { cents: event.cents },
        )
    }
}
