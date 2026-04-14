use assert_fs::TempDir;
use assert_fs::prelude::*;

use super::expand_template;
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
    // Given: a temp directory with no markers anywhere in the tree
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
fn finds_root_when_awf_config_exists_in_ancestor() {
    // Given: a temp dir with `.config/awf.toml` and a nested subdir
    let root = TempDir::new().unwrap();
    root.child(".config/awf.toml").touch().unwrap();
    root.child("project/src").create_dir_all().unwrap();

    let start = root.child("project/src").path().to_path_buf();

    // When: we search for the project root
    let result = find_project_root(&start);

    // Then: the directory containing `.config/awf.toml` is returned
    assert_eq!(result.unwrap(), root.path());
}

#[test]
fn finds_root_when_jj_dir_exists_in_ancestor() {
    // Given: a temp dir with a `.jj/` directory and a nested subdir
    let root = TempDir::new().unwrap();
    root.child(".jj").create_dir_all().unwrap();
    root.child("project/src").create_dir_all().unwrap();

    let start = root.child("project/src").path().to_path_buf();

    // When: we search for the project root
    let result = find_project_root(&start);

    // Then: the directory containing `.jj/` is returned
    assert_eq!(result.unwrap(), root.path());
}

#[test]
fn finds_root_when_git_dir_exists_in_ancestor() {
    // Given: a temp dir with a `.git/` directory and a nested subdir
    let root = TempDir::new().unwrap();
    root.child(".git").create_dir_all().unwrap();
    root.child("project/src").create_dir_all().unwrap();

    let start = root.child("project/src").path().to_path_buf();

    // When: we search for the project root
    let result = find_project_root(&start);

    // Then: the directory containing `.git/` is returned
    assert_eq!(result.unwrap(), root.path());
}

#[test]
fn awf_config_takes_priority_over_agents_dir_at_deeper_level() {
    // Given: `.config/awf.toml` at the root and `.agents/` in a subdirectory
    // (so the closer ancestor has `.agents/` but the root has
    // `.config/awf.toml`)
    let root = TempDir::new().unwrap();
    root.child(".config/awf.toml").touch().unwrap();
    root.child("sub/.agents").create_dir_all().unwrap();
    root.child("sub/src").create_dir_all().unwrap();

    let start = root.child("sub/src").path().to_path_buf();

    // When: we search for the project root
    let result = find_project_root(&start);

    // Then: the closer `.agents/` directory wins (it's encountered first while
    // walking up)
    assert_eq!(result.unwrap(), root.child("sub").path());
}

#[test]
fn resolve_flow_path_uses_flag_when_provided() {
    // Given: a flag value and an env var that would also supply a value
    // (to confirm the flag takes priority)
    let tmp = TempDir::new().unwrap();
    let env_flow = tmp.path().join("env-flow");
    let flag_flow = tmp.path().join("flag-flow");

    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::set_var("AWF_ACTIVE_FLOW", &env_flow);
    }
    let config = Config::default();

    // When: resolve_flow_path is called with a flag value
    let result = resolve_flow_path(Some(flag_flow.to_str().unwrap()), &config);

    // SAFETY: single-threaded test; no other thread reads this env var.
    unsafe {
        std::env::remove_var("AWF_ACTIVE_FLOW");
    }

    // Then: the flag value is returned (tilde-expanded)
    assert_eq!(result.unwrap(), flag_flow);
}

#[test]
fn resolve_flow_path_uses_config_active_flow_when_flag_absent() {
    // Given: no flag, but config.active_flow is set
    let tmp = TempDir::new().unwrap();
    let config_flow = tmp.path().join("config-flow");
    let config =
        Config { active_flow: Some(config_flow.clone()), ..Config::default() };

    // When: resolve_flow_path is called with no flag
    let result = resolve_flow_path(None, &config);

    // Then: the config path is returned
    assert_eq!(result.unwrap(), config_flow);
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

// ---------------------------------------------------------------------------
// expand_template
// ---------------------------------------------------------------------------

#[test]
fn expand_template_project_name() {
    // When: template contains {project_name} and the name is provided
    let result =
        expand_template("flows/{project_name}/work", Some("my-project"), None)
            .unwrap();

    // Then: the name is substituted
    assert_eq!(result, "flows/my-project/work");
}

#[test]
fn expand_template_project_name_not_configured_returns_error() {
    // When: {project_name} is used but no project_name is supplied
    let err = expand_template("{project_name}", None, None).unwrap_err();

    assert!(
        matches!(err, super::TemplateError::ProjectNameNotConfigured),
        "expected ProjectNameNotConfigured, got {err:?}",
    );
    assert!(
        err.to_string().contains("project_name"),
        "error message should mention project_name: {err}",
    );
}

#[test]
fn expand_template_project_root() {
    use std::path::Path;

    // When: template contains {project_root} and a root path is provided
    let root = Path::new("/home/user/myproject");
    let result =
        expand_template("prompts/{project_root}/design.md", None, Some(root))
            .unwrap();

    // Then: the path is substituted
    assert_eq!(result, "prompts//home/user/myproject/design.md");
}

#[test]
fn expand_template_project_root_at_start() {
    use std::path::Path;

    // When: {project_root} is the leading component (typical usage)
    let root = Path::new("/home/user/myproject");
    let result =
        expand_template("{project_root}/prompts/design.md", None, Some(root))
            .unwrap();

    // Then: expands to an absolute path
    assert_eq!(result, "/home/user/myproject/prompts/design.md");
}

#[test]
fn expand_template_project_root_not_configured_returns_error() {
    // When: {project_root} is used but no root is supplied
    let err = expand_template("{project_root}", None, None).unwrap_err();

    assert!(
        matches!(err, super::TemplateError::ProjectRootNotConfigured),
        "expected ProjectRootNotConfigured, got {err:?}",
    );
    assert!(
        err.to_string().contains("project_root"),
        "error message should mention project_root: {err}",
    );
}

#[test]
fn expand_template_time_yields_date_shaped_string() {
    // When: template contains {time:YYYY-MM-DD}
    let result =
        expand_template("flows/{time:YYYY-MM-DD}/work", None, None).unwrap();

    // Then: the placeholder is replaced with something matching YYYY-MM-DD
    let date_part = result
        .strip_prefix("flows/")
        .and_then(|s| s.strip_suffix("/work"))
        .expect("unexpected result shape");
    assert!(
        date_part.len() == 10
            && date_part.chars().nth(4) == Some('-')
            && date_part.chars().nth(7) == Some('-'),
        "date part {date_part:?} does not look like YYYY-MM-DD",
    );
}

#[test]
fn expand_template_unknown_variable_returns_error() {
    let err = expand_template("{unknown_var}", None, None).unwrap_err();
    assert!(
        err.to_string().contains("unknown_var"),
        "error message should mention the variable name: {err}",
    );
}

#[test]
fn expand_template_backslash_escapes_braces() {
    // \{ and \} should produce literal braces without triggering expansion
    let result =
        expand_template(r"flows/\{literal\}/work", None, None).unwrap();
    assert_eq!(result, "flows/{literal}/work");
}

#[test]
fn expand_template_unclosed_brace_returns_error() {
    let err = expand_template("flows/{unclosed", None, None).unwrap_err();
    assert!(
        err.to_string().contains("unclosed"),
        "error message should mention unclosed brace: {err}",
    );
}

#[test]
fn expand_template_tilde_left_intact_for_caller() {
    // When: template starts with ~/ (tilde expansion is the caller's job)
    let result =
        expand_template("~/{project_name}/work", Some("proj"), None).unwrap();

    // Then: ~ is preserved
    assert_eq!(result, "~/proj/work");
}
