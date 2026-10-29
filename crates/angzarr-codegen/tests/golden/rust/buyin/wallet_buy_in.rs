//! WalletBuyIn: the interface of `buyin.v1.WalletBuyInPolicy`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. Each reaction takes the source stream's root, the
//! event and its context, and returns the commands to send, each with its
//! target root, or its `Error`: a bug, since a policy reacts to events the
//! application's own domains recorded. The generated adapter
//! (`crate::adapters::WalletBuyIn`) carries the markup.

use crate::domain::*;

pub trait WalletBuyIn {
    fn held(
        &self,
        source: &[u8],
        event: &Held,
        ctx: &PolicyContext,
    ) -> Result<Vec<(Vec<u8>, BuyInCommand)>, Error>;
}
