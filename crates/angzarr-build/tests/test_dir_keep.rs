//! `ANGZARR_KEEP_TEST_DIRS` keeps a panicking test's directory
//! (`test-support/test_dir.rs`). Alone in its binary, as it sets the
//! variable for the process.

use std::panic::{AssertUnwindSafe, catch_unwind};

#[path = "../../../test-support/test_dir.rs"]
mod test_dir;
use test_dir::{KEEP, TestDir};

#[test]
fn the_keep_variable_keeps_a_panicking_tests_directory() {
    // SAFETY: this binary's one test; nothing else reads the environment.
    unsafe { std::env::set_var(KEEP, "1") };
    let mut seen = None;
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let dir = TestDir::new("angzarr-build-test-dir-keep");
        seen = Some(dir.to_path_buf());
        panic!("the test fails while its directory is held");
    }));
    assert!(unwound.is_err(), "the closure panicked");
    let path = seen.expect("the directory was made");
    let kept = path.is_dir();
    let _ = std::fs::remove_dir_all(&path);
    assert!(kept, "{}", path.display());
}
