//! BuyInWalletImpl: the scaffold of `BuyInWallet`'s implementation, with Hold
//! translated (the refusal switch), bodies written.

use crate::domain::*;

#[derive(Clone, Debug, Default)]
pub struct BuyInWalletImpl;

impl BuyInWallet for BuyInWalletImpl {
    fn buy_in_started(
        &self,
        source: &[u8],
        event: &BuyInStarted,
        _ctx: &PolicyContext,
    ) -> Result<Vec<(Vec<u8>, WalletOutput)>, Bug> {
        Ok(vec![(
            event.wallet.clone(),
            WalletOutput::Hold(Hold {
                cents: event.cents,
                buy_in: source.to_vec(),
            }),
        )])
    }

    fn hold_refused(
        &self,
        command: &Hold,
        refusal: Option<InsufficientFunds>,
        _ctx: &CommandContext,
    ) -> Result<Option<BuyInFact>, Bug> {
        Ok(Some(
            HoldLost {
                short: refusal.map(|r| r.short).unwrap_or(command.cents),
            }
            .into(),
        ))
    }
}
