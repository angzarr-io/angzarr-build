//! BuyInWalletImpl: the scaffold of `BuyInWallet`'s implementation, every
//! refusal translated (decision 14.76), Hold's into HoldLost (the refusal
//! switch names it), bodies written.

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
    ) -> Result<BuyInFact, Bug> {
        Ok(HoldLost {
            short: refusal.map(|r| r.short).unwrap_or(command.cents),
        }
        .into())
    }

    /// Never sent: the buy-in deposits nothing.
    fn deposit_refused(
        &self,
        _command: &Deposit,
        _refusal: Option<DepositRefused>,
        _ctx: &CommandContext,
    ) -> Result<BuyInFact, Bug> {
        Err(Bug::new(angzarr_embedded::angzarr_diagnostic::code!("deposit-never-sent")))
    }
}
