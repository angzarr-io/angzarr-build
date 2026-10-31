//! AuditStateImpl: the implementation of `AuditState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::audit_state::AuditState;
use crate::domain::*;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct AuditStateImpl {
    pub state: crate::wire::AuditState,
}

impl AuditState for AuditStateImpl {
    /// Audits every root but 2, which it refuses with one of its two
    /// refusal types: the policy that sent it translates the refusal.
    fn audit(&self, _cmd: &Audit, ctx: &Context) -> Result<Audited, Failed<AuditRefuses>> {
        let root = ctx
            .command
            .cover
            .as_ref()
            .and_then(|cover| cover.root.as_ref())
            .map(|root| root.value.to_vec());
        if root == Some(vec![2; 16]) {
            return Err(Failed::rejected(AuditRefuses::from(Unreasoned {
                reason: Tone::Loud,
            })));
        }
        Ok(Audited {})
    }

    fn apply_audited(&mut self, _event: &Audited) {}
}
