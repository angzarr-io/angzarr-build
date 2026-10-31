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

    /// A refused Audit is the ledger's Arrived; the refusal, the choice of
    /// Audit's two types, is printed as it is handed over.
    fn audit_refused(
        &self,
        _command: &Audit,
        refusal: Option<AuditRefuses>,
        _ctx: &CommandContext,
    ) -> Result<Arrived, Bug> {
        println!("audit refused: {refusal:?}");
        Ok(Arrived {})
    }
}
