//! Prints each typed refusal's diagnostic as the generated domain layer
//! has it, one line each, `code arg=value ..`, and registers the generated
//! adapter with angzarr-embedded.

use angzarr_embedded::next::Registry;
use angzarr_embedded::next::Timestamp;
use angzarr_embedded_diagnostic::{Arg, Diagnostic};
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

fn main() {
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
    let mut registry = Registry::new();
    kinds_app::register(&mut registry);
    println!("registered {}", registry.domains().collect::<Vec<_>>().join(" "));
}
