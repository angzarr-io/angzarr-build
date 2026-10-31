//! The buy-in on angzarr-embedded: `angzarr_embedded_host::run` reads the
//! app's angzarr.toml and calls the generated `register`, which wires the
//! wallet, the buy-in and both policies into angzarr-embedded, which
//! dispatches each through angzarr-router. Prints what happened, one line
//! per fact the test checks. The App is async, on the caller's tokio runtime.

use angzarr_embedded::next::markup::DomainMessage;
use angzarr_embedded::next::v1::{Command, CommandBook, CommandPage, Cover, Uuid, command_page};
use angzarr_embedded::next::{Domain, Payload, Sampled, Timestamp, pack};
use angzarr_embedded_host::App;
use buyin_app::domain::{Deposit, StartBuyIn};
use buyin_app::{adapters, register};

fn root(n: u8) -> Uuid {
    Uuid {
        value: vec![n; 16].into(),
    }
}

fn cover<D: Domain>(root: Uuid) -> Cover {
    Cover {
        domain: D::NAME.to_string(),
        root: Some(root),
        edition: None,
    }
}

/// `command` to the `D` stream at `root`, the `n`th command of the run;
/// panics if the command, or a policy step it sets off, fails.
async fn send<D: Domain, C: DomainMessage>(app: &App, n: u8, root: Uuid, command: C) {
    let book = CommandBook {
        cover: Some(cover::<D>(root)),
        page: Some(CommandPage {
            body: Some(command_page::Body::Command(Command {
                payload: Some(pack(&command.to_wire())),
            })),
            command_id: Some(Uuid::from_bytes([0xc0 | n; 16])),
            ..CommandPage::default()
        }),
        handling: None,
    };
    let sampled = Sampled {
        now: Timestamp {
            seconds: i64::from(n),
            nanos: 0,
        },
        command_id: Uuid::from_bytes([0x70 | n; 16]),
    };
    app.handle(book, &sampled)
        .await
        .expect("the command is handled");
}

/// The state of the `D` stream at `root`: its events, folded.
async fn state<D: Domain<State = D, Rules = ()> + Default>(app: &App, root: Uuid) -> D {
    let mut state = D::default();
    for page in app
        .events(&cover::<D>(root))
        .await
        .expect("the stream reads")
    {
        let any = page.payload.expect("an event page carries its event");
        let event = D::Event::from_any(&any).expect("the event decodes");
        D::apply(&mut state, &event, &());
    }
    state
}

#[tokio::main]
async fn main() {
    let config = concat!(env!("CARGO_MANIFEST_DIR"), "/angzarr.toml");
    let app = angzarr_embedded_host::run(config, register)
        .await
        .expect("the app runs");

    // One command.
    send::<adapters::Wallet, _>(&app, 1, root(1), Deposit { cents: 100 }).await;
    let wallet = state::<adapters::Wallet>(&app, root(1)).await;
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());

    // A policy step each way: BuyInStarted -> Hold on the wallet (BuyInWallet),
    // Held -> RecordHeld on the buy-in (WalletBuyIn).
    send::<adapters::BuyIn, _>(
        &app,
        2,
        root(10),
        StartBuyIn {
            wallet: vec![1; 16],
            cents: 30,
        },
    )
    .await;
    let wallet = state::<adapters::Wallet>(&app, root(1)).await;
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());
    let buy_in = state::<adapters::BuyIn>(&app, root(10)).await;
    println!("buyin held={} failed={}", buy_in.held(), buy_in.failed());

    // A refusal: the wallet refuses the Hold (InsufficientFunds), records
    // nothing, and BuyInWallet's translate callback hands the buy-in HoldLost.
    send::<adapters::BuyIn, _>(
        &app,
        3,
        root(11),
        StartBuyIn {
            wallet: vec![1; 16],
            cents: 500,
        },
    )
    .await;
    let wallet = state::<adapters::Wallet>(&app, root(1)).await;
    println!("wallet balance={} held={}", wallet.balance(), wallet.held());
    let buy_in = state::<adapters::BuyIn>(&app, root(11)).await;
    println!("buyin held={} failed={}", buy_in.held(), buy_in.failed());
}
