AWF Change Proposal: Dynamic Task Lists
=======================================

Summary
-------

Extend AWF to support dynamically-generated, structured task lists as
first-class artifacts. A new `tasks/*.toml` artifact format replaces static
stage definitions for implementation-heavy workflows. A new
`kind =
"task-runner"` stage dispatches these lists, supports inter-list
dependency resolution, and optionally runs lists in parallel — all while keeping
agents anchored to the AWF feedback loop.

---

Motivation
----------

The current AWF model requires all stages to be statically defined in the
workflow TOML at authoring time. This breaks down for implementation stages
where the work is only known after upstream stages (research → plan → design →
task-breakdown) have run.

The naive fallback — one large agent stage that reads `tasks.md` and runs until
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
This uses the same artifact-gating logic already applied to `produces` on
stages.

---

New Stage Type: `task-runner`
-----------------------------

A new `kind = "task-runner"` stage replaces static per-stage definitions with a
`task_lists` declaration. The stage declares which task list artifacts it will
dispatch, and whether they may run in parallel.

```toml
[[stages]]
name = "implement"
kind = "task-runner"
args = { task_lists = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"], parallel = true }
```

| Field        | Type       | Default | Description                                                                                    |
| ------------ | ---------- | ------- | ---------------------------------------------------------------------------------------------- |
| `kind`       | string     | —       | `"task-runner"` (existing values: `"agent"`, `"command"`)                                      |
| `task_lists` | `[string]` | —       | Artifact paths to the task list TOML files (in `args`)                                         |
| `parallel`   | bool       | `false` | If `true`, lists whose `depends_on` are satisfied may be dispatched simultaneously (in `args`) |

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
kind = "agent"
args = { prompt_file = "research.md" }

[[stages]]
name = "plan"
produces = ["proposal.md"]
kind = "agent"
args = { prompt_file = "plan.md" }

[[stages]]
name = "design"
produces = ["design.md"]
kind = "agent"
args = { prompt_file = "design.md" }

[[stages]]
name = "task-breakdown"
produces = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"]
kind = "agent"
args = { prompt_file = "breakdown.md" }

[[stages]]
name = "implement"
kind = "task-runner"
args = { task_lists = ["tasks/foo.toml", "tasks/bar.toml", "tasks/baz.toml"], parallel = true }
```

---

New CLI Commands
----------------

In a `task-runner` stage, `awf flow next-stage` blocks advancement until all
task lists are `complete`. When tasks remain, it exits non-zero and prints the
current task status plus the specific `awf flow tasks ...` commands needed to
make progress — consistent with how `awf flow next-stage` reports blockers for
other stage kinds.

| Command                                                             | Purpose                                                                                                                                                          |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `awf flow tasks status --flow <path> [--task-list <name>]`          | Show task list status (`pending` / `in-progress` / `complete` / `blocked`), blocking indicators, and the next actionable task(s) — optionally scoped to one list |
| `awf flow tasks run <list> <task-id> --flow <path>`                 | Execute a command task and log the result                                                                                                                        |
| `awf flow tasks complete <list> <task-id> --flow <path>`            | Mark an agent task complete                                                                                                                                      |
| `awf flow tasks skip <list> <task-id> --reason "..." --flow <path>` | Skip a task with a reason; counts as `pass` for gate purposes                                                                                                    |

---

State Representation
--------------------

### `stage_history` entries

Task execution is appended to the existing `stage_history` array in the flow
state file (`.awf/flows/<flow-id>.toml`) using `<list>/<task-id>` as the `step`
field. The `stage` field holds the task-runner stage name as usual. A new
optional `step` field is added to `StageHistoryEntry`; existing entries without
it remain valid.

```toml
[[stage_history]]
stage = "implement"
step = "foo/1.3"
time = "2026-04-05T10:31:00Z"
status = "completed"
duration_ms = 320

[[stage_history]]
stage = "implement"
step = "bar/2.1"
time = "2026-04-05T10:45:00Z"
status = "completed"
```

### `task_runner_state` block

A new top-level block in the flow state file (`.awf/flows/<flow-id>.toml`)
tracks list-level completion for fast `depends_on` resolution:

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

The `task-runner` stage plugs into the existing `can_advance` rules. The
task-level gates mirror the stage-level gates already in place.

| Condition                           | `awf flow next-stage` / `awf flow tasks status` output             |
| ----------------------------------- | ------------------------------------------------------------------ |
| A list is `blocked` by `depends_on` | Shows list as blocked; names which dependencies are incomplete     |
| A command task fails                | Blocks; instructs `awf flow tasks run <list> <id>` to retry        |
| An agent task is pending            | Prints description; requests `awf flow tasks complete <list> <id>` |
| All lists are `complete`            | Suggests `awf flow next-stage` to exit the stage                   |

`--force` on `awf flow next-stage` bypasses list-level gates as it does today
for stage-level gates, and is logged as a forced transition.

---

Parallel Agent Loop Example
---------------------------

With `parallel = true`, two agents work simultaneously on `foo` and `bar`, then
either picks up `baz` once both are complete:

```bash
# Agent A — assigned to foo
awf flow tasks status --task-list foo    # → "foo/1.1: Scaffold auth module (agent)"
# ... does the work ...
awf flow tasks complete foo 1.1
awf flow tasks run  foo 1.3              # PASS: cargo fmt --check 320ms
awf flow tasks run  foo 1.4             # PASS: cargo test 4.1s  → foo → complete

# Agent B — working on bar simultaneously
awf flow tasks status --task-list bar   # → "bar/2.1: ..."
# ...
                                        # bar → complete (auto, last task done)

# baz unblocks automatically (depends_on: [foo, bar] both complete)
# Either agent picks it up
awf flow tasks status                   # → "baz/3.1: Wire foo and bar together (agent)"
awf flow tasks complete baz 3.1         # baz → complete (auto, last task done)

awf flow next-stage                     # all lists complete → exit implement
```

---

Impact on Existing Workflows
----------------------------

This change is fully additive. All existing workflow TOML files are unaffected:

- The existing stage kinds (`"agent"` and `"command"`) are unmodified
- No existing CLI commands are modified
- The `stage_history` format is extended — a new optional `step` field is added
  to entries; existing entries without `step` remain valid
- `task_runner_state` only appears in flow state files for flows that use a
  `task-runner` stage

---

Test Coverage
-------------

### Unit tests

New `.awf_test` cases under `tests/cases/flow/tasks/`:

- **`status_pending`** — task-runner stage just entered; `awf flow tasks status`
  shows all lists as `pending` with the first task of the first unblocked list
- **`status_blocked`** — a list with unsatisfied `depends_on`; status shows it
  as `blocked` and names the incomplete dependencies
- **`complete_agent_task`** — `awf flow tasks complete <list> <id>` records a
  `completed` entry in `stage_history` with the correct `step` field
- **`run_command_task_pass`** — `awf flow tasks run <list> <id>` executes the
  command, records `completed` + `duration_ms`
- **`run_command_task_fail`** — non-zero exit records `failed`; subsequent
  `status` shows the list as blocked on that task
- **`skip_task`** — `awf flow tasks skip <list> <id> --reason "..."` records
  `skipped`
- **`auto_complete_list`** — completing the last task in a list automatically
  sets `task_runner_state["tasks/foo.toml"] = "complete"` and unblocks any
  dependent lists
- **`next_stage_blocked`** — `awf flow next-stage` in a task-runner stage with
  incomplete tasks exits non-zero and prints actionable task status
- **`next_stage_advances`** — `awf flow next-stage` succeeds and advances the
  flow once all lists are `complete`

### End-to-end test update: `e2e_agent_session.awf_test`

The existing e2e test ends with `decompose` (agent, skippable) → `implement`
(agent). Update it so `decompose` produces task list artifacts and `implement`
is a `task-runner`.

**Workflow changes:**

```toml
[[stages]]
name = "decompose"
description = "Break work into tasks."
produces = ["tasks/auth.toml", "tasks/ui.toml"]
kind = "agent"
args = { prompt_file = "decompose.md" }
skippable = true

[[stages]]
name = "implement"
description = "Execute the task lists."
kind = "task-runner"
args = { task_lists = ["tasks/auth.toml", "tasks/ui.toml"] }
```

**New e2e steps after decompose completes:**

```
# Agent writes task list artifacts produced by decompose
write_file flows/add-feature/artifacts/tasks/auth.toml  → two agent tasks + one command task
write_file flows/add-feature/artifacts/tasks/ui.toml    → one agent task

awf flow next-stage   → enters implement (task-runner); shows task status

# Agent works through tasks
awf flow tasks status                    → shows auth/1.1 as next
awf flow tasks complete auth 1.1
awf flow tasks complete auth 1.2
awf flow tasks run auth 1.3              → PASS; auth → complete (auto)

awf flow tasks status                    → shows ui/2.1 as next
awf flow tasks complete ui 2.1           → ui → complete (auto)

awf flow next-stage                      → all lists complete; workflow done
```

The final `awf flow history` and `awf flow status` assertions must be updated to
reflect the new stage list and the task entries in `stage_history`.

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
