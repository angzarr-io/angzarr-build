//! The workspace's test directories (`test-support/test_dir.rs`) are
//! removed when their guard drops, on a pass and on a panic, and kept only
//! for a panic with keeping asked for.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

#[path = "../../../test-support/test_dir.rs"]
mod test_dir;
use test_dir::TestDir;

/// `TestDir::keeping_on_panic(prefix, keep)`, its path recorded, then a
/// panic while it is held: the path, the guard dropped in the unwind.
fn panicking_with(prefix: &str, keep: bool) -> PathBuf {
    let mut seen = None;
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let dir = TestDir::keeping_on_panic(prefix, keep);
        std::fs::write(dir.join("file"), "x").unwrap();
        seen = Some(dir.to_path_buf());
        panic!("the test fails while its directory is held");
    }));
    assert!(unwound.is_err(), "the closure panicked");
    seen.expect("the directory was made")
}

#[test]
fn a_test_dir_is_fresh_and_empty_under_the_temporary_directory() {
    let dir = TestDir::new("angzarr-build-test-dir");
    assert!(dir.is_dir());
    assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
    assert_eq!(dir.parent(), Some(std::env::temp_dir().as_path()));
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let pid = format!("angzarr-build-test-dir-{}-", std::process::id());
    assert!(name.starts_with(&pid), "{name}");
}

#[test]
fn a_test_dir_and_its_contents_go_when_it_drops() {
    let dir = TestDir::new("angzarr-build-test-dir");
    std::fs::create_dir_all(dir.join("a/b")).unwrap();
    std::fs::write(dir.join("a/b/file"), "x").unwrap();
    let path = dir.to_path_buf();
    drop(dir);
    assert!(!path.exists(), "{}", path.display());
}

#[test]
fn a_test_dir_goes_when_its_test_panics() {
    let path = panicking_with("angzarr-build-test-dir-panic", false);
    assert!(!path.exists(), "{}", path.display());
}

#[test]
fn a_test_dir_kept_on_panic_stays_when_its_test_panics() {
    let path = panicking_with("angzarr-build-test-dir-kept", true);
    let kept = path.join("file").is_file();
    std::fs::remove_dir_all(&path).unwrap();
    assert!(kept, "{}", path.display());
}

#[test]
fn a_test_dir_kept_on_panic_still_goes_when_its_test_passes() {
    let dir = TestDir::keeping_on_panic("angzarr-build-test-dir-kept", true);
    let path = dir.to_path_buf();
    drop(dir);
    assert!(!path.exists(), "{}", path.display());
}
