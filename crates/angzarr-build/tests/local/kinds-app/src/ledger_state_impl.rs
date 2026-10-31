//! LedgerStateImpl: the implementation of `LedgerState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks, each `todo!()`.

use crate::domain::*;
use crate::ledger_state::LedgerState;

/// The state the `apply_` methods fold.
#[derive(Clone, Debug, Default)]
pub struct LedgerStateImpl {
    pub state: crate::wire::LedgerState,
}

impl LedgerState for LedgerStateImpl {
    /// Always refused: over the limit, with every kind of arg set.
    fn post(&self, _cmd: &Post, _ctx: &Context) -> Result<Posted, Failed<PostRefuses>> {
        Err(Failed::rejected(PostRefused {
            reason: PostRefusedReason::OverLimit,
            small: -1,
            big: 2,
            count: 3,
            total: 4,
            text: "t".to_string(),
            data: vec![0xab, 0x01],
            flag: true,
            ratio: 0.5,
            share: 1.5,
            tone: Tone::Loud,
            limit: Some(9),
            maybe: None,
            note: Note {
                text: "n".to_string(),
            },
            list: vec![1],
            table: Default::default(),
        }))
    }

    fn apply_posted(&mut self, _event: &Posted) {}
}
