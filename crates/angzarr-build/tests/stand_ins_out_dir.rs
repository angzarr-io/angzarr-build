//! `stand_ins`, with no arguments but its protos: the output directory is
//! Cargo's. Alone in its binary, as it sets `OUT_DIR` for the process.

use std::path::Path;

#[test]
fn test_stand_ins_writes_into_out_dir() {
    let out = std::env::temp_dir().join(format!("angzarr-build-stand-ins-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    // SAFETY: this binary's one test; nothing else reads the environment.
    unsafe { std::env::set_var("OUT_DIR", &out) };
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../angzarr-codegen/tests/fixtures")
        .display()
        .to_string();
    angzarr_build::stand_ins(&["buyin/v1/wallet.proto"], &[&fixtures]).unwrap();
    let source = std::fs::read_to_string(out.join("angzarr-stand-ins.rs")).unwrap();
    assert!(source.contains("impl CommandTarget for Wallet"), "{source}");
    std::fs::remove_dir_all(&out).unwrap();
}
