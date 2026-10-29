//! BuyInWallet: the interface of `buyin.v1.BuyInWalletPolicy`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. Each reaction takes the source stream's root and the
//! event, and returns the commands to send, each with its target root; the
//! generated adapter (`crate::adapters::BuyInWallet`) carries the markup.

use crate::domain::*;

pub trait BuyInWallet {
    fn buy_in_started(&self, source: &[u8], event: &BuyInStarted) -> Vec<(Vec<u8>, WalletCommand)>;
}
