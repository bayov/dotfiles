`awf` — Design Document
=======================

Overview
--------

`awf` is a local-first CLI for defining and running reusable agent workflows.
Workflows are pure TOML; stage guidance lives in separate SKILL.md files.
Greenfield Rust project in `./awf`.

---

Architecture
------------

Three layers, each with a clear responsibility boundary:

```
CLI layer       — Clap subcommands, output formatting
Engine layer    — Pure logic: gate checks, next actions, shell execution
Store layer     — File I/O: read/write workflows, flows, artifacts
Model layer     — Types: Workflow, Stage, FlowState, CommandLogEntry
```

### Layer contracts

- **Model** has no I/O — plain Rust types, TOML (de)serialization only.
- **Engine** takes model types as input; returns results. No filesystem access.
- **Store** handles all filesystem concerns. Resolves `.awf/` root and
  `state_dir`.
- **CLI** wires everything together; owns output formatting.

---

Workflow Definition
-------------------

Stored at `.awf/workflows/<name>.toml`. Committed to the repo.

```toml
name = "feature"
version = 1
description = "Ship a new feature from design through merge."

[commands]
format = "cargo fmt --check"
lint = "cargo clippy -- -D warnings"
test = "cargo test"
build = "cargo build --release"

[[stages]]
name = "design"
description = "Produce a design doc."
produces = ["design.md"]
steps = [
    { name = "write-design", type = "agent", skill = "design.md" },
]

[[stages]]
name = "implement"
description = "Write the code."
requires = ["design.md"]
steps = [
    { name = "write-code", type = "agent", skill = "implement.md" },
    { name = "format", type = "command", command = "format" },
    { name = "lint", type = "command", command = "lint" },
    { name = "test", type = "command", command = "test" },
]

[[stages]]
name = "review"
description = "Code review and integration."
repeatable = true
produces = ["changelog-entry.md"]
steps = [
    { name = "pre-review-test", type = "command", command = "test" },
    { name = "do-review", type = "agent", skill = "review.md" },
    { name = "post-review-test", type = "command", command = "test" },
]

[[stages]]
name = "done"
description = "Complete."
```

### Stage fields

| Field         | Type       | Default | Description                            |
| ------------- | ---------- | ------- | -------------------------------------- |
| `name`        | string     | —       | Unique within workflow                 |
| `description` | string     | `""`    | Human-readable label                   |
| `produces`    | `[string]` | `[]`    | Artifacts this stage must produce      |
| `requires`    | `[string]` | `[]`    | Artifacts that must exist before entry |
| `repeatable`  | bool       | `false` | Show in `next` even after completion   |
| `steps`       | `[Step]`   | `[]`    | Ordered steps to complete this stage   |

### Step fields

| Field          | Type   | Default | Description                                                                |
| -------------- | ------ | ------- | -------------------------------------------------------------------------- |
| `name`         | string | —       | Unique within stage                                                        |
| `type`         | string | —       | `"command"` or `"agent"`                                                   |
| `command`      | string | —       | Key in `[commands]` table _(command steps)_                                |
| `run`          | string | —       | Inline shell string _(command steps; mutually exclusive with `command`)_   |
| `skill`        | string | —       | Path to SKILL.md relative to `.awf/workflows/<workflow>/` _(agent steps)_  |
| `instructions` | string | —       | Inline agent instructions _(agent steps; mutually exclusive with `skill`)_ |

Command steps complete automatically on execution (pass/fail by exit code).
Agent steps are marked complete explicitly via `awf flow step done <step>`.

### Workflow name rules

- Pattern: `[a-zA-Z0-9_/\-]+`
- Slashes allow subdirectory organization (e.g. `infra/deploy`)
- Must not already exist as a workflow file

---

Flow Instance State
-------------------

Lives at `.awf/flows/active/<flow_id>/state.toml`.

```toml
id = "add-oauth"
workflow = "feature"
workflow_version = 1
current_stage = "implement"
created_at = "2026-04-04T10:00:00Z"
updated_at = "2026-04-04T14:32:00Z"
archived = false

[[stage_history]]
stage = "design"
entered_at = "2026-04-04T10:00:00Z"
exited_at = "2026-04-04T12:00:00Z"

[[step_log]]
step = "write-code"
stage = "implement"
type = "agent"
status = "pass"
completed_at = "2026-04-04T13:00:00Z"

[[step_log]]
step = "format"
stage = "implement"
type = "command"
status = "pass"
exit_code = 0
started_at = "2026-04-04T14:30:00Z"
duration_ms = 1200
```

Artifacts live at `.awf/flows/active/<flow_id>/artifacts/`.

### Flow ID rules

- Pattern: `[a-zA-Z0-9_/\-]+`
- Name used as-is; never auto-generated
- Blocked if an active or archived flow already uses the name

---

State Machine Rules
-------------------

### Can enter a stage?

All must hold:

1. All `requires` artifacts exist in the flow's artifact dir
2. If stage already appears in `stage_history` and `repeatable = false`: CLI
   warns, prints existing `produces` artifact contents, and requires explicit
   confirmation — never hard-blocks

### Can exit a stage?

All must hold:

1. Every step has a `pass` entry in `step_log` during this stage visit
2. Every `produces` artifact exists in the flow's artifact dir

### `awf flow next` priority

1. Failing command step → instruct to rerun it
2. Incomplete agent step → print skill/instructions for the agent
3. Missing `produces` artifact → instruct to create it
4. All steps pass and artifacts present → suggest `awf flow advance`
5. On last stage → `done`

### `--force`

Bypasses all gate checks. Logged as a forced transition.

---

Directory Structure
-------------------

### Project-local (default)

```
project-root/
  .awf/
    workflows/
      feature.toml
      feature/           # Stage skill files (committed)
        design.md
        implement.md
        review.md
    flows/
      active/
        add-oauth/
          state.toml
          artifacts/
            design.md
      archived/
        add-oauth/
          state.toml
          artifacts/
```

### External state (configurable)

```
~/.awf/projects/<project_name>/
  flows/
    active/
    archived/
```

Configure via `.awf/config.toml` or `AWF_STATE_DIR` env var:

```toml
state_dir = "~/.awf/projects/my-project"
```

---

CLI Surface
-----------

All commands support `--output-format {human|json|toml|xml}` (default:
`human`).\
`<flow-id>` is optional when exactly one active flow exists.

| Command                                     | Purpose                                |
| ------------------------------------------- | -------------------------------------- |
| `awf workflow new <name>`                   | Create workflow template               |
| `awf workflow validate <name>`              | Validate workflow definition           |
| `awf workflow list`                         | List available workflows               |
| `awf flow new <name> --workflow <workflow>` | Create flow instance                   |
| `awf flow list [--all]`                     | List flows (`--all` includes archived) |
| `awf flow status [<id>]`                    | Show current state                     |
| `awf flow next [<id>]`                      | Show next required action              |
| `awf flow run <step> [<id>]`                | Execute a command step                 |
| `awf flow step done <step> [<id>]`          | Mark an agent step complete            |
| `awf flow advance [<id>] [--force]`         | Move to next stage if all steps pass   |
| `awf flow history [<id>]`                   | Show transition/command log            |
| `awf flow archive [<id>]`                   | Archive a completed flow               |

---

Rust Project Structure
----------------------

```
awf/
  Cargo.toml
  src/
    main.rs
    cli/
      mod.rs
      init.rs, new.rs, status.rs, next.rs, run.rs,
      step.rs, advance.rs, history.rs, archive.rs, list.rs,
      validate.rs, output.rs
    engine/
      mod.rs
      state.rs      # FlowState transitions, gate checks
      actions.rs    # next_actions(), can_advance(), artifact checks
      command.rs    # Shell execution, timing, exit code
    model/
      mod.rs
      workflow.rs   # Workflow, Stage, Step types + TOML parsing
      flow.rs       # FlowState, StepLogEntry, StageHistoryEntry
      id.rs         # ID validation
    store/
      mod.rs
      fs.rs         # Read/write workflows, flows, archive ops
      resolve.rs    # Find .awf/ dir, resolve state_dir, implicit flow-id
```

---

Implementation Order
--------------------

1. **Model** — TOML types, parsing, ID validation
2. **Store** — `.awf/` discovery, state_dir resolution, TOML read/write,
   artifact dirs
3. **Engine** — `next_actions()`, `can_advance()`, shell execution
4. **CLI** — Clap subcommands in order: `workflow new` → `validate` → `list` →
   `flow new` → `list` → `status` → `next` → `run` → `advance` → `history` →
   `archive`
5. **Tests** — `assert_cmd` + `tempfile` integration tests

---

Deferred (not v1)
-----------------

- Parallel stage execution
- Conditional branches / `if` expressions
- `continue_on_error` on commands
- Workflow inheritance / composition
- Stdout/stderr capture in command log
- Stage rollback
- Plugin system
- Watch mode / daemon
- Workflow variables / templating
- TUI
- Remote sync
