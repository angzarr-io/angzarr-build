//! AuditState: the interface of `kinds.v1.AuditState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. It names only `crate::domain`'s types, its handlers'
//! `Context` among them; the generated adapter (`crate::adapters::AuditState`)
//! carries the markup and calls it.

use crate::domain::*;

pub trait AuditState {
    fn audit(&self, cmd: &Audit, ctx: &Context) -> Result<Audited, Failed<AuditRefuses>>;

    fn apply_audited(&mut self, event: &Audited);
}
