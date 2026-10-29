//! The buy-in on angzarr-embedded: one generated `register` call wires the
//! wallet, the buy-in and both policies into angzarr-embedded, which dispatches each
//! through angzarr-router. Prints what happened, one line per fact the test
//! checks.

use buyin_app::{adapters, register};
use angzarr_embedded::next::local::Local;
use angzarr_embedded::next::v1::Uuid;

use buyin_app::domain::{Deposit, StartBuyIn};

fn root(n: u8) -> Uuid {
    Uuid {
        value: vec![n; 16].into(),
    }
}

// The one file to adjust when angzarr-embedded's local entry point settles: the
// in-memory runtime, a command to a stream, and a read of a stream's state.
fn main() {
    let mut registry = angzarr_embedded::next::Registry::new();
    register(&mut registry);
    let local = Local::in_memory(registry);

    // One command.
    local
        .send::<adapters::Wallet>(root(1), Deposit { cents: 100 })
        .unwrap();
    let wallet = local.state::<adapters::Wallet>(root(1));
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());

    // A policy step each way: BuyInStarted -> Hold on the wallet (BuyInWallet),
    // Held -> RecordHeld on the buy-in (WalletBuyIn).
    local
        .send::<adapters::BuyIn>(
            root(10),
            StartBuyIn {
                wallet: vec![1; 16],
                cents: 30,
            },
        )
        .unwrap();
    let wallet = local.state::<adapters::Wallet>(root(1));
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());
    let buy_in = local.state::<adapters::BuyIn>(root(10));
    println!("buyin held={} failed={}", buy_in.held(), buy_in.failed());

    // A refusal: the wallet refuses the Hold (InsufficientFunds), records
    // nothing, and BuyInWallet's translate callback hands the buy-in HoldLost.
    local
        .send::<adapters::BuyIn>(
            root(11),
            StartBuyIn {
                wallet: vec![1; 16],
                cents: 500,
            },
        )
        .unwrap();
    let wallet = local.state::<adapters::Wallet>(root(1));
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());
    let buy_in = local.state::<adapters::BuyIn>(root(11));
    println!("buyin held={} failed={}", buy_in.held(), buy_in.failed());
}
