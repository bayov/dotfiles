use assert_fs::TempDir;
use assert_fs::prelude::*;

use super::find_awf_root;
use super::resolve_state_dir;
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

#[test]
fn resolve_state_dir_uses_env_var_when_set() {
    // Given: A temp directory acting as the awf root, and an explicit state dir
    // path set via the environment variable
    let root = TempDir::new().unwrap();
    let expected = root.child("custom-state").path().to_path_buf();

    // When: AWF_STATE_DIR is set to the custom path
    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::set_var("AWF_STATE_DIR", &expected);
    }
    let result = resolve_state_dir(root.path());
    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::remove_var("AWF_STATE_DIR");
    }

    // Then: The custom path is returned verbatim (no `.awf` suffix appended)
    assert_eq!(result.unwrap(), expected);
}

#[test]
fn resolve_state_dir_uses_config_toml_when_env_var_absent() {
    // Given: A temp directory with a `.awf/config.toml` that names a custom
    // state directory
    let root = TempDir::new().unwrap();
    let configured_state = root.child("shared-state").path().to_path_buf();
    root.child(".awf").create_dir_all().unwrap();
    root.child(".awf/config.toml")
        .write_str(&format!(
            "state_dir = {:?}\n",
            configured_state.to_string_lossy()
        ))
        .unwrap();

    // Ensure the env var is absent so priority 2 is exercised.
    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::remove_var("AWF_STATE_DIR");
    }

    // When: We resolve the state directory
    let result = resolve_state_dir(root.path());

    // Then: The path from config.toml is returned
    assert_eq!(result.unwrap(), configured_state);
}

#[test]
fn resolve_state_dir_falls_back_to_project_local_awf() {
    // Given: A temp directory with no env var and no config.toml
    let root = TempDir::new().unwrap();

    // Ensure the env var is absent so priority 3 is exercised.
    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::remove_var("AWF_STATE_DIR");
    }

    // When: We resolve the state directory
    let result = resolve_state_dir(root.path());

    // Then: The project-local `.awf/` directory is returned
    assert_eq!(result.unwrap(), root.path().join(".awf"));
}
