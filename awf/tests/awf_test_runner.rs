//! `.awf_test` file-based E2E test runner.
//!
//! Each `.awf_test` file under `tests/cases/` is a self-contained,
//! human-readable end-to-end test case. This module parses and executes them.
//!
//! See the plan at `~/.claude/plans/happy-sparking-quokka.md` for the full
//! format spec.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;
use owo_colors::OwoColorize;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssertMode {
    /// Default: string contains (with auto JSON partial-match detection).
    Contains,
    /// Negated string contains.
    NotContains,
    /// Force JSON partial match (panics on parse failure instead of fallback).
    Json,
    /// Parse stdout as TOML → JSON, then JSON partial match.
    Toml,
    /// Strip `<data>…</data>` XML envelope, parse inner JSON, partial match.
    Xml,
    /// String equality (trimmed), with `<...>` wildcard support.
    ///
    /// The placeholder `<...>` matches any non-empty run of characters within a
    /// single line.  Literal segments between placeholders are matched in
    /// order.  Lines without `<...>` must match exactly.
    Equals,
    /// Assert output is empty (no content lines needed).
    Empty,
}

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
    AssertStdout { mode: AssertMode, content: Option<String> },
    AssertStderr { mode: AssertMode, content: Option<String> },
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
        expected_stdout: Vec<(AssertMode, String)>,
        expected_stderr: Vec<(AssertMode, String)>,
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
                    Directive::AssertStdout { content, .. }
                    | Directive::AssertStderr { content, .. },
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
    } else if let Some(arg) = rest.strip_prefix("assert_stdout ") {
        let mode = parse_assert_mode(arg.trim(), line_number, path);
        Directive::AssertStdout { mode, content: None }
    } else if rest == "assert_stdout" {
        panic!(
            "{}:{}: assert_stdout requires an explicit mode (contains, not_contains, json, toml, xml, equals, empty)",
            path.display(),
            line_number
        )
    } else if let Some(arg) = rest.strip_prefix("assert_stderr ") {
        let mode = parse_assert_mode(arg.trim(), line_number, path);
        Directive::AssertStderr { mode, content: None }
    } else if rest == "assert_stderr" {
        panic!(
            "{}:{}: assert_stderr requires an explicit mode (contains, not_contains, json, toml, xml, equals, empty)",
            path.display(),
            line_number
        )
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

fn parse_assert_mode(
    mode_str: &str,
    line_number: usize,
    file: &Path,
) -> AssertMode {
    match mode_str {
        "contains" => AssertMode::Contains,
        "not_contains" => AssertMode::NotContains,
        "json" => AssertMode::Json,
        "toml" => AssertMode::Toml,
        "xml" => AssertMode::Xml,
        "equals" => AssertMode::Equals,
        "empty" => AssertMode::Empty,
        other => panic!(
            "{}:{}: unknown assert mode: {:?} (expected contains, not_contains, json, toml, xml, equals, empty)",
            file.display(),
            line_number,
            other
        ),
    }
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
            Directive::AssertStdout { mode, content } => {
                let cmd = actions.iter_mut().rev().find_map(|a| match a {
                    Action::RunCommand { expected_stdout, .. } => {
                        Some(expected_stdout)
                    }
                    _ => None,
                });
                match cmd {
                    Some(es) => {
                        if mode == AssertMode::Empty {
                            es.push((AssertMode::Empty, String::new()));
                        } else if let Some(c) = content {
                            es.push((mode, c));
                        }
                        // None content with Contains mode = assert empty
                        // (default, backward compat)
                    }
                    None => panic!(
                        "{}: assert_stdout without preceding awf command",
                        path.display()
                    ),
                }
            }
            Directive::AssertStderr { mode, content } => {
                let cmd = actions.iter_mut().rev().find_map(|a| match a {
                    Action::RunCommand { expected_stderr, .. } => {
                        Some(expected_stderr)
                    }
                    _ => None,
                });
                match cmd {
                    Some(es) => {
                        if mode == AssertMode::Empty {
                            es.push((AssertMode::Empty, String::new()));
                        } else if let Some(c) = content {
                            es.push((mode, c));
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
                    .env("NO_COLOR", "1")
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

                let case_name =
                    case.path.file_stem().unwrap_or_default().to_string_lossy();
                let context = || {
                    let mut s = String::new();
                    let _ = writeln!(
                        s,
                        "\n\n{} {} (step {})",
                        "FAIL".red().bold(),
                        case_name.cyan().bold(),
                        step_number,
                    );
                    let _ = writeln!(
                        s,
                        "  {} {}:{}",
                        "at".dimmed(),
                        case.path.display(),
                        line_number,
                    );
                    let _ = writeln!(
                        s,
                        "  {} awf {}",
                        "cmd".dimmed(),
                        expanded_args.join(" "),
                    );
                    let _ = writeln!(
                        s,
                        "  {} {}",
                        "exit".dimmed(),
                        if exit == *expected_exit_code {
                            exit.to_string().green().to_string()
                        } else {
                            format!("{exit} (expected {expected_exit_code})")
                                .red()
                                .to_string()
                        },
                    );
                    if stdout.trim().is_empty() {
                        let _ = writeln!(
                            s,
                            "  {} {}",
                            "stdout".dimmed(),
                            "(empty)".dimmed(),
                        );
                    } else {
                        let _ = writeln!(s, "  {}:", "stdout".dimmed());
                        for line in stdout.trim().lines().take(30) {
                            let _ = writeln!(s, "    {}", line.yellow());
                        }
                        let line_count = stdout.trim().lines().count();
                        if line_count > 30 {
                            let _ = writeln!(
                                s,
                                "    {} ({} more lines)",
                                "...".dimmed(),
                                line_count - 30,
                            );
                        }
                    }
                    if stderr.trim().is_empty() {
                        let _ = writeln!(
                            s,
                            "  {} {}",
                            "stderr".dimmed(),
                            "(empty)".dimmed(),
                        );
                    } else {
                        let _ = writeln!(s, "  {}:", "stderr".dimmed());
                        for line in stderr.trim().lines().take(15) {
                            let _ = writeln!(s, "    {}", line.red());
                        }
                        let line_count = stderr.trim().lines().count();
                        if line_count > 15 {
                            let _ = writeln!(
                                s,
                                "    {} ({} more lines)",
                                "...".dimmed(),
                                line_count - 15,
                            );
                        }
                    }
                    s
                };

                // Check exit code
                assert_eq!(
                    exit,
                    *expected_exit_code,
                    "{} expected exit code {}, got {}{}",
                    "exit code mismatch".red().bold(),
                    expected_exit_code.green(),
                    exit.red(),
                    context()
                );

                // Check stdout
                if expected_stdout.is_empty() {
                    // Implicit: assert stdout is empty
                    assert!(
                        stdout.trim().is_empty(),
                        "{} expected empty stdout but got content\n  {}: {:?}{}",
                        "implicit assert".red().bold(),
                        "actual".dimmed(),
                        stdout.trim(),
                        context()
                    );
                } else {
                    for (mode, expected) in expected_stdout {
                        check_output_assertion(
                            "stdout", &stdout, *mode, expected, &context,
                        );
                    }
                }

                // Check stderr
                if expected_stderr.is_empty() {
                    // Implicit: assert stderr is empty
                    assert!(
                        stderr.trim().is_empty(),
                        "{} expected empty stderr but got content\n  {}: {:?}{}",
                        "implicit assert".red().bold(),
                        "actual".dimmed(),
                        stderr.trim(),
                        context()
                    );
                } else {
                    for (mode, expected) in expected_stderr {
                        check_output_assertion(
                            "stderr", &stderr, *mode, expected, &context,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Output assertion dispatch
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn check_output_assertion(
    stream: &str,
    actual: &str,
    mode: AssertMode,
    expected: &str,
    context: &dyn Fn() -> String,
) {
    match mode {
        AssertMode::Contains => {
            // Backward compat: auto-detect JSON when both sides parse.
            if let (Ok(expected_json), Ok(actual_json)) = (
                serde_json::from_str::<serde_json::Value>(expected),
                serde_json::from_str::<serde_json::Value>(actual.trim()),
            ) {
                assert_json_contains(&actual_json, &expected_json, context);
            } else {
                assert!(
                    actual.contains(expected),
                    "{} {} does not contain expected text\n  {}: {:?}\n  {}: {:?}{}",
                    "assert_stdout contains".red().bold(),
                    stream,
                    "expected".dimmed(),
                    expected,
                    "actual".dimmed(),
                    actual.trim(),
                    context()
                );
            }
        }
        AssertMode::NotContains => {
            assert!(
                !actual.contains(expected),
                "{} {} contains text that should be absent\n  {}: {:?}\n  {}: {:?}{}",
                "assert_stdout not_contains".red().bold(),
                stream,
                "unwanted".dimmed(),
                expected,
                "actual".dimmed(),
                actual.trim(),
                context()
            );
        }
        AssertMode::Json => {
            let expected_json: serde_json::Value =
                serde_json::from_str(expected).unwrap_or_else(|e| {
                    panic!(
                        "expected content is not valid JSON: {e}\n  content: {expected:?}{}",
                        context()
                    );
                });
            let actual_json: serde_json::Value =
                serde_json::from_str(actual.trim()).unwrap_or_else(|e| {
                    panic!(
                        "{stream} is not valid JSON: {e}\n  {stream}: {:?}{}",
                        actual.trim(),
                        context()
                    );
                });
            assert_json_contains(&actual_json, &expected_json, context);
        }
        AssertMode::Toml => {
            let expected_json: serde_json::Value =
                serde_json::from_str(expected).unwrap_or_else(|e| {
                    panic!(
                        "expected content is not valid JSON: {e}\n  content: {expected:?}{}",
                        context()
                    );
                });
            let toml_val: toml::Value =
                actual.trim().parse().unwrap_or_else(|e| {
                    panic!(
                        "{stream} is not valid TOML: {e}\n  {stream}: {:?}{}",
                        actual.trim(),
                        context()
                    );
                });
            let actual_json = serde_json::to_value(&toml_val)
                .expect("TOML value to serde_json::Value");
            assert_json_contains(&actual_json, &expected_json, context);
        }
        AssertMode::Xml => {
            let trimmed = actual.trim();
            assert!(
                trimmed.starts_with("<data>\n"),
                "{} {stream} XML output must start with '<data>\\n'\n  {}: {:?}{}",
                "assert xml".red().bold(),
                "got".dimmed(),
                &trimmed[..trimmed.len().min(80)],
                context()
            );
            assert!(
                trimmed.ends_with("</data>"),
                "{} {stream} XML output must end with '</data>'\n  {}: {:?}{}",
                "assert xml".red().bold(),
                "got".dimmed(),
                &trimmed[trimmed.len().saturating_sub(80)..],
                context()
            );
            let inner = trimmed
                .trim_start_matches("<data>\n")
                .trim_end_matches("</data>")
                .trim();
            let expected_json: serde_json::Value =
                serde_json::from_str(expected).unwrap_or_else(|e| {
                    panic!(
                        "expected content is not valid JSON: {e}\n  content: {expected:?}{}",
                        context()
                    );
                });
            let actual_json: serde_json::Value =
                serde_json::from_str(inner).unwrap_or_else(|e| {
                    panic!(
                        "{stream} XML inner content is not valid JSON: {e}\n  inner: {inner:?}{}",
                        context()
                    );
                });
            assert_json_contains(&actual_json, &expected_json, context);
        }
        AssertMode::Equals => {
            assert_equals_with_placeholders(
                actual.trim(),
                expected.trim(),
                stream,
                context,
            );
        }
        AssertMode::Empty => {
            assert!(
                actual.trim().is_empty(),
                "{} expected empty {stream} but got content\n  {}: {:?}{}",
                "assert empty".red().bold(),
                "actual".dimmed(),
                actual.trim(),
                context()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Equals with `<...>` placeholders
// ---------------------------------------------------------------------------

/// Compares `actual` against `expected` line-by-line.
///
/// Lines in `expected` that contain no `<...>` placeholder must match the
/// corresponding `actual` line exactly.  When `<...>` appears, the line is
/// split into literal segments around each placeholder and matched
/// left-to-right: each literal segment must appear in order, and each `<...>`
/// must consume at least one character.
fn assert_equals_with_placeholders(
    actual: &str,
    expected: &str,
    stream: &str,
    context: &dyn Fn() -> String,
) {
    let actual_lines: Vec<&str> = actual.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();

    if !expected.contains("<...>") {
        // Fast path: no placeholders, plain equality.
        assert_eq!(
            actual,
            expected,
            "{} {stream} does not equal expected text{}",
            "assert equals".red().bold(),
            context()
        );
        return;
    }

    assert_eq!(
        actual_lines.len(),
        expected_lines.len(),
        "{} {stream} line count mismatch (expected {}, got {})\n  \
         {}: {:?}\n  {}: {:?}{}",
        "assert equals".red().bold(),
        expected_lines.len(),
        actual_lines.len(),
        "expected".dimmed(),
        expected,
        "actual".dimmed(),
        actual,
        context()
    );

    for (i, (actual_line, expected_line)) in
        actual_lines.iter().zip(expected_lines.iter()).enumerate()
    {
        if !expected_line.contains("<...>") {
            assert_eq!(
                actual_line,
                expected_line,
                "{} {stream} line {} differs\n  {}: {:?}\n  {}: {:?}{}",
                "assert equals".red().bold(),
                i + 1,
                "expected".dimmed(),
                expected_line,
                "actual".dimmed(),
                actual_line,
                context()
            );
            continue;
        }
        assert!(
            line_matches_pattern(actual_line, expected_line),
            "{} {stream} line {} does not match pattern\n  {}: {:?}\n  {}: {:?}{}",
            "assert equals".red().bold(),
            i + 1,
            "pattern".dimmed(),
            expected_line,
            "actual".dimmed(),
            actual_line,
            context()
        );
    }
}

/// Returns `true` if `actual` matches `pattern` where `<...>` segments each
/// consume one or more characters and literal segments match exactly.
fn line_matches_pattern(actual: &str, pattern: &str) -> bool {
    let segments: Vec<&str> = pattern.split("<...>").collect();
    let placeholder_count = segments.len() - 1;

    // No placeholders — exact match (shouldn't reach here, but be safe).
    if placeholder_count == 0 {
        return actual == pattern;
    }

    let mut pos = 0usize;

    for (i, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            // Leading, trailing, or consecutive `<...>` — the placeholder still
            // needs to consume ≥1 char, so just ensure we haven't exhausted
            // the string between required literal anchors.
            if i > 0 && i < segments.len() - 1 {
                // Consecutive `<...><...>`: each must consume ≥1 char.
                // We'll enforce this by requiring at least one char gap per
                // remaining placeholder when we reach the next literal.
            }
            continue;
        }

        if i == 0 {
            // First segment: must be a prefix.
            if !actual[pos..].starts_with(segment) {
                return false;
            }
            pos += segment.len();
        } else {
            // Non-first segment: find it after pos, ensuring ≥1 char gap for
            // the preceding `<...>`.
            let search_start = pos + 1; // at least 1 char for <...>
            if search_start > actual.len() {
                return false;
            }
            match actual[search_start..].find(segment) {
                Some(offset) => pos = search_start + offset + segment.len(),
                None => return false,
            }
        }
    }

    // If the pattern ends with a literal (last segment non-empty), `pos`
    // should be at the end.  If it ends with `<...>`, there must be ≥1 char
    // remaining.
    let last_segment = segments.last().unwrap();
    if last_segment.is_empty() {
        // Pattern ends with `<...>` — need ≥1 char remaining.
        pos < actual.len()
    } else {
        // Pattern ends with a literal — must have consumed everything.
        // But we found the segment by searching forward, so check the actual
        // end.
        actual.ends_with(last_segment) && pos == actual.len()
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
// Test entry point (custom harness via libtest-mimic)
// ---------------------------------------------------------------------------

fn main() {
    let args = libtest_mimic::Arguments::from_args();

    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    assert!(
        cases_dir.is_dir(),
        "test cases directory not found: {}",
        cases_dir.display()
    );

    let mut test_files: Vec<PathBuf> = walkdir::WalkDir::new(&cases_dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.into_path();
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

    let trials: Vec<libtest_mimic::Trial> = test_files
        .into_iter()
        .map(|path| {
            let relative = path.strip_prefix(&cases_dir).unwrap_or(&path);
            let name = relative
                .with_extension("")
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            libtest_mimic::Trial::test(name, move || {
                let case = parse_file(&path);
                run_test_case(&case);
                Ok(())
            })
        })
        .collect();

    libtest_mimic::run(&args, trials).exit();
}
