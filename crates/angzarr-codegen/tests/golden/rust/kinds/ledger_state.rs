//! LedgerState: the interface of `kinds.v1.LedgerState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. It names only `crate::domain`'s types, its handlers'
//! `Context` among them; the generated adapter (`crate::adapters::LedgerState`)
//! carries the markup and calls it.

use crate::domain::*;

pub trait LedgerState {
    fn post(&self, cmd: &Post, ctx: &Context) -> Result<Posted, Failed<PostRefuses>>;

    fn apply_posted(&mut self, event: &Posted);
}
