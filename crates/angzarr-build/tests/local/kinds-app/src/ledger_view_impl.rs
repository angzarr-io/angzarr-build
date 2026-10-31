//! LedgerViewImpl: the implementation of `LedgerView`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::domain::*;
use crate::ledger_view::LedgerView;

/// The projection the methods update.
#[derive(Clone, Debug, Default)]
pub struct LedgerViewImpl {
    pub state: crate::wire::LedgerView,
}

impl LedgerView for LedgerViewImpl {
    async fn posted(&mut self, _event: &Posted, ctx: &ProjectionContext) -> Result<(), Bug> {
        self.state.seen += 1;
        println!("view: Posted from {} ({})", ctx.stream().domain, self.state.seen);
        Ok(())
    }

    async fn audited(&mut self, _event: &Audited, ctx: &ProjectionContext) -> Result<(), Bug> {
        self.state.seen += 1;
        println!("view: Audited from {} ({})", ctx.stream().domain, self.state.seen);
        Ok(())
    }
}
