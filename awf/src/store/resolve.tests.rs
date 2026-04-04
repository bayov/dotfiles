use assert_fs::TempDir;
use assert_fs::prelude::*;

use super::find_awf_root;
use crate::result::Error;

#[test]
fn finds_root_when_awf_dir_exists_in_ancestor() {
    // Given: A temp directory with a `.awf/` subdir and a nested project dir
    let root = TempDir::new().unwrap();
    root.child(".awf").create_dir_all().unwrap();
    root.child("project/src").create_dir_all().unwrap();

    let start = root.child("project/src").path().to_path_buf();

    // When: We search for the awf root starting from the nested directory
    let result = find_awf_root(&start);

    // Then: The directory containing `.awf/` is returned
    assert_eq!(result.unwrap(), root.path());
}

#[test]
fn returns_error_when_no_awf_dir_exists() {
    // Given: A temp directory with no `.awf/` anywhere in the tree
    let root = TempDir::new().unwrap();
    root.child("project/src").create_dir_all().unwrap();

    let start = root.child("project/src").path().to_path_buf();

    // When: We search for the awf root starting from the nested directory
    let result = find_awf_root(&start);

    // Then: AwfRootNotFound is returned with the original start path
    assert!(
        matches!(result, Err(Error::AwfRootNotFound { start: ref p }) if p == &start),
        "expected AwfRootNotFound with start={start:?}, got {result:?}",
    );
}
