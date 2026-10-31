//! LedgerView: the interface of `kinds.v1.LedgerView`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. One async method per projected event, of any of its
//! source domains (`ctx.stream()` says which), returning its `Bug` when
//! the projection fails. The generated adapter
//! (`crate::adapters::LedgerView`) carries the markup.

use crate::domain::*;

pub trait LedgerView {
    fn posted(
        &mut self,
        event: &Posted,
        ctx: &ProjectionContext,
    ) -> impl std::future::Future<Output = Result<(), Bug>> + Send;

    fn audited(
        &mut self,
        event: &Audited,
        ctx: &ProjectionContext,
    ) -> impl std::future::Future<Output = Result<(), Bug>> + Send;
}
