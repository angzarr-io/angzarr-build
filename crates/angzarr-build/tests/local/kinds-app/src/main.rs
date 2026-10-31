//! Prints each typed refusal's diagnostic as the generated domain layer
//! has it, one line each, `code arg=value ..`; then runs the app from its
//! angzarr.toml (the generated register() and a projector written by hand)
//! and sends a command to each domain: the generated projector projects
//! both, the hand-written one the ledger's.

use angzarr_embedded::next::markup::DomainMessage;
use angzarr_embedded::next::v1::{Command, CommandBook, CommandPage, Cover, Uuid, command_page};
use angzarr_embedded::next::{Sampled, Timestamp, pack};
use angzarr_embedded_diagnostic::{Arg, Diagnostic};
use angzarr_embedded_host::App;
use kinds_app::domain::*;
use kinds_app::ledger_state::LedgerState;
use kinds_app::ledger_state_impl::LedgerStateImpl;

fn line(diagnostic: &dyn Diagnostic) -> String {
    let args: Vec<String> = diagnostic
        .args()
        .into_iter()
        .map(|(name, value)| match value {
            Arg::Int(n) => format!("{name}={n}"),
            Arg::Uint(n) => format!("{name}={n}"),
            Arg::Str(s) => format!("{name}={s}"),
            Arg::Nested(_) => format!("{name}=.."),
        })
        .collect();
    format!("{} {}", diagnostic.code(), args.join(" "))
}

/// `command` to `domain`'s stream at root `n`, the `n`th command.
async fn send<C: DomainMessage>(app: &App, n: u8, domain: &str, command: C) {
    let book = CommandBook {
        cover: Some(Cover {
            domain: domain.to_string(),
            root: Some(Uuid::from_bytes([n; 16])),
            edition: None,
        }),
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

#[tokio::main]
async fn main() {
    let ctx = Context::new(Timestamp::default());
    let Err(Failed::Rejected { rejection, .. }) = LedgerStateImpl::default().post(&Post {}, &ctx)
    else {
        panic!("post refuses");
    };
    println!("{}", line(&rejection));
    println!("{rejection}");
    println!("{}", line(&Unreasoned { reason: Tone::Loud }));
    println!(
        "{}",
        line(&Listed {
            reason: vec![ListedReason::Full]
        })
    );
    println!(
        "{}",
        line(&Elsewhere {
            why: ElsewhereReason::Away
        })
    );
    println!(
        "{}",
        line(&Mislabeled {
            reason: MislabeledCause::Late
        })
    );

    let config = concat!(env!("CARGO_MANIFEST_DIR"), "/angzarr.toml");
    let app = angzarr_embedded_host::run(config, kinds_app::register_all)
        .await
        .expect("the app runs");
    // ONE of two: both recorded, the projectors run once for the book;
    // Noted sets the policy off, auditing the same root.
    send(&app, 1, "ledger", Open {}).await;
    println!("generated policy: {}", audit_command(&app, 1).await);
    // MANY: Noted, then Posted, as the handler ordered them.
    send(&app, 2, "ledger", Batch {}).await;
    send(&app, 3, "audit", Audit {}).await;
    drop(app);

    // The policy swapped for the hand-written one of the same name: the
    // same trigger derives the same command id.
    println!("swapped");
    let app = angzarr_embedded_host::run(config, kinds_app::register_swapped)
        .await
        .expect("the swapped app runs");
    send(&app, 1, "ledger", Open {}).await;
    println!("hand-written policy: {}", audit_command(&app, 1).await);
}

/// The id of the command recorded first on the audit stream at root `n`,
/// in hex.
async fn audit_command(app: &App, n: u8) -> String {
    let cover = Cover {
        domain: "audit".to_string(),
        root: Some(Uuid::from_bytes([n; 16])),
        edition: None,
    };
    let pages = app.events(&cover).await.expect("the stream reads");
    let meta = pages[0].meta.as_ref().expect("an event records its meta");
    let Some(angzarr_embedded::next::v1::event_meta::Causation::CommandId(id)) = &meta.causation
    else {
        panic!("the event records its command id");
    };
    id.value.iter().map(|b| format!("{b:02x}")).collect()
}
