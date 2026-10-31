//! A projector written by hand beside the generated ones: angzarr-router's
//! `ProjectorHandler`, registered with `Registry::projector_handler`. It
//! names nothing of the generator's.

use angzarr_router::error::CodedError;
use angzarr_router::pb;
use angzarr_router::router::ProjectorHandler;

/// Counts the ledger's events, a committed book at a time.
pub struct Tally;

impl ProjectorHandler for Tally {
    fn name(&self) -> &str {
        "projector-tally"
    }

    async fn dispatch(&self, events: &pb::EventBook) -> Result<pb::Projection, CodedError> {
        let domain = events
            .cover
            .as_ref()
            .map_or("", |cover| cover.domain.as_str());
        println!("tally: {} events from {domain}", events.pages.len());
        Ok(pb::Projection::default())
    }
}
