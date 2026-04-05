//! `.awf_test` file-based E2E test runner.
//!
//! Each `.awf_test` file under `tests/cases/` is a self-contained,
//! human-readable end-to-end test case. This module parses and executes them.
//!
//! See the plan at `~/.claude/plans/happy-sparking-quokka.md` for the full
//! format spec.

use std::path::Path;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Directive {
    WriteFile { path: String, content: String },
    AssertExists { path: String },
    AssertNotExists { path: String },
    Delete { path: String },
    Export { key: String, value: String },
    Unset { key: String },
    AwfCommand { args: Vec<String>, line_number: usize },
    AssertExitCode { code: i32 },
    AssertStdout { content: Option<String> },
    AssertStderr { content: Option<String> },
}

#[derive(Debug)]
enum Action {
    WriteFile {
        path: String,
        content: String,
    },
    AssertExists {
        path: String,
    },
    AssertNotExists {
        path: String,
    },
    Delete {
        path: String,
    },
    Export {
        key: String,
        value: String,
    },
    Unset {
        key: String,
    },
    RunCommand {
        args: Vec<String>,
        line_number: usize,
        expected_exit_code: i32,
        expected_stdout: Vec<String>,
        expected_stderr: Vec<String>,
    },
}

struct TestCase {
    path: PathBuf,
    actions: Vec<Action>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn strip_inline_comment(line: &str) -> &str {
    line.find(" # ").map_or_else(
        || line.strip_suffix(" #").map_or(line, str::trim_end),
        |idx| line[..idx].trim_end(),
    )
}

fn parse_file(path: &Path) -> TestCase {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let mut directives: Vec<Directive> = Vec::new();

    for (line_idx, raw_line) in content.lines().enumerate() {
        let line_number = line_idx + 1;
        let trimmed = raw_line.trim();

        // Skip blank lines and comment-only lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Content line: `| ` prefix (literal, no comment stripping)
        if raw_line.starts_with("| ") || raw_line == "|" {
            let body = if raw_line == "|" {
                String::new()
            } else {
                raw_line[2..].to_string()
            };
            // Attach to the last directive
            match directives.last_mut() {
                Some(Directive::WriteFile { content, .. }) => {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&body);
                }
                Some(
                    Directive::AssertStdout { content }
                    | Directive::AssertStderr { content },
                ) => {
                    let c = content.get_or_insert_with(String::new);
                    if !c.is_empty() {
                        c.push('\n');
                    }
                    c.push_str(&body);
                }
                _ => panic!(
                    "{}:{}: content line `| ` without a preceding directive that accepts content",
                    path.display(),
                    line_number
                ),
            }
            continue;
        }

        // Directive line: `$ ...`
        if let Some(rest) = trimmed.strip_prefix("$ ") {
            let rest = strip_inline_comment(rest);
            let directive = parse_directive(rest, line_number, path);
            directives.push(directive);
            continue;
        }

        panic!(
            "{}:{}: unexpected line (expected `#`, `$ `, `| `, or blank): {:?}",
            path.display(),
            line_number,
            raw_line
        );
    }

    let actions = group_into_actions(directives, path);
    TestCase { path: path.to_path_buf(), actions }
}

#[allow(clippy::option_if_let_else)]
fn parse_directive(rest: &str, line_number: usize, path: &Path) -> Directive {
    if let Some(arg) = rest.strip_prefix("write_file ") {
        let arg = arg.trim();
        validate_path(arg, line_number, path);
        Directive::WriteFile { path: arg.to_string(), content: String::new() }
    } else if let Some(arg) = rest.strip_prefix("assert_exists ") {
        let arg = arg.trim();
        validate_path(arg, line_number, path);
        Directive::AssertExists { path: arg.to_string() }
    } else if let Some(arg) = rest.strip_prefix("assert_not_exists ") {
        let arg = arg.trim();
        validate_path(arg, line_number, path);
        Directive::AssertNotExists { path: arg.to_string() }
    } else if let Some(arg) = rest.strip_prefix("delete ") {
        let arg = arg.trim();
        validate_path(arg, line_number, path);
        Directive::Delete { path: arg.to_string() }
    } else if let Some(arg) = rest.strip_prefix("export ") {
        let arg = arg.trim();
        let (key, value) = arg.split_once('=').unwrap_or_else(|| {
            panic!(
                "{}:{}: export requires KEY=VALUE",
                path.display(),
                line_number
            )
        });
        Directive::Export { key: key.to_string(), value: value.to_string() }
    } else if let Some(arg) = rest.strip_prefix("unset ") {
        Directive::Unset { key: arg.trim().to_string() }
    } else if let Some(arg) = rest.strip_prefix("awf ") {
        let args: Vec<String> =
            arg.split_whitespace().map(String::from).collect();
        Directive::AwfCommand { args, line_number }
    } else if rest == "awf" {
        Directive::AwfCommand { args: vec![], line_number }
    } else if let Some(arg) = rest.strip_prefix("assert_exit_code ") {
        let code: i32 = arg.trim().parse().unwrap_or_else(|e| {
            panic!("{}:{}: bad exit code: {e}", path.display(), line_number)
        });
        Directive::AssertExitCode { code }
    } else if rest == "assert_stdout" {
        Directive::AssertStdout { content: None }
    } else if rest == "assert_stderr" {
        Directive::AssertStderr { content: None }
    } else {
        panic!(
            "{}:{}: unknown directive: {:?}",
            path.display(),
            line_number,
            rest
        );
    }
}

fn validate_path(p: &str, line_number: usize, file: &Path) {
    assert!(
        !p.starts_with('/'),
        "{}:{}: absolute paths not allowed: {p}",
        file.display(),
        line_number
    );
    assert!(
        !p.contains(".."),
        "{}:{}: `..` in paths not allowed: {p}",
        file.display(),
        line_number
    );
}

fn group_into_actions(directives: Vec<Directive>, path: &Path) -> Vec<Action> {
    let mut actions: Vec<Action> = Vec::new();

    for directive in directives {
        match directive {
            Directive::WriteFile { path: p, content: c } => {
                actions.push(Action::WriteFile { path: p, content: c });
            }
            Directive::AssertExists { path: p } => {
                actions.push(Action::AssertExists { path: p });
            }
            Directive::AssertNotExists { path: p } => {
                actions.push(Action::AssertNotExists { path: p });
            }
            Directive::Delete { path: p } => {
                actions.push(Action::Delete { path: p });
            }
            Directive::Export { key, value } => {
                actions.push(Action::Export { key, value });
            }
            Directive::Unset { key } => actions.push(Action::Unset { key }),
            Directive::AwfCommand { args, line_number } => {
                actions.push(Action::RunCommand {
                    args,
                    line_number,
                    expected_exit_code: 0,
                    expected_stdout: vec![],
                    expected_stderr: vec![],
                });
            }
            Directive::AssertExitCode { code } => {
                let cmd = actions.iter_mut().rev().find_map(|a| match a {
                    Action::RunCommand { expected_exit_code, .. } => {
                        Some(expected_exit_code)
                    }
                    _ => None,
                });
                match cmd {
                    Some(ec) => *ec = code,
                    None => panic!(
                        "{}: assert_exit_code without preceding awf command",
                        path.display()
                    ),
                }
            }
            Directive::AssertStdout { content } => {
                let cmd = actions.iter_mut().rev().find_map(|a| match a {
                    Action::RunCommand { expected_stdout, .. } => {
                        Some(expected_stdout)
                    }
                    _ => None,
                });
                match cmd {
                    Some(es) => {
                        if let Some(c) = content {
                            es.push(c);
                        }
                        // None content with no prior content = assert empty
                        // (default)
                    }
                    None => panic!(
                        "{}: assert_stdout without preceding awf command",
                        path.display()
                    ),
                }
            }
            Directive::AssertStderr { content } => {
                let cmd = actions.iter_mut().rev().find_map(|a| match a {
                    Action::RunCommand { expected_stderr, .. } => {
                        Some(expected_stderr)
                    }
                    _ => None,
                });
                match cmd {
                    Some(es) => {
                        if let Some(c) = content {
                            es.push(c);
                        }
                    }
                    None => panic!(
                        "{}: assert_stderr without preceding awf command",
                        path.display()
                    ),
                }
            }
        }
    }

    actions
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

fn setup_temp_dir() -> TempDir {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".agents/workflows")).unwrap();
    std::fs::create_dir_all(tmp.path().join("flows")).unwrap();
    std::fs::create_dir_all(tmp.path().join("user-home")).unwrap();
    tmp
}

#[allow(clippy::too_many_lines)]
fn run_test_case(case: &TestCase) {
    let tmp = setup_temp_dir();
    let project_dir = tmp.path().to_string_lossy().into_owned();
    let flows_dir = tmp.path().join("flows").to_string_lossy().into_owned();
    let user_dir = tmp.path().join("user-home").to_string_lossy().into_owned();

    let expand = |s: &str| -> String {
        s.replace("$PROJECT_DIR", &project_dir)
            .replace("$FLOWS_DIR", &flows_dir)
            .replace("$USER_DIR", &user_dir)
    };

    let mut env: Vec<(String, String)> = vec![];
    let mut step_number = 0usize;

    for action in &case.actions {
        match action {
            Action::WriteFile { path, content } => {
                let full = tmp.path().join(expand(path));
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(&full, expand(content)).unwrap();
            }
            Action::AssertExists { path } => {
                let full = tmp.path().join(expand(path));
                assert!(
                    full.exists(),
                    "{}: assert_exists failed — path does not exist: {}",
                    case.path.display(),
                    path
                );
            }
            Action::AssertNotExists { path } => {
                let full = tmp.path().join(expand(path));
                assert!(
                    !full.exists(),
                    "{}: assert_not_exists failed — path exists: {}",
                    case.path.display(),
                    path
                );
            }
            Action::Delete { path } => {
                let full = tmp.path().join(expand(path));
                if full.is_dir() {
                    std::fs::remove_dir_all(&full).unwrap_or_else(|e| {
                        panic!(
                            "{}: delete dir failed for {}: {e}",
                            case.path.display(),
                            path
                        )
                    });
                } else {
                    std::fs::remove_file(&full).unwrap_or_else(|e| {
                        panic!(
                            "{}: delete file failed for {}: {e}",
                            case.path.display(),
                            path
                        )
                    });
                }
            }
            Action::Export { key, value } => {
                let expanded_value = expand(value);
                // Update existing or add new
                if let Some(entry) = env.iter_mut().find(|(k, _)| k == key) {
                    entry.1 = expanded_value;
                } else {
                    env.push((key.clone(), expanded_value));
                }
            }
            Action::Unset { key } => {
                env.retain(|(k, _)| k != key);
            }
            Action::RunCommand {
                args,
                line_number,
                expected_exit_code,
                expected_stdout,
                expected_stderr,
            } => {
                step_number += 1;
                let expanded_args: Vec<String> =
                    args.iter().map(|a| expand(a)).collect();

                let output = Command::cargo_bin("awf")
                    .expect("awf binary not found")
                    .current_dir(tmp.path())
                    .envs(env.iter().cloned())
                    .args(&expanded_args)
                    .output()
                    .unwrap_or_else(|e| {
                        panic!(
                            "{} (line {}, step {}): failed to execute awf {}: {e}",
                            case.path.display(),
                            line_number,
                            step_number,
                            expanded_args.join(" ")
                        )
                    });

                let stdout =
                    String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr =
                    String::from_utf8_lossy(&output.stderr).into_owned();
                let exit = output.status.code().unwrap_or(-1);

                let context = || {
                    format!(
                        "\n  file:   {}\n  line:   {}\n  step:   {}\n  cmd:    awf {}\n  exit:   {}\n  stdout: {:?}\n  stderr: {:?}",
                        case.path.display(),
                        line_number,
                        step_number,
                        expanded_args.join(" "),
                        exit,
                        stdout.trim(),
                        stderr.trim(),
                    )
                };

                // Check exit code
                assert_eq!(
                    exit,
                    *expected_exit_code,
                    "exit code mismatch (expected {expected_exit_code}, got {exit}){}",
                    context()
                );

                // Check stdout
                if expected_stdout.is_empty() {
                    // Implicit: assert stdout is empty
                    assert!(
                        stdout.trim().is_empty(),
                        "expected empty stdout but got content{}",
                        context()
                    );
                } else {
                    for expected in expected_stdout {
                        // Try JSON partial match first
                        if let (Ok(expected_json), Ok(actual_json)) = (
                            serde_json::from_str::<serde_json::Value>(expected),
                            serde_json::from_str::<serde_json::Value>(
                                stdout.trim(),
                            ),
                        ) {
                            assert_json_contains(
                                &actual_json,
                                &expected_json,
                                &context,
                            );
                        } else {
                            assert!(
                                stdout.contains(expected.as_str()),
                                "stdout does not contain expected text\n  expected to contain: {:?}{}",
                                expected,
                                context()
                            );
                        }
                    }
                }

                // Check stderr
                if expected_stderr.is_empty() {
                    // Implicit: assert stderr is empty
                    assert!(
                        stderr.trim().is_empty(),
                        "expected empty stderr but got content{}",
                        context()
                    );
                } else {
                    for expected in expected_stderr {
                        assert!(
                            stderr.contains(expected.as_str()),
                            "stderr does not contain expected text\n  expected to contain: {:?}{}",
                            expected,
                            context()
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// JSON partial matching
// ---------------------------------------------------------------------------

fn assert_json_contains(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    context: &dyn Fn() -> String,
) {
    use serde_json::Value;

    match expected {
        Value::String(s) if s == "<any_array>" => {
            assert!(
                actual.is_array(),
                "expected any array, got {actual}{}",
                context()
            );
        }
        Value::String(s) if s == "<any_string>" => {
            assert!(
                actual.is_string(),
                "expected any string, got {actual}{}",
                context()
            );
        }
        Value::String(s) if s == "<any_number>" => {
            assert!(
                actual.is_number(),
                "expected any number, got {actual}{}",
                context()
            );
        }
        Value::Object(expected_map) => {
            let actual_map = actual.as_object().unwrap_or_else(|| {
                panic!("expected object, got {actual}{}", context());
            });
            for (key, expected_val) in expected_map {
                let actual_val = actual_map.get(key).unwrap_or_else(|| {
                    panic!(
                        "missing key {:?} in JSON object\n  actual keys: {:?}{}",
                        key,
                        actual_map.keys().collect::<Vec<_>>(),
                        context()
                    );
                });
                assert_json_contains(actual_val, expected_val, context);
            }
        }
        Value::Array(expected_arr) => {
            let actual_arr = actual.as_array().unwrap_or_else(|| {
                panic!("expected array, got {actual}{}", context());
            });
            assert_eq!(
                actual_arr.len(),
                expected_arr.len(),
                "array length mismatch{}",
                context()
            );
            for (a, e) in actual_arr.iter().zip(expected_arr.iter()) {
                assert_json_contains(a, e, context);
            }
        }
        _ => {
            assert_eq!(actual, expected, "JSON value mismatch{}", context());
        }
    }
}

// ---------------------------------------------------------------------------
// Test entry point
// ---------------------------------------------------------------------------

#[test]
fn awf_test_cases() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    assert!(
        cases_dir.is_dir(),
        "test cases directory not found: {}",
        cases_dir.display()
    );

    let mut test_files: Vec<PathBuf> = std::fs::read_dir(&cases_dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("awf_test") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    test_files.sort();

    assert!(
        !test_files.is_empty(),
        "no .awf_test files found in {}",
        cases_dir.display()
    );

    let mut failures: Vec<(PathBuf, String)> = vec![];

    for path in &test_files {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let case = parse_file(path);
                run_test_case(&case);
            }));
        if let Err(e) = result {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(ToString::to_string))
                .unwrap_or_else(|| "unknown panic".to_string());
            failures.push((path.clone(), msg));
        }
    }

    if !failures.is_empty() {
        let mut report = format!(
            "\n{} of {} test case(s) failed:\n",
            failures.len(),
            test_files.len()
        );
        for (path, msg) in &failures {
            use std::fmt::Write as _;
            let _ = write!(report, "\n--- {} ---\n{msg}\n", path.display());
        }
        panic!("{report}");
    }
}
