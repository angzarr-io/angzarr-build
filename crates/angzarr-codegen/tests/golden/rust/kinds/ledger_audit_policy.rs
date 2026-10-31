//! LedgerAuditPolicy: the interface of `kinds.v1.LedgerAuditPolicy`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. Each reaction takes the source stream's root, the
//! event and its context, and returns the commands to send, each with its
//! target root, or its `Bug`: a defect, since a policy reacts to events the
//! application's own domains recorded. The generated adapter
//! (`crate::adapters::LedgerAuditPolicy`) carries the markup.

use crate::domain::*;

pub trait LedgerAuditPolicy {
    fn noted(
        &self,
        source: &[u8],
        event: &Noted,
        ctx: &PolicyContext,
    ) -> Result<Vec<(Vec<u8>, AuditStateCommand)>, Bug>;
}
