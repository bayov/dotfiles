AWF Change Proposal: Dynamic Task Lists
=======================================

Summary
-------

Extend AWF to support dynamically-generated, structured task lists as
first-class artifacts. A new `tasks/*.toml` artifact format replaces static
stage steps for implementation-heavy workflows. A new `type = "task-runner"`
stage dispatches these lists, supports inter-list dependency resolution, and
optionally runs lists in parallel — all while keeping agents anchored to the AWF
feedback loop.

---

Motivation
----------

The current AWF model requires all steps to be statically defined in the
workflow TOML at authoring time. This breaks down for implementation stages
where the work is only known after upstream stages (research → plan → design →
task-breakdown) have run.

The naive fallback — one large agent step that reads `tasks.md` and runs until
done — loses all AWF benefits: no gate checks, no per-task logging, no failure
recovery, no parallelism. An agent running free over a large task list is
unanchored and unobservable.

This proposal makes the task list itself a structured artifact that AWF can
parse, dispatch, gate, and log — without requiring the workflow author to know
the tasks at definition time.

---

New Artifact Type: `tasks/*.toml`
---------------------------------

Task lists are TOML files produced by an agent stage, stored under `tasks/` in
the flow's artifact directory. They have a defined schema that AWF understands
natively.

```toml
# tasks/foo.toml
name = "foo"
description = "Implement the authentication module"
depends_on = [] # names of other task lists that must complete first

[[tasks]]
id = "1.1"
description = "Scaffold the auth module"
type = "agent"

[[tasks]]
id = "1.2"
description = "Write unit tests"
type = "agent"

[[tasks]]
id = "1.3"
description = "Run formatter"
type = "command"
command = "format" # key in the workflow-level [commands] table

[[tasks]]
id = "1.4"
description = "Run tests"
type = "command"
command = "test"
```

### Schema Fields

**List-level:**

| Field         | Type       | Default | Description                                                                    |
| ------------- | ---------- | ------- | ------------------------------------------------------------------------------ |
| `name`        | string     | —       | Unique within the task-runner stage                                            |
| `description` | string     | —       | Human-readable label                                                           |
| `depends_on`  | `[string]` | `[]`    | Names of task lists that must be `complete` before this list can be dispatched |

**Task-level:**

| Field         | Type   | Default | Description                                             |
| ------------- | ------ | ------- | ------------------------------------------------------- |
| `id`          | string | —       | Unique within the list (e.g. `"1.1"`, `"2.3"`)          |
| `description` | string | —       | Human-readable task description                         |
| `type`        | string | —       | `"agent"` or `"command"`                                |
| `command`     | string | —       | Key in workflow `[commands]` table. Command tasks only. |

Tasks within a list are always executed **sequentially** in declaration order.
Parallelism lives between lists, not within them.

### `depends_on` Example

```toml
# tasks/baz.toml
name = "baz"
description = "Integration layer — requires foo and bar"
depends_on = ["foo", "bar"]

[[tasks]]
id = "3.1"
description = "Wire foo and bar modules together"
type = "agent"
```

AWF will not dispatch `baz` until both `foo` and `bar` are marked `complete`.
This uses the same artifact-gating logic already applied to `requires` on
stages.

---

New Stage Type: `task-runner`
-----------------------------

A new `type = "task-runner"` stage replaces the static `steps` array with a
`task_lists` declaration. The stage declares which task list artifacts it will
dispatch, and whether they may run in parallel.

```toml
[[stages]]
name = "implement"
type = "task-runner"
requires = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"]
task_lists = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"]
parallel = true # default: false
```

| Field        | Type       | Default    | Description                                                                        |
| ------------ | ---------- | ---------- | ---------------------------------------------------------------------------------- |
| `type`       | string     | `"linear"` | Set to `"task-runner"` to enable this mode                                         |
| `task_lists` | `[string]` | —          | Artifact paths to the task list TOML files                                         |
| `parallel`   | bool       | `false`    | If `true`, lists whose `depends_on` are satisfied may be dispatched simultaneously |

`parallel = false` dispatches lists one at a time in declaration order, still
respecting `depends_on`. `parallel = true` allows simultaneous agents to work on
any lists whose dependencies are already complete.

---

Full Workflow Example
---------------------

```toml
name = "research-to-implementation"
version = 1

[commands]
format = "cargo fmt --check"
test = "cargo test"

[[stages]]
name = "research"
produces = ["research.md"]
steps = [{ name = "do-research", type = "agent", prompt = "research.md" }]

[[stages]]
name = "plan"
requires = ["research.md"]
produces = ["proposal.md"]
steps = [{ name = "write-plan", type = "agent", prompt = "plan.md" }]

[[stages]]
name = "design"
requires = ["proposal.md"]
produces = ["design.md"]
steps = [{ name = "write-design", type = "agent", prompt = "design.md" }]

[[stages]]
name = "task-breakdown"
requires = ["design.md"]
produces = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"]
steps = [{ name = "write-tasks", type = "agent", prompt = "breakdown.md" }]

[[stages]]
name = "implement"
type = "task-runner"
requires = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"]
task_lists = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"]
parallel = true
```

---

New CLI Commands
----------------

`awf flow next` in a `task-runner` stage delegates automatically to
`awf flow tasks next`, so existing agent loops need no changes at the top level.

| Command                                                           | Purpose                                                                                                      |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `awf flow tasks status --flow <id>`                               | Show all task lists with status (`pending` / `in-progress` / `complete` / `blocked`) and blocking indicators |
| `awf flow tasks next --flow <id> [--list <name>]`                 | Show the next actionable task across all unblocked lists, or within a specific list                          |
| `awf flow tasks run <list> <task-id> --flow <id>`                 | Execute a command task and log the result                                                                    |
| `awf flow tasks done <list> <task-id> --flow <id>`                | Mark an agent task complete                                                                                  |
| `awf flow tasks skip <list> <task-id> --reason "..." --flow <id>` | Skip a task with a reason; counts as `pass` for gate purposes                                                |
| `awf flow tasks advance <list> --flow <id>`                       | Mark a task list as complete; unblocks any lists depending on it                                             |

---

State Representation
--------------------

### `steplog` entries

Task execution is appended to the existing `steplog` structure using
`<list>/<task-id>` as the step name. No new top-level state keys are required.

```toml
[[steplog]]
step = "foo/1.3"
stage = "implement"
type = "command"
status = "pass"
exit_code = 0
started_at = "2026-04-05T10:31:00Z"
duration_ms = 320

[[steplog]]
step = "bar/2.1"
stage = "implement"
type = "agent"
status = "pass"
completed_at = "2026-04-05T10:45:00Z"
```

### `task_runner_state` block

A new top-level block in `state.toml` tracks list-level completion for fast
`depends_on` resolution:

```toml
[task_runner_state]
"tasks/foo.toml" = "complete"
"tasks/bar.toml" = "in-progress"
"tasks/baz.toml" = "blocked" # waiting on foo + bar
```

Valid values: `"pending"` / `"in-progress"` / `"complete"` / `"blocked"`.

---

Gate and Advancement Logic
--------------------------

The `task-runner` stage plugs into the existing `can_exit_stage?` rules. The
task-level gates mirror the step-level gates already in place.

| Condition                           | `awf flow next` / `awf flow tasks next` output                 |
| ----------------------------------- | -------------------------------------------------------------- |
| A list is `blocked` by `depends_on` | Shows list as blocked; names which dependencies are incomplete |
| A command task fails                | Blocks; instructs `awf flow tasks run <list> <id>` to retry    |
| An agent task is pending            | Prints description; requests `awf flow tasks done <list> <id>` |
| All tasks in a list pass            | Suggests `awf flow tasks advance <list>`                       |
| All lists are `complete`            | Suggests `awf flow advance` to exit the stage                  |

`--force` on `awf flow advance` bypasses list-level gates as it does today for
step-level gates, and is logged as a forced transition.

---

Parallel Agent Loop Example
---------------------------

With `parallel = true`, two agents work simultaneously on `foo` and `bar`, then
either picks up `baz` once both are complete:

```bash
# Agent A — assigned to foo
awf flow tasks next --list foo    # → "foo/1.1: Scaffold auth module (agent)"
# ... does the work ...
awf flow tasks done foo 1.1
awf flow tasks run  foo 1.3       # PASS: cargo fmt --check 320ms
awf flow tasks run  foo 1.4       # PASS: cargo test 4.1s
awf flow tasks advance foo        # foo → complete

# Agent B — working on bar simultaneously
awf flow tasks next --list bar    # → "bar/2.1: ..."
# ...
awf flow tasks advance bar        # bar → complete

# baz unblocks automatically (depends_on: [foo, bar] both complete)
# Either agent picks it up
awf flow tasks next               # → "baz/3.1: Wire foo and bar together (agent)"
awf flow tasks done baz 3.1
awf flow tasks advance baz        # baz → complete

awf flow advance                  # all lists complete → exit implement
```

---

Impact on Existing Workflows
----------------------------

This change is fully additive. All existing workflow TOML files are unaffected:

- The default stage type remains `"linear"` (the current behaviour, implicit
  today, made explicit)
- No existing CLI commands are modified
- The `steplog` format is extended, not changed — existing entries remain valid
- `task_runner_state` only appears in state files for flows that use a
  `task-runner` stage

---

Deferred (Not v1)
-----------------

- **Intra-list parallelism** — tasks within a single `.toml` are always
  sequential for now
- **Dynamic list creation mid-flight** — an agent writing a new `tasks/qux.toml`
  not declared in the stage definition; requires re-validation logic and is a
  larger design question
- **Nested task runners** — a task that itself spawns a task list
- **`continue_on_error` for task lists** — consistent with the same deferral on
  stages
