//! A test directory is fresh: a leftover at its name (a reused process id's
//! debris) is cleared first (`test-support/test_dir.rs`). Alone in its
//! binary, as it predicts the binary's next directory name.

#[path = "../../../test-support/test_dir.rs"]
mod test_dir;
use test_dir::TestDir;

#[test]
fn a_leftover_at_a_test_dirs_name_is_cleared() {
    let first = TestDir::new("angzarr-build-test-dir-fresh");
    let name = first.file_name().unwrap().to_string_lossy().into_owned();
    let (stem, n) = name.rsplit_once('-').unwrap();
    let next: usize = n.parse::<usize>().unwrap() + 1;
    let leftover = first.with_file_name(format!("{stem}-{next}"));
    std::fs::create_dir_all(&leftover).unwrap();
    std::fs::write(leftover.join("debris"), "x").unwrap();
    let second = TestDir::new("angzarr-build-test-dir-fresh");
    assert_eq!(second.path(), leftover);
    assert!(std::fs::read_dir(&second).unwrap().next().is_none());
}
