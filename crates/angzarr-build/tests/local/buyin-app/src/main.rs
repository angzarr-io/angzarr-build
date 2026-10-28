//! The buy-in on angzarr-local: one generated `register` call wires the
//! wallet, the buy-in and both policies into angzarr-embedded, which dispatches each
//! through angzarr-router. Prints what happened, one line per fact the test
//! checks.

pub mod wire {
    include!(concat!(env!("OUT_DIR"), "/buyin.v1.rs"));
}
pub mod domain {
    include!(concat!(env!("OUT_DIR"), "/buyin.v1.domain.rs"));
}
include!(concat!(env!("OUT_DIR"), "/angzarr-register.rs"));

pub mod buy_in;
pub mod buy_in_wallet;
pub mod wallet;
pub mod wallet_buy_in;

use angzarr_embedded::next::local::Local;
use angzarr_embedded::next::v1::Uuid;

use crate::domain::{Deposit, StartBuyIn};

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
        .send::<wallet::Wallet>(root(1), Deposit { cents: 100 })
        .unwrap();
    let wallet = local.state::<wallet::Wallet>(root(1));
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());

    // A policy step each way: BuyInStarted -> Hold on the wallet (BuyInWallet),
    // Held -> RecordHeld on the buy-in (WalletBuyIn).
    local
        .send::<buy_in::BuyIn>(
            root(10),
            StartBuyIn {
                wallet: vec![1; 16],
                cents: 30,
            },
        )
        .unwrap();
    let wallet = local.state::<wallet::Wallet>(root(1));
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());
    let buy_in = local.state::<buy_in::BuyIn>(root(10));
    println!("buyin held={} failed={}", buy_in.held(), buy_in.failed());

    // A refusal: the wallet refuses the Hold (InsufficientFunds), records
    // nothing, and BuyInWallet's translate callback hands the buy-in HoldLost.
    local
        .send::<buy_in::BuyIn>(
            root(11),
            StartBuyIn {
                wallet: vec![1; 16],
                cents: 500,
            },
        )
        .unwrap();
    let wallet = local.state::<wallet::Wallet>(root(1));
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());
    let buy_in = local.state::<buy_in::BuyIn>(root(11));
    println!("buyin held={} failed={}", buy_in.held(), buy_in.failed());
}
