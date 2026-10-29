//! Generates the wire types, the domain layer, the adapters and
//! `register()` into OUT_DIR from the protos; a domain angzarr.toml serves
//! from a child process gets a contract, not an adapter. Written once by
//! `angzarr scaffold startup`.

const PROTOS: &[&str] = &[
    "io/angzarr/examples/v1/table.proto",
    "io/angzarr/examples/v1/player.proto",
    "io/angzarr/examples/v1/policies.proto",
];
const INCLUDES: &[&str] = &["proto"];

fn main() -> Result<(), angzarr_build::BuildError> {
    angzarr_build::app("angzarr.toml", PROTOS, INCLUDES)
}
