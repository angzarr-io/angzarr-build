//! LedgerState: the interface of `kinds.v1.LedgerState`, written once by
//! `angzarr scaffold rust` and yours from then on: `--missing` adds the
//! methods it lacks. It names only `crate::domain`'s types, its handlers'
//! `Context` among them; the generated adapter (`crate::adapters::LedgerState`)
//! carries the markup and calls it.

use crate::domain::*;

pub trait LedgerState {
    fn post(&self, cmd: &Post, ctx: &Context) -> Result<Posted, Failed<PostRefuses>>;

    fn open(&self, cmd: &Open, ctx: &Context) -> Result<(Posted, Noted), Failed<Rejection>>;

    fn four(
        &self,
        cmd: &Four,
        ctx: &Context,
    ) -> Result<(Posted, Noted, Cleared, Closed), Failed<Rejection>>;

    fn maybe(&self, cmd: &Maybe, ctx: &Context) -> Result<Option<Posted>, Failed<Rejection>>;

    fn maybe_both(
        &self,
        cmd: &MaybeBoth,
        ctx: &Context,
    ) -> Result<Option<(Posted, Noted)>, Failed<Rejection>>;

    fn batch(
        &self,
        cmd: &Batch,
        ctx: &Context,
    ) -> Result<Vec<LedgerStateEvent>, Failed<PostRefused>>;

    fn raw(&self, cmd: &Raw, ctx: &Context) -> Result<Vec<LedgerStateEvent>, Failed<Rejection>>;

    fn arrived(&self, fact: &Arrived, ctx: &Context) -> Option<(Posted, Noted)>;

    fn swept(&self, fact: &Swept, ctx: &Context) -> Vec<LedgerStateEvent>;

    fn apply_posted(&mut self, event: &Posted);

    fn apply_noted(&mut self, event: &Noted);

    fn apply_cleared(&mut self, event: &Cleared);

    fn apply_closed(&mut self, event: &Closed);
}
