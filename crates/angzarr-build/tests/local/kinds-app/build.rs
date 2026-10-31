//! The wire types, the markup domain layer, the adapters and register(),
//! generated from the kinds fixture proto into OUT_DIR.

const PROTOS: &[&str] = &["kinds/v1/kinds.proto"];
const INCLUDES: &[&str] = &["../../../../angzarr-codegen/tests/fixtures"];

fn main() -> Result<(), angzarr_build::BuildError> {
    angzarr_build::wire(PROTOS, INCLUDES)?;
    angzarr_build::markup_domain(PROTOS, INCLUDES, &[("kinds.v1", "crate::wire")])?;
    angzarr_build::adapters(PROTOS, INCLUDES, &[])?;
    angzarr_build::register(PROTOS, INCLUDES, &[])
}
