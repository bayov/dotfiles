//! Handlers for `awf flow tasks` subcommands.
//!
//! These commands operate on individual tasks within a `task-runner` stage.

use std::collections::HashMap;
use std::env;
use std::fmt;

use chrono::Utc;
use owo_colors::OwoColorize as _;

use crate::cli::output::eprint_error;
use crate::cli::output::eprint_warning;
use crate::cli::output::print_output;
use crate::engine::command::run_command;
use crate::engine::task_runner;
use crate::model::flow::TaskListStatus;
use crate::model::flow::TaskLogEntry;
use crate::model::flow::TaskLogKind;
use crate::model::flow::TaskLogStatus;
use crate::model::task_list::TaskKind;
use crate::model::task_list::TaskList;
use crate::model::workflow::StageKind;
use crate::result::Error;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single task's display state for `awf flow tasks status` and `tasks next`.
#[derive(Debug)]
pub struct TaskDisplay {
    pub task_id: String,
    pub description: String,
    pub kind: &'static str,
    pub status: TaskDisplayStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TaskDisplayStatus {
    Done,
    Failed,
    Skipped,
    Next,
    Pending,
}

/// Output for `awf flow tasks status`.
#[derive(Debug)]
pub struct TasksStatusOutput {
    pub stage: String,
    pub lists: Vec<ListStatusDisplay>,
}

#[derive(Debug)]
pub struct ListStatusDisplay {
    pub name: String,
    pub status: TaskListStatus,
    pub blocked_by: Vec<String>,
    pub tasks: Vec<TaskDisplay>,
}

impl fmt::Display for TasksStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Task-runner stage: {}", self.stage.magenta())?;
        writeln!(f)?;
        for list in &self.lists {
            let status_label = match list.status {
                TaskListStatus::Pending => "pending".bright_black().to_string(),
                TaskListStatus::InProgress => "in-progress".blue().to_string(),
                TaskListStatus::Complete => "complete".green().to_string(),
                TaskListStatus::Blocked => "blocked".red().to_string(),
            };
            writeln!(f, "  {} [{}]", list.name.magenta(), status_label)?;
            if !list.blocked_by.is_empty() {
                writeln!(
                    f,
                    "    Waiting on: {}",
                    list.blocked_by.join(", ").red()
                )?;
            }
            for task in &list.tasks {
                let (marker, desc_color): (String, String) = match task.status {
                    TaskDisplayStatus::Done => (
                        "✓".green().to_string(),
                        task.description.bright_black().to_string(),
                    ),
                    TaskDisplayStatus::Skipped => (
                        "○".bright_black().to_string(),
                        task.description.bright_black().to_string(),
                    ),
                    TaskDisplayStatus::Failed => (
                        "✗".red().to_string(),
                        task.description.red().to_string(),
                    ),
                    TaskDisplayStatus::Next => (
                        "▶".blue().to_string(),
                        task.description.blue().to_string(),
                    ),
                    TaskDisplayStatus::Pending => {
                        (" ".to_string(), task.description.clone())
                    }
                };
                writeln!(
                    f,
                    "    {marker} {} — {desc_color} [{}]",
                    task.task_id.cyan(),
                    task.kind,
                )?;
            }
        }
        Ok(())
    }
}

/// Output for `awf flow tasks next`.
#[derive(Debug)]
pub struct TasksNextOutput {
    pub flow_path: String,
    pub list_name: String,
    pub task_id: String,
    pub description: String,
    pub kind: &'static str,
}

impl fmt::Display for TasksNextOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} {}/{}: {}",
            "▶".blue(),
            self.list_name.magenta(),
            self.task_id.cyan(),
            self.description,
        )?;
        writeln!(f)?;
        match self.kind {
            "agent" => {
                writeln!(f, "When done, mark it complete with:")?;
                writeln!(
                    f,
                    "  awf flow tasks done {} {} --flow={}",
                    self.list_name.magenta(),
                    self.task_id.cyan(),
                    self.flow_path.cyan(),
                )?;
            }
            "command" => {
                writeln!(f, "Run the command with:")?;
                writeln!(
                    f,
                    "  awf flow tasks run {} {} --flow={}",
                    self.list_name.magenta(),
                    self.task_id.cyan(),
                    self.flow_path.cyan(),
                )?;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Output for `awf flow tasks run`.
#[derive(Debug)]
pub struct TasksRunOutput {
    pub list_name: String,
    pub task_id: String,
    pub command: String,
    pub status: &'static str,
    pub duration_ms: u64,
    pub flow_path: String,
}

impl fmt::Display for TasksRunOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let badge = if self.status == "pass" {
            "PASS".green().to_string()
        } else {
            "FAIL".red().to_string()
        };
        writeln!(
            f,
            "{badge}  {}/{}: {}  ({}ms)",
            self.list_name.magenta(),
            self.task_id.cyan(),
            self.command,
            self.duration_ms,
        )?;
        if self.status == "fail" {
            writeln!(
                f,
                "Re-run with: awf flow tasks run {} {} --flow={}",
                self.list_name.magenta(),
                self.task_id.cyan(),
                self.flow_path.cyan(),
            )?;
        }
        Ok(())
    }
}

/// Simple one-line confirmation output shared by `done`, `skip`, `advance`.
#[derive(Debug)]
pub struct TasksConfirmOutput {
    pub message: String,
}

impl fmt::Display for TasksConfirmOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {}", "✓".green(), self.message)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `awf flow tasks status [--flow=<path>]`
pub fn tasks_status(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let (flow, workflow, flow_dir, entries, config) = load_context(flow_flag)?;

    let stage = require_task_runner_stage(&workflow, &flow, &flow_dir);
    let StageKind::TaskRunner { task_lists, .. } = &stage.kind else {
        unreachable!()
    };

    let loaded = load_all_task_lists(flow_dir.as_path(), task_lists);

    let mut lists = Vec::new();
    for path in task_lists {
        let status = flow
            .task_runner_state
            .get(path)
            .copied()
            .unwrap_or(TaskListStatus::Pending);

        let Some(tl) = loaded.get(path) else {
            lists.push(ListStatusDisplay {
                name: path.clone(),
                status,
                blocked_by: vec![],
                tasks: vec![],
            });
            continue;
        };

        let blocked_by = if status == TaskListStatus::Blocked {
            tl.depends_on
                .iter()
                .filter(|dep| {
                    let dep_complete = loaded.values().any(|l| {
                        &l.name == *dep
                            && flow
                                .task_runner_state
                                .iter()
                                .find(|(p, _)| {
                                    loaded
                                        .get(p.as_str())
                                        .is_some_and(|l2| l2.name == l.name)
                                })
                                .is_some_and(|(_, s)| {
                                    *s == TaskListStatus::Complete
                                })
                    });
                    !dep_complete
                })
                .cloned()
                .collect()
        } else {
            vec![]
        };

        let tasks = build_task_displays(tl, &flow.current_stage, &flow);
        lists.push(ListStatusDisplay {
            name: tl.name.clone(),
            status,
            blocked_by,
            tasks,
        });
    }

    let _ = entries;
    let _ = config;
    let output = TasksStatusOutput { stage: flow.current_stage.clone(), lists };
    print_output(&output);
    Ok(())
}

/// `awf flow tasks next [--list=<name>] [--flow=<path>]`
pub fn tasks_next(
    flow_flag: Option<&str>,
    list_name: Option<&str>,
) -> color_eyre::Result<()> {
    let (flow, workflow, flow_dir, _entries, _config) =
        load_context(flow_flag)?;

    let stage = require_task_runner_stage(&workflow, &flow, &flow_dir);
    let StageKind::TaskRunner { task_lists, parallel } = &stage.kind else {
        unreachable!()
    };

    let loaded = load_all_task_lists(flow_dir.as_path(), task_lists);

    // If a specific list is requested, search only there.
    if let Some(name) = list_name {
        tasks_next_named(&flow, &flow_dir, task_lists, &loaded, name);
        return Ok(());
    }

    // No specific list: find the first pending task in any unblocked list.
    // In serial mode, only consider the first non-complete list.
    let candidate_paths: Vec<&str> = task_lists
        .iter()
        .filter(|p| {
            let status = flow
                .task_runner_state
                .get(p.as_str())
                .copied()
                .unwrap_or(TaskListStatus::Pending);
            !matches!(
                status,
                TaskListStatus::Complete | TaskListStatus::Blocked
            )
        })
        .map(String::as_str)
        .collect();

    // In serial mode, only consider the first candidate list.
    let candidates: &[&str] = if *parallel {
        &candidate_paths
    } else {
        candidate_paths.get(..1).unwrap_or(&[])
    };

    for path in candidates {
        let Some(tl) = loaded.get(*path) else { continue };
        let Some(task) =
            task_runner::next_task_in_list(tl, &flow.current_stage, &flow)
        else {
            continue;
        };
        let output = TasksNextOutput {
            flow_path: flow_dir.display().to_string(),
            list_name: tl.name.clone(),
            task_id: task.id.clone(),
            description: task.description.clone(),
            kind: task_kind_str(&task.kind),
        };
        print_output(&output);
        return Ok(());
    }

    handle_no_pending_tasks(&flow, task_lists, &loaded, &flow_dir);
    Ok(())
}

fn tasks_next_named(
    flow: &crate::model::flow::FlowState,
    flow_dir: &std::path::Path,
    task_lists: &[String],
    loaded: &HashMap<String, TaskList>,
    name: &str,
) {
    let (path, tl) =
        find_list_by_name_or_exit(name, task_lists, loaded, flow_dir);

    let list_status = flow
        .task_runner_state
        .get(path)
        .copied()
        .unwrap_or(TaskListStatus::Pending);
    if list_status == TaskListStatus::Blocked {
        eprint_error(format_args!(
            "list {name:?} is blocked by unmet dependencies. \
             Use `awf flow tasks status` to see which lists must complete first.",
        ));
        std::process::exit(1);
    }

    let Some(task) =
        task_runner::next_task_in_list(tl, &flow.current_stage, flow)
    else {
        eprint_error(format_args!(
            "no pending tasks in list {name:?}. \
             Run `awf flow tasks advance {name}` if all tasks are done.",
        ));
        std::process::exit(1);
    };
    let output = TasksNextOutput {
        flow_path: flow_dir.display().to_string(),
        list_name: tl.name.clone(),
        task_id: task.id.clone(),
        description: task.description.clone(),
        kind: task_kind_str(&task.kind),
    };
    print_output(&output);
    let _ = path;
}

fn handle_no_pending_tasks(
    flow: &crate::model::flow::FlowState,
    task_lists: &[String],
    loaded: &HashMap<String, TaskList>,
    flow_dir: &std::path::Path,
) {
    // Check if any unadvanced list has all tasks done → suggest advance.
    let ready_to_advance: Vec<&str> = task_lists
        .iter()
        .filter_map(|p| {
            let status = flow
                .task_runner_state
                .get(p)
                .copied()
                .unwrap_or(TaskListStatus::Pending);
            if status == TaskListStatus::Complete {
                return None;
            }
            let tl = loaded.get(p.as_str())?;
            if task_runner::all_tasks_done(tl, &flow.current_stage, flow) {
                Some(tl.name.as_str())
            } else {
                None
            }
        })
        .collect();

    if !ready_to_advance.is_empty() {
        for name in &ready_to_advance {
            anstream::println!(
                "All tasks in list {:?} are done. Advance with: {}",
                name,
                format!(
                    "awf flow tasks advance {name} --flow={}",
                    flow_dir.display()
                )
                .cyan()
            );
        }
        return;
    }

    // All unblocked lists are either done or blocked by a failing task.
    let any_blocked =
        flow.task_runner_state.values().any(|s| *s == TaskListStatus::Blocked);
    if any_blocked {
        eprint_warning("some lists are blocked by unmet dependencies");
    }
    let all_done = task_lists.iter().all(|p| {
        flow.task_runner_state
            .get(p)
            .is_some_and(|s| *s == TaskListStatus::Complete)
    });
    if all_done {
        anstream::println!(
            "All task lists complete. Advance with: {}",
            format!("awf flow next-stage --flow={}", flow_dir.display()).cyan()
        );
    } else {
        eprint_error(
            "no actionable tasks found. Check `awf flow tasks status` for blocked or failed tasks.",
        );
        std::process::exit(1);
    }
}

/// `awf flow tasks run <list> <task-id> [--flow=<path>]`
pub fn tasks_run(
    flow_flag: Option<&str>,
    list_name: &str,
    task_id: &str,
) -> color_eyre::Result<()> {
    let (mut flow, workflow, flow_dir, _entries, config) =
        load_context(flow_flag)?;

    let stage = require_task_runner_stage(&workflow, &flow, &flow_dir);
    let StageKind::TaskRunner { task_lists, .. } = &stage.kind else {
        unreachable!()
    };
    let stage_name = stage.name.clone();

    let loaded = load_all_task_lists(flow_dir.as_path(), task_lists);
    let (_, tl) =
        find_list_by_name_or_exit(list_name, task_lists, &loaded, &flow_dir);

    let task = tl.tasks.iter().find(|t| t.id == task_id).unwrap_or_else(|| {
        eprint_error(format_args!(
            "task {task_id:?} not found in list {list_name:?}",
        ));
        std::process::exit(1);
    });

    let TaskKind::Command { command: ref key } = task.kind else {
        eprint_error(format_args!(
            "task {list_name:?}/{task_id:?} is an agent task; use `awf flow tasks done` instead",
        ));
        std::process::exit(1);
    };

    let shell_cmd = workflow
        .commands
        .get(key.as_str())
        .or_else(|| config.commands.get(key.as_str()))
        .cloned()
        .unwrap_or_else(|| {
            eprint_error(format_args!(
                "command key {key:?} not found in workflow [commands]",
            ));
            std::process::exit(1);
        });

    let result = run_command(&shell_cmd)?;
    let log_status = if result.exit_code == 0 {
        TaskLogStatus::Pass
    } else {
        TaskLogStatus::Fail
    };

    flow.task_log.push(TaskLogEntry {
        list: tl.name.clone(),
        task_id: task_id.to_string(),
        stage: stage_name,
        kind: TaskLogKind::Command,
        status: log_status,
        completed_at: Some(Utc::now()),
        duration_ms: Some(result.duration_ms),
        exit_code: Some(result.exit_code),
        skip_reason: None,
    });

    // Mark list as in-progress if it was pending.
    if flow
        .task_runner_state
        .get(tl.name.as_str())
        .is_some_and(|s| *s == TaskListStatus::Pending)
    {
        // The state key is a path, not a name; look it up.
        update_list_to_in_progress(&mut flow, list_name, &loaded);
    }

    flow.updated_at = Utc::now();
    fs::save_flow(&flow_dir, &flow)?;

    let status_str = if result.exit_code == 0 { "pass" } else { "fail" };
    let output = TasksRunOutput {
        list_name: tl.name.clone(),
        task_id: task_id.to_string(),
        command: shell_cmd,
        status: status_str,
        duration_ms: result.duration_ms,
        flow_path: flow_dir.display().to_string(),
    };
    print_output(&output);
    Ok(())
}

/// `awf flow tasks done <list> <task-id> [--flow=<path>]`
pub fn tasks_done(
    flow_flag: Option<&str>,
    list_name: &str,
    task_id: &str,
) -> color_eyre::Result<()> {
    let (mut flow, workflow, flow_dir, _entries, _config) =
        load_context(flow_flag)?;

    let stage = require_task_runner_stage(&workflow, &flow, &flow_dir);
    let StageKind::TaskRunner { task_lists, .. } = &stage.kind else {
        unreachable!()
    };
    let stage_name = stage.name.clone();

    let loaded = load_all_task_lists(flow_dir.as_path(), task_lists);
    let (_, tl) =
        find_list_by_name_or_exit(list_name, task_lists, &loaded, &flow_dir);

    let task = tl.tasks.iter().find(|t| t.id == task_id).unwrap_or_else(|| {
        eprint_error(format_args!(
            "task {task_id:?} not found in list {list_name:?}",
        ));
        std::process::exit(1);
    });

    if !matches!(task.kind, TaskKind::Agent) {
        eprint_error(format_args!(
            "task {list_name:?}/{task_id:?} is a command task; use `awf flow tasks run` instead",
        ));
        std::process::exit(1);
    }

    flow.task_log.push(TaskLogEntry {
        list: tl.name.clone(),
        task_id: task_id.to_string(),
        stage: stage_name,
        kind: TaskLogKind::Agent,
        status: TaskLogStatus::Pass,
        completed_at: Some(Utc::now()),
        duration_ms: None,
        exit_code: None,
        skip_reason: None,
    });

    update_list_to_in_progress(&mut flow, list_name, &loaded);
    flow.updated_at = Utc::now();
    fs::save_flow(&flow_dir, &flow)?;

    let output = TasksConfirmOutput {
        message: format!(
            "Marked {}/{} as done.",
            list_name.magenta(),
            task_id.cyan()
        ),
    };
    print_output(&output);
    Ok(())
}

/// `awf flow tasks skip <list> <task-id> --reason=<reason> [--flow=<path>]`
pub fn tasks_skip(
    flow_flag: Option<&str>,
    list_name: &str,
    task_id: &str,
    reason: &str,
) -> color_eyre::Result<()> {
    let (mut flow, workflow, flow_dir, _entries, _config) =
        load_context(flow_flag)?;

    let stage = require_task_runner_stage(&workflow, &flow, &flow_dir);
    let StageKind::TaskRunner { task_lists, .. } = &stage.kind else {
        unreachable!()
    };
    let stage_name = stage.name.clone();

    let loaded = load_all_task_lists(flow_dir.as_path(), task_lists);
    let (_, tl) =
        find_list_by_name_or_exit(list_name, task_lists, &loaded, &flow_dir);

    // Validate task exists.
    if !tl.tasks.iter().any(|t| t.id == task_id) {
        eprint_error(format_args!(
            "task {task_id:?} not found in list {list_name:?}",
        ));
        std::process::exit(1);
    }

    flow.task_log.push(TaskLogEntry {
        list: tl.name.clone(),
        task_id: task_id.to_string(),
        stage: stage_name,
        kind: TaskLogKind::Agent,
        status: TaskLogStatus::Skipped,
        completed_at: Some(Utc::now()),
        duration_ms: None,
        exit_code: None,
        skip_reason: Some(reason.to_string()),
    });

    update_list_to_in_progress(&mut flow, list_name, &loaded);
    flow.updated_at = Utc::now();
    fs::save_flow(&flow_dir, &flow)?;

    let output = TasksConfirmOutput {
        message: format!(
            "Skipped {}/{}: {}",
            list_name.magenta(),
            task_id.cyan(),
            reason
        ),
    };
    print_output(&output);
    Ok(())
}

/// `awf flow tasks advance <list> [--flow=<path>]`
pub fn tasks_advance(
    flow_flag: Option<&str>,
    list_name: &str,
) -> color_eyre::Result<()> {
    let (mut flow, workflow, flow_dir, _entries, _config) =
        load_context(flow_flag)?;

    let stage = require_task_runner_stage(&workflow, &flow, &flow_dir);
    let StageKind::TaskRunner { task_lists, .. } = &stage.kind else {
        unreachable!()
    };

    let loaded = load_all_task_lists(flow_dir.as_path(), task_lists);
    let (path, tl) =
        find_list_by_name_or_exit(list_name, task_lists, &loaded, &flow_dir);

    // Gate: all tasks must be done.
    if !task_runner::all_tasks_done(tl, &flow.current_stage, &flow) {
        eprint_error(format_args!(
            "list {list_name:?} still has incomplete tasks. Complete all tasks before advancing the list.",
        ));
        std::process::exit(1);
    }

    // Mark list as complete and unblock dependents.
    flow.task_runner_state.insert(path.to_string(), TaskListStatus::Complete);
    task_runner::recompute_blocked(&mut flow, &loaded);
    flow.updated_at = Utc::now();
    fs::save_flow(&flow_dir, &flow)?;

    let all_done = task_runner::all_lists_complete(&flow);
    let flow_display = flow_dir.display();
    let message = if all_done {
        format!(
            "List {list_name:?} marked complete. All lists done — run `awf flow next-stage --flow={flow_display}` to advance.",
        )
    } else {
        format!("List {list_name:?} marked complete.")
    };
    let output = TasksConfirmOutput { message };
    print_output(&output);
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

type Context = (
    crate::model::flow::FlowState,
    crate::model::workflow::Workflow,
    std::path::PathBuf,
    Vec<crate::config::WorkflowSearchEntry>,
    crate::config::Config,
);

fn load_context(flow_flag: Option<&str>) -> color_eyre::Result<Context> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = super::resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = super::load_flow_or_exit(&flow_dir)?;
    let workflow = crate::cli::load_workflow_or_exit(&entries, &flow.workflow)?;
    crate::cli::validate_workflow_or_exit(&workflow, &entries, &config);

    Ok((flow, workflow, flow_dir, entries, config))
}

/// Verifies the current stage is a task-runner stage, exiting with an error
/// if not.
fn require_task_runner_stage<'a>(
    workflow: &'a crate::model::workflow::Workflow,
    flow: &crate::model::flow::FlowState,
    flow_dir: &std::path::Path,
) -> &'a crate::model::workflow::Stage {
    let stage = super::current_stage_or_exit(workflow, flow);
    if !matches!(stage.kind, StageKind::TaskRunner { .. }) {
        eprint_error(format_args!(
            "stage {:?} is not a task-runner stage. \
             The `awf flow tasks` commands only apply to task-runner stages.",
            stage.name
        ));
        let _ = flow_dir;
        std::process::exit(1);
    }
    stage
}

/// Loads all task list files for the given paths. Missing or malformed files
/// are reported as warnings and skipped.
fn load_all_task_lists(
    flow_dir: &std::path::Path,
    task_list_paths: &[String],
) -> HashMap<String, TaskList> {
    let mut map = HashMap::new();
    for path in task_list_paths {
        match fs::load_task_list(flow_dir, path) {
            Ok(tl) => {
                map.insert(path.clone(), tl);
            }
            Err(Error::TaskListNotFound { path: p }) => {
                eprint_warning(format_args!(
                    "task list file not found: {}",
                    p.display()
                ));
            }
            Err(Error::TaskListParseFailed { path: p, source: e }) => {
                eprint_warning(format_args!(
                    "failed to parse task list {}: {e}",
                    p.display()
                ));
            }
            Err(e) => {
                eprint_error(format_args!("unexpected I/O error: {e}"));
                std::process::exit(1);
            }
        }
    }
    map
}

/// Finds a task list by its `name` field (not path), returning the matching
/// path and `TaskList`. Exits if no match is found.
fn find_list_by_name_or_exit<'a>(
    list_name: &str,
    task_list_paths: &[String],
    loaded: &'a HashMap<String, TaskList>,
    flow_dir: &std::path::Path,
) -> (&'a str, &'a TaskList) {
    for path in task_list_paths {
        if let Some(tl) = loaded.get(path)
            && tl.name == list_name
        {
            // Return the key from the map rather than the arg to avoid
            // lifetime issues with the temporary `path` variable.
            let (key, val) = loaded.get_key_value(path).unwrap();
            return (key.as_str(), val);
        }
    }
    eprint_error(format_args!(
        "task list {:?} not found in task-runner stage. \
         Run `awf flow tasks status --flow={}` to see available lists.",
        list_name,
        flow_dir.display()
    ));
    std::process::exit(1);
}

/// Builds the per-task display list for a task list.
fn build_task_displays(
    tl: &TaskList,
    stage_name: &str,
    flow: &crate::model::flow::FlowState,
) -> Vec<TaskDisplay> {
    let mut found_next = false;
    tl.tasks
        .iter()
        .map(|task| {
            let status = match task_runner::task_last_status(
                &tl.name, &task.id, stage_name, flow,
            ) {
                Some(TaskLogStatus::Pass) => TaskDisplayStatus::Done,
                Some(TaskLogStatus::Fail) => TaskDisplayStatus::Failed,
                Some(TaskLogStatus::Skipped) => TaskDisplayStatus::Skipped,
                None => {
                    if found_next {
                        TaskDisplayStatus::Pending
                    } else {
                        found_next = true;
                        TaskDisplayStatus::Next
                    }
                }
            };
            TaskDisplay {
                task_id: task.id.clone(),
                description: task.description.clone(),
                kind: task_kind_str(&task.kind),
                status,
            }
        })
        .collect()
}

const fn task_kind_str(kind: &TaskKind) -> &'static str {
    match kind {
        TaskKind::Agent => "agent",
        TaskKind::Command { .. } => "command",
    }
}

/// Transitions a list from Pending → `InProgress` using the path key in
/// `task_runner_state`.
fn update_list_to_in_progress(
    flow: &mut crate::model::flow::FlowState,
    list_name: &str,
    loaded: &HashMap<String, TaskList>,
) {
    let Some((path, _)) = loaded.iter().find(|(_, tl)| tl.name == list_name)
    else {
        return;
    };
    let status = flow
        .task_runner_state
        .entry(path.clone())
        .or_insert(TaskListStatus::Pending);
    if *status == TaskListStatus::Pending {
        *status = TaskListStatus::InProgress;
    }
}
