`awf` — Agent Workflow CLI
==========================

Context
-------

Build a local-first CLI tool for defining and running reusable agent workflows.
Workflows are pure TOML. Stage guidance lives in separate SKILL.md files.
Designed for both humans and LLM agents. Greenfield Rust project in `./awf`.

---

Architecture: Three Layers
--------------------------

1. **Workflow Definition** — TOML files defining stages, commands, artifacts,
   dependencies
2. **State Engine** — Pure logic: given workflow + flow state → valid actions,
   gate checks
3. **Command Execution** — Named shell commands, logged results, gate stage
   transitions

Workflow Format (Pure TOML)
---------------------------

`.awf/workflows/feature.toml`:

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
    { name = "write-design", type = "agent", prompt = "design.md" },
]

[[stages]]
name = "implement"
description = "Write the code."
requires = ["design.md"]
steps = [
    { name = "write-code", type = "agent", prompt = "implement.md" },
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
    { name = "do-review", type = "agent", prompt = "review.md" },
    { name = "post-review-test", type = "command", command = "test" },
]

[[stages]]
name = "done"
description = "Complete."
```

### Key design choices

- **Workflow name** — Must match `[a-zA-Z0-9_/\-]+` (alphanumeric, dashes,
  underscores, slashes). Slashes allow subdirectory organization. Must not
  already exist as a workflow file.
- **`repeatable`** — Default `false`. A non-repeatable stage that already
  appears in `stage_history` is excluded from `awf flow next` suggestions.
  However, any stage can always be explicitly re-entered: the CLI warns that the
  stage was already completed, prints existing `produces` artifact contents, and
  asks for confirmation before proceeding. Set `true` for stages like `review`
  that should appear as next steps even after completion.
- **Stage ordering** — Defined by TOML array order. Stages advance in sequence.
- **`produces` / `requires`** — Explicit artifact flow between stages. A stage
  cannot start if its `requires` artifacts don't exist. Agents register
  artifacts via `awf flow artifact put <name> <file>` — never by writing to the
  artifact directory directly.
- **`steps`** — Inline array of step objects on each stage. Each step is either
  `type = "command"` (shell command, pass/fail by exit code) or `type = "agent"`
  (human/LLM task). Command steps reference `[commands]` via `command = "key"`
  or define inline with `run = "..."`. Agent steps reference a SKILL.md via
  `prompt = "filename.md"` (any .md file, relative to
  `.awf/workflows/<workflow>/`) or inline with `instructions = "..."`. Order is
  explicit by position in the array.
- **Prompt files** — Any .md file (skill, checklist, instructions) can be
  referenced via the `prompt` field. The engine reads and displays the file
  content in `awf flow next`.
- **`continue_on_error`** (future) — Not in v1, but the schema can accommodate
  it.

Flow Instance State (TOML)
--------------------------

State lives at `.awf/flows/active/<flow_id>/state.toml`:

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

[[step_log]]
step = "test"
stage = "implement"
type = "command"
status = "fail"
exit_code = 1
started_at = "2026-04-04T14:31:00Z"
duration_ms = 8500
```

Flow artifacts live at `.awf/flows/active/<flow_id>/artifacts/`.

Directory Structure
-------------------

### Project-local (default)

```
project-root/
  .awf/
    workflows/              # Workflow definitions (committed)
      feature.toml
      feature/              # Stage prompt files (committed)
        design.md
        implement.md
        review.md
    flows/active/           # Active flow instances
      add-oauth/
        state.toml
        artifacts/
          design.md
    flows/archived/         # Archived flows
      add-oauth/
        state.toml
        artifacts/
```

### External storage (configurable)

```
~/.awf/
  projects/
    <project_name>/         # Derived from project dir name or config
      flows/active/
        add-oauth/
          state.toml
          artifacts/
      flows/archived/
```

Config via `.awf/config.toml` or env var `AWF_STATE_DIR`:

```toml
# .awf/config.toml
state_dir = "~/.awf/projects/my-project"
```

CLI Surface
-----------

All commands support `--output-format {human|json|toml|xml}` (default: `human`).

Flow-scoped commands require a flow ID, resolved in order:

1. `--flow <id>` flag
2. `AWF_ACTIVE_FLOW` env var
3. Error if neither is set

| Command                                             | Purpose                                              |
| --------------------------------------------------- | ---------------------------------------------------- |
| `awf workflow new <name>`                           | Create workflow template                             |
| `awf workflow validate <name>`                      | Validate workflow definition                         |
| `awf workflow list`                                 | List available workflow definitions                  |
| `awf flow new <name> --workflow <workflow>`         | Create flow instance; name used as-is as the flow ID |
| `awf flow list [--all]`                             | List flows (--all includes archived)                 |
| `awf flow status [--flow <id>]`                     | Show current state                                   |
| `awf flow next [--flow <id>]`                       | Show next required action                            |
| `awf flow run <step> [--flow <id>]`                 | Execute a command step, log result                   |
| `awf flow step done <step> [--flow <id>]`           | Mark an agent step complete                          |
| `awf flow artifact put <name> <file> [--flow <id>]` | Register a file as a named artifact                  |
| `awf flow advance [--flow <id>] [--force]`          | Move to next stage if all steps pass                 |
| `awf flow history [--flow <id>]`                    | Show transition/command log                          |
| `awf flow archive [--flow <id>]`                    | Archive completed flow                               |

### Example invocations

```
# Set up a new workflow template
awf workflow new feature

# Check a workflow definition is valid
awf workflow validate feature

# List available workflow definitions
awf workflow list

# Start a new flow instance (name used as-is as the flow ID)
awf flow new add-oauth --workflow feature
# => Created flow add-oauth

# If a flow already exists with that name:
# => ERROR: flow "add-oauth" already exists. Choose a different name or check `awf flow list`.

# List active flows
awf flow list
# => add-oauth  feature  design

# List all flows including archived
awf flow list --all

# See current stage and gate status
awf flow status --flow add-oauth
export AWF_ACTIVE_FLOW=add-oauth
awf flow status                    # uses AWF_ACTIVE_FLOW

# Ask what to do next
awf flow next --flow add-oauth

# Run a command step
awf flow run test --flow add-oauth
# => PASS  cargo test  (1243ms)

# Mark an agent step complete
awf flow step done write-code --flow add-oauth

# Advance to next stage
awf flow advance --flow add-oauth
# => ERROR: `lint` failed. Fix lint errors then run `awf flow run lint` before advancing.
awf flow advance --flow add-oauth --force   # bypass gates

# Show transition/command log
awf flow history --flow add-oauth

# Archive a completed flow
awf flow archive --flow add-oauth

# Structured output for agent consumption
awf flow status --flow add-oauth --output-format json
awf flow next --flow add-oauth --output-format toml
```

End-to-End Example
------------------

Full walkthrough using the `feature` workflow.

```
# Create the workflow definition
$ awf workflow new feature
Created .awf/workflows/feature.toml
Edit it to define your stages, then run `awf workflow validate feature`.

# Start a new flow
$ awf flow new add-oauth --workflow feature
Created flow add-oauth (workflow: feature, stage: design)

# Pin the active flow for this session
$ export AWF_ACTIVE_FLOW=add-oauth

# See where we are
$ awf flow status
Flow:    add-oauth
Stage:   design
Steps:   write-design [pending]

# Ask what to do next
$ awf flow next
[agent] write-design — see .awf/workflows/feature/design.md
Mark complete with: awf flow step done write-design

# Agent writes design.md then registers it as an artifact
$ awf flow artifact put design.md ./design.md
Artifact design.md registered.

$ awf flow step done write-design
Step write-design marked complete.

$ awf flow next
All steps complete. Run `awf flow advance` to move to implement.

$ awf flow advance
Entered stage: implement

# Agent reads implement.md and writes the code ...

$ awf flow step done write-code
Step write-code marked complete.

$ awf flow next
[command] format — next step to run: awf flow run format

$ awf flow run format
PASS  cargo fmt --check  (320ms)

$ awf flow run lint
FAIL  cargo clippy -- -D warnings  (1.2s)
  error[E0502]: cannot borrow ...

# Fix the lint error, then rerun

$ awf flow run lint
PASS  cargo clippy -- -D warnings  (1.1s)

$ awf flow run test
PASS  cargo test  (4.3s)

$ awf flow next
All steps complete. Run `awf flow advance` to move to review.

$ awf flow advance
Entered stage: review

$ awf flow run pre-review-test
PASS  cargo test  (4.3s)

$ awf flow step done do-review
Step do-review marked complete.

$ awf flow run post-review-test
PASS  cargo test  (4.3s)

$ awf flow next
All steps complete. Produce artifact: changelog-entry.md
Register it with: awf flow artifact put changelog-entry.md <file>

# Agent writes changelog-entry.md then registers it
$ awf flow artifact put changelog-entry.md ./changelog-entry.md
Artifact changelog-entry.md registered.

$ awf flow next
All steps complete. Run `awf flow advance` to move to done.

$ awf flow advance
Entered stage: done

$ awf flow archive
Archived flow add-oauth → .awf/flows/archived/add-oauth/
```

State Machine Rules
-------------------

1. **Flow name** — Used as-is as the flow ID. Must match `[a-zA-Z0-9_/\-]+`. If
   a flow already exists with that name (active or archived), the CLI blocks and
   instructs the agent to choose a different name or alerts the user that a
   similar flow exists.
2. **Stage ordering** — TOML array order defines sequence. No DAG logic.
3. **Can enter stage?** All must hold:
   - All `requires` artifacts exist in flow's artifact dir
   - If stage was already completed and `repeatable = false`: CLI warns, prints
     existing `produces` artifact contents, and requires explicit confirmation
     to proceed (never blocks — just informs)
4. **Can exit stage?** All must hold:
   - Every step has a `pass` in step_log _during this stage visit_
   - Every `produces` artifact exists in flow's artifact dir
5. **`awf next` priority:** failing command step → incomplete agent step →
   missing `produces` artifact → `advance` → `done`
6. **`--force`** bypasses gates (logged as forced)
7. **Step failure** — CLI blocks advancement and prints agent-readable
   instructions: which step failed, what to fix, and which step to rerun.

Rust Project Structure
----------------------

```
awf/
  Cargo.toml
  src/
    main.rs           # Clap CLI definition + dispatch
    cli/
      mod.rs
      init.rs, new.rs, status.rs, next.rs, run.rs,
      step.rs, artifact.rs, advance.rs, history.rs, archive.rs, list.rs,
      validate.rs, output.rs
    engine/
      mod.rs
      state.rs         # FlowState transitions, gate checks
      actions.rs       # next_actions(), can_advance(), artifact checks
      command.rs       # Shell execution, timing, exit code
    model/
      mod.rs
      workflow.rs      # Workflow, Stage, Step types + TOML parsing
      flow.rs          # FlowState, StepLogEntry, StageHistoryEntry
      id.rs            # Flow ID validation (alphanum, dashes, underscores, slashes)
    store/
      mod.rs
      fs.rs            # Read/write workflows, flows, archive ops
      resolve.rs       # Find .awf/ dir, resolve state_dir, resolve flow-id (--flow / AWF_ACTIVE_FLOW)
```

Implementation Order
--------------------

1. **Model layer** — Parse TOML workflow files. FlowState type. ID generation.
2. **Store layer** — Find `.awf/`, resolve state_dir (local vs external),
   read/write TOML state, manage artifact dirs.
3. **Engine layer** — Pure `next_actions()` and `can_advance()` logic. Shell
   command execution.
4. **CLI layer** — Clap subcommands: `workflow new` → `workflow validate` →
   `workflow list` → `flow new` → `flow list` → `flow status` → `flow next` →
   `flow run` → `flow advance` → `flow history` → `flow archive`.
5. **Integration tests** — `assert_cmd` + `tempfile`.

Deferred (not v1)
-----------------

- Parallel stage execution
- Conditional branches / `if` expressions on stages
- `continue_on_error` on commands
- Workflow inheritance / composition
- Stdout/stderr capture in command log
- Stage rollback
- Plugin system
- Watch mode / daemon
- Workflow variables / templating
- TUI
- Remote sync

Verification
------------

1. `cargo build` — compiles cleanly
2. `cargo test` — unit tests for TOML parsing, engine logic, ID generation
3. Manual walkthrough: `awf init` → `awf new` → `awf status` → `awf run` →
   `awf advance` → `awf archive`
4. `awf status --json` produces valid, stable JSON
5. `awf next` returns deterministic recommendations
6. Artifact gating: stage with `requires` fails to start if artifact missing
