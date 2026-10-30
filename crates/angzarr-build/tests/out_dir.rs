//! What a build script calls with no arguments but its protos: the output
//! directory and the version are Cargo's. Alone in its binary, as it sets
//! `OUT_DIR` for the process.

use std::path::Path;

#[test]
fn test_descriptors_writes_into_out_dir_with_the_crates_version() {
    let out = std::env::temp_dir().join(format!("angzarr-build-out-dir-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    // SAFETY: this binary's one test; nothing else reads the environment.
    unsafe { std::env::set_var("OUT_DIR", &out) };
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../angzarr-codegen/tests/fixtures")
        .display()
        .to_string();
    angzarr_build::descriptors(&["orders/v1/orders.proto"], &[&fixtures], &[]).unwrap();
    let source = std::fs::read_to_string(out.join("angzarr-descriptors.rs")).unwrap();
    // The doc names the version, and so do the bytes.
    assert!(
        source.contains(&format!("{:?}", env!("CARGO_PKG_VERSION"))),
        "{source}"
    );
    assert!(source.contains("AuditTrail"), "{source}");
    std::fs::remove_dir_all(&out).unwrap();
}
