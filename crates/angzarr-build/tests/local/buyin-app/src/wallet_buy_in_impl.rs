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

    /// Never sent: the wallet starts no buy-in.
    fn start_buy_in_refused(
        &self,
        _command: &StartBuyIn,
        _refusal: Option<BuyInRefused>,
        _ctx: &CommandContext,
    ) -> Result<WalletFact, Bug> {
        Err(Bug::new(angzarr_embedded::angzarr_diagnostic::code!("start-never-sent")))
    }

    fn record_held_refused(
        &self,
        _command: &RecordHeld,
        refusal: Option<BuyInRefused>,
        _ctx: &CommandContext,
    ) -> Result<WalletFact, Bug> {
        Ok(RecordBuyInRefused {
            reason: refusal.map(|r| r.reason).unwrap_or_default(),
        }
        .into())
    }
}
