//! WalletBuyIn: the interface of `buyin.v1.WalletBuyInPolicy`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. Each reaction takes the source stream's root and the
//! event, and returns the commands to send, each with its target root; the
//! generated adapter (`crate::adapters::WalletBuyIn`) carries the markup.

use crate::domain::*;

pub trait WalletBuyIn {
    fn held(&self, source: &[u8], event: &Held) -> Vec<(Vec<u8>, BuyInCommand)>;
}
