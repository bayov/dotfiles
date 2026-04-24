//! Pure engine logic for task-runner stages.
//!
//! All functions are I/O-free. The CLI layer loads task lists from disk and
//! passes references here for decision-making.

use std::collections::HashMap;

use crate::model::flow::FlowState;
use crate::model::flow::TaskListStatus;
use crate::model::flow::TaskLogStatus;
use crate::model::task_list::Task;
use crate::model::task_list::TaskList;

// ---------------------------------------------------------------------------
// Initialisation
// ---------------------------------------------------------------------------

/// Initialises the `task_runner_state` map for a newly-entered task-runner
/// stage.
///
/// A list with an empty `depends_on` starts as [`TaskListStatus::Pending`];
/// one with dependencies starts as [`TaskListStatus::Blocked`].
///
/// `task_list_paths` are the artifact-relative paths declared in the stage
/// (e.g. `"tasks/foo.toml"`). `loaded` maps the same paths to their parsed
/// [`TaskList`] values.
pub fn init_task_runner_state(
    task_list_paths: &[String],
    loaded: &HashMap<String, TaskList>,
) -> HashMap<String, TaskListStatus> {
    task_list_paths
        .iter()
        .map(|path| {
            let status =
                if loaded.get(path).map_or(true, |tl| tl.depends_on.is_empty())
                {
                    TaskListStatus::Pending
                } else {
                    TaskListStatus::Blocked
                };
            (path.clone(), status)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Task-level queries
// ---------------------------------------------------------------------------

/// Returns the last log status for a task, or `None` if no entry exists.
pub fn task_last_status(
    list_name: &str,
    task_id: &str,
    stage: &str,
    flow: &FlowState,
) -> Option<TaskLogStatus> {
    flow.task_log
        .iter()
        .rfind(|e| {
            e.list == list_name && e.task_id == task_id && e.stage == stage
        })
        .map(|e| e.status)
}

/// Returns `true` if the task has a [`TaskLogStatus::Pass`] or
/// [`TaskLogStatus::Skipped`] entry.
pub fn task_is_done(
    list_name: &str,
    task_id: &str,
    stage: &str,
    flow: &FlowState,
) -> bool {
    matches!(
        task_last_status(list_name, task_id, stage, flow),
        Some(TaskLogStatus::Pass | TaskLogStatus::Skipped)
    )
}

/// Returns `true` if the task's last log entry has status
/// [`TaskLogStatus::Fail`].
pub fn task_is_failed(
    list_name: &str,
    task_id: &str,
    stage: &str,
    flow: &FlowState,
) -> bool {
    task_last_status(list_name, task_id, stage, flow)
        == Some(TaskLogStatus::Fail)
}

/// Returns `true` if every task in `list` has a pass or skipped entry.
pub fn all_tasks_done(list: &TaskList, stage: &str, flow: &FlowState) -> bool {
    list.tasks.iter().all(|t| task_is_done(&list.name, &t.id, stage, flow))
}

/// Returns the next undone task in `list`, in declaration order.
///
/// Stops and returns `None` if the immediately preceding task has a
/// [`TaskLogStatus::Fail`] entry — the failing task must be re-run before
/// continuing.
pub fn next_task_in_list<'a>(
    list: &'a TaskList,
    stage: &str,
    flow: &FlowState,
) -> Option<&'a Task> {
    let mut prev_failed = false;
    for task in &list.tasks {
        if task_is_done(&list.name, &task.id, stage, flow) {
            prev_failed = false;
            continue;
        }
        if prev_failed {
            // Blocked by a failing predecessor — surface nothing.
            return None;
        }
        return Some(task);
    }
    None
}

/// Returns `true` if any undone task is immediately blocked by a failed
/// predecessor. Used to display blocking state in `tasks status`.
pub fn list_has_failed_task(
    list: &TaskList,
    stage: &str,
    flow: &FlowState,
) -> bool {
    let mut seen_fail = false;
    for task in &list.tasks {
        if task_is_done(&list.name, &task.id, stage, flow) {
            seen_fail = false;
            continue;
        }
        if task_is_failed(&list.name, &task.id, stage, flow) {
            seen_fail = true;
        } else if seen_fail {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// List-level state updates
// ---------------------------------------------------------------------------

/// Transitions any [`TaskListStatus::Blocked`] list whose `depends_on` are all
/// [`TaskListStatus::Complete`] to [`TaskListStatus::Pending`].
///
/// Call after marking a list complete via `tasks advance`.
pub fn recompute_blocked(
    flow: &mut FlowState,
    all_lists: &HashMap<String, TaskList>,
) {
    let complete_names: std::collections::HashSet<&str> = flow
        .task_runner_state
        .iter()
        .filter(|(_, s)| **s == TaskListStatus::Complete)
        .filter_map(|(path, _)| all_lists.get(path).map(|tl| tl.name.as_str()))
        .collect();

    for (path, status) in flow.task_runner_state.iter_mut() {
        if *status != TaskListStatus::Blocked {
            continue;
        }
        let Some(tl) = all_lists.get(path) else {
            continue;
        };
        if tl.depends_on.iter().all(|dep| complete_names.contains(dep.as_str()))
        {
            *status = TaskListStatus::Pending;
        }
    }
}

/// Returns `true` if all task lists in `task_runner_state` are
/// [`TaskListStatus::Complete`].
pub fn all_lists_complete(flow: &FlowState) -> bool {
    !flow.task_runner_state.is_empty()
        && flow
            .task_runner_state
            .values()
            .all(|s| *s == TaskListStatus::Complete)
}
