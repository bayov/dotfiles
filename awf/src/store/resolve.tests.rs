use assert_fs::TempDir;
use assert_fs::prelude::*;

use super::find_project_root;
use super::resolve_flow_path;
use crate::config::Config;
use crate::result::Error;

#[test]
fn finds_root_when_agents_dir_exists_in_ancestor() {
    // Given: a temp directory with a `.agents/` subdir and a nested project dir
    let root = TempDir::new().unwrap();
    root.child(".agents").create_dir_all().unwrap();
    root.child("project/src").create_dir_all().unwrap();

    let start = root.child("project/src").path().to_path_buf();

    // When: we search for the project root starting from the nested directory
    let result = find_project_root(&start);

    // Then: the directory containing `.agents/` is returned
    assert_eq!(result.unwrap(), root.path());
}

#[test]
fn returns_error_when_no_agents_dir_exists() {
    // Given: a temp directory with no `.agents/` anywhere in the tree
    let root = TempDir::new().unwrap();
    root.child("project/src").create_dir_all().unwrap();

    let start = root.child("project/src").path().to_path_buf();

    // When: we search for the project root starting from the nested directory
    let result = find_project_root(&start);

    // Then: ProjectRootNotFound is returned with the original start path
    assert!(
        matches!(result, Err(Error::ProjectRootNotFound { start: ref p }) if p == &start),
        "expected ProjectRootNotFound with start={start:?}, got {result:?}",
    );
}

#[test]
fn resolve_flow_path_uses_flag_when_provided() {
    // Given: a flag value and an env var that would also supply a value
    // (to confirm the flag takes priority)
    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::set_var("AWF_ACTIVE_FLOW", "/tmp/env-flow");
    }
    let config = Config::default();

    // When: resolve_flow_path is called with a flag value
    let result = resolve_flow_path(Some("/tmp/flag-flow"), &config);

    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::remove_var("AWF_ACTIVE_FLOW");
    }

    // Then: the flag value is returned (tilde-expanded)
    assert_eq!(result.unwrap().to_str().unwrap(), "/tmp/flag-flow");
}

#[test]
fn resolve_flow_path_uses_config_active_flow_when_flag_absent() {
    // Given: no flag, but config.active_flow is set
    let config = Config {
        active_flow: Some(std::path::PathBuf::from("/tmp/config-flow")),
        ..Config::default()
    };

    // When: resolve_flow_path is called with no flag
    let result = resolve_flow_path(None, &config);

    // Then: the config path is returned
    assert_eq!(result.unwrap(), std::path::PathBuf::from("/tmp/config-flow"));
}

#[test]
fn resolve_flow_path_returns_error_when_no_source_available() {
    // Given: no flag value and empty config
    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::remove_var("AWF_ACTIVE_FLOW");
    }
    let config = Config::default();

    // When: resolve_flow_path is called with no flag
    let result = resolve_flow_path(None, &config);

    // Then: FlowPathRequired error is returned
    assert!(
        matches!(result, Err(Error::FlowPathRequired)),
        "expected FlowPathRequired, got {result:?}",
    );
}
