//! BuyInWallet: the scaffold of `buyin.v1.BuyInWalletPolicy`, with Hold
//! translated (the refusal switch), bodies written.

use annals::next::markup::{Commands, PolicyContext, Refusal};
use annals::next::v1::Uuid;
use annals_macros::{on_event, on_refused, policy};

use crate::buy_in::BuyIn;
use crate::domain::*;
use crate::wallet::Wallet;

pub struct BuyInWallet;

#[policy(name = "buy-in-wallet", source = BuyIn, target = Wallet)]
impl BuyInWallet {
    #[on_event(BuyInStarted)]
    fn buy_in_started(&self, event: &BuyInStarted, ctx: &PolicyContext) -> Commands<Wallet> {
        let _ = ctx;
        let wallet = Uuid {
            value: event.wallet.clone().into(),
        };
        Commands::none().to(wallet, Hold { cents: event.cents })
    }

    #[on_refused(Hold)]
    fn hold_refused(&self, refusal: &Refusal<Hold>) -> HoldLost {
        HoldLost {
            short: refusal
                .refusal::<InsufficientFunds>()
                .map(|object| object.short)
                .unwrap_or(refusal.command().cents),
        }
    }
}
