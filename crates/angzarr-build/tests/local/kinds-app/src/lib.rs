//! The kinds fixture on angzarr-embedded: its wire types, domain layer,
//! adapters and register() generated, its components the scaffold's
//! interfaces and implementations, bodies written, and one projector
//! written by hand.

pub mod wire {
    include!(concat!(env!("OUT_DIR"), "/kinds.v1.rs"));
}
pub mod domain {
    include!(concat!(env!("OUT_DIR"), "/kinds.v1.domain.rs"));
}
pub mod adapters {
    include!(concat!(env!("OUT_DIR"), "/angzarr-adapters.rs"));
}
include!(concat!(env!("OUT_DIR"), "/angzarr-register.rs"));

pub mod audit_state;
pub mod audit_state_impl;
pub mod hand_written;
pub mod ledger_state;
pub mod ledger_state_impl;
pub mod ledger_view;
pub mod ledger_view_impl;

/// The application's registration: the generated components, then the
/// projector written by hand, through angzarr-embedded's public API only.
pub fn register_all(registry: &mut angzarr_embedded::next::Registry) {
    register(registry);
    registry.projector_handler(&["ledger"], hand_written::Tally);
}
