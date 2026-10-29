//! blackjack on angzarr-embedded: the Rust domains in process, and each
//! domain angzarr.toml lists in another language launched as a child
//! process. Written once by `angzarr scaffold startup`; `angzarr scaffold
//! rust` writes the modules.

pub mod wire {
    include!(concat!(env!("OUT_DIR"), "/io.angzarr.examples.v1.rs"));
}
pub mod domain {
    include!(concat!(env!("OUT_DIR"), "/io.angzarr.examples.v1.domain.rs"));
}
pub mod adapters {
    include!(concat!(env!("OUT_DIR"), "/angzarr-adapters.rs"));
}
include!(concat!(env!("OUT_DIR"), "/angzarr-register.rs"));

pub mod player_table_policy;
pub mod player_table_policy_impl;
pub mod table_aggregate;
pub mod table_aggregate_impl;
pub mod table_player_policy;
pub mod table_player_policy_impl;

fn main() -> std::process::ExitCode {
    angzarr_embedded_host::run("angzarr.toml", register)
}
