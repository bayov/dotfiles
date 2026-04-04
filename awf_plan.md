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
# Artifacts this stage produces
produces = ["design.md"]

[[stages]]
name = "implement"
description = "Write the code."
# Requires design.md artifact to exist before stage can start
requires = ["design.md"]
after = ["format", "lint", "test"]

[[stages]]
name = "review"
description = "Code review and integration."
repeatable = true
before = ["test"]
after = ["test"]
produces = ["changelog-entry.md"]

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
  cannot start if its `requires` artifacts don't exist. Artifacts are stored in
  the flow's artifact directory.
- **`before` / `after`** — Named commands that gate entry/exit. References keys
  in `[commands]`. On failure, the CLI prints instructions to fix and rerun
  before proceeding — the stage does not advance.
- **Stage SKILL.md files** — Human/agent guidance lives in
  `.awf/workflows/feature/implement.md` (separate from the TOML definition). The
  engine doesn't parse these; agents read them directly.
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

[[command_log]]
command = "format"
stage = "implement"
status = "pass"
exit_code = 0
started_at = "2026-04-04T14:30:00Z"
duration_ms = 1200

[[command_log]]
command = "test"
stage = "implement"
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
      feature/              # Stage skill files (committed)
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
When only one active flow exists, `<flow-id>` is optional.

| Command                                     | Purpose                                              |
| ------------------------------------------- | ---------------------------------------------------- |
| `awf workflow new <name>`                   | Create workflow template                             |
| `awf workflow validate <name>`              | Validate workflow definition                         |
| `awf workflow list`                         | List available workflow definitions                  |
| `awf flow new <name> --workflow <workflow>` | Create flow instance; name used as-is as the flow ID |
| `awf flow list [--all]`                     | List flows (--all includes archived)                 |
| `awf flow status [<id>]`                    | Show current state                                   |
| `awf flow next [<id>]`                      | Show next required action                            |
| `awf flow run <command> [<id>]`             | Execute named command, log result                    |
| `awf flow advance [<id>] [--force]`         | Move to next stage if gates pass                     |
| `awf flow history [<id>]`                   | Show transition/command log                          |
| `awf flow archive [<id>]`                   | Archive completed flow                               |

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

# See current stage and gate status (implicit flow-id when only one exists)
awf flow status
awf flow status add-oauth

# Ask what to do next
awf flow next
# => Run command: test
# or: Produce artifact: design.md
# or: Run `awf flow advance` to move to implement

# Run a named command
awf flow run test
# => PASS  cargo test  (1243ms)
awf flow run test add-oauth

# Advance to next stage (runs before/after gates)
awf flow advance
# => ERROR: `lint` failed. Fix lint errors then run `awf flow run lint` before advancing.
awf flow advance --force   # bypass gates

# Show command and transition history
awf flow history
awf flow history add-oauth

# Archive a completed flow
awf flow archive
awf flow archive add-oauth

# Structured output for agent consumption
awf flow status --output-format json
awf flow next --output-format toml
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
   - All `before` pass (run automatically on advance)
   - If stage was already completed and `repeatable = false`: CLI warns, prints
     existing `produces` artifact contents, and requires explicit confirmation
     to proceed (never blocks — just informs)
4. **Can exit stage?** All must hold:
   - Every `after` has a `pass` in command_log _during this stage visit_
   - Every `produces` artifact exists in flow's artifact dir
5. **`awf next` priority:** failing `after` command → missing `produces`
   artifact → `advance` → `done`
6. **`--force`** bypasses gates (logged as forced)
7. **`before`/`after` failure** — CLI blocks advancement and prints
   agent-readable instructions: which command failed, what to fix, and which
   command to rerun.

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
      advance.rs, history.rs, archive.rs, list.rs,
      validate.rs, output.rs
    engine/
      mod.rs
      state.rs         # FlowState transitions, gate checks
      actions.rs       # next_actions(), can_advance(), artifact checks
      command.rs       # Shell execution, timing, exit code
    model/
      mod.rs
      workflow.rs      # Workflow/Stage types + TOML parsing
      flow.rs          # FlowState, CommandLogEntry, StageHistoryEntry
      id.rs            # Flow ID validation (alphanum, dashes, underscores, slashes)
    store/
      mod.rs
      fs.rs            # Read/write workflows, flows, archive ops
      resolve.rs       # Find .awf/ dir, resolve state_dir, implicit flow-id
```

**Dependencies:** `clap` (derive), `serde` + `serde_json`, `toml`, `chrono`,
`anyhow`, `rand`

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
