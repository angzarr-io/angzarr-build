//! LedgerAuditPolicyImpl: the implementation of `LedgerAuditPolicy`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::domain::*;
use crate::ledger_audit_policy::LedgerAuditPolicy;

#[derive(Clone, Debug, Default)]
pub struct LedgerAuditPolicyImpl;

impl LedgerAuditPolicy for LedgerAuditPolicyImpl {
    /// Audits the ledger's root.
    fn noted(
        &self,
        source: &[u8],
        _event: &Noted,
        _ctx: &PolicyContext,
    ) -> Result<Vec<(Vec<u8>, AuditStateCommand)>, Bug> {
        Ok(vec![(source.to_vec(), Audit {}.into())])
    }
}
