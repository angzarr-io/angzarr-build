//! The kinds fixture on angzarr-embedded: its wire types, domain layer,
//! adapters and register() generated, its one component the scaffold's
//! interface and implementation, body written.

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

pub mod ledger_state;
pub mod ledger_state_impl;
