awf — Agent Workflow CLI
========================

`awf` manages multi-stage workflows where each stage is either a shell command
or an AI agent invocation. You define a workflow once as a TOML file, then
create and drive **flow instances** through those stages, with `awf` tracking
progress, recording history, and guarding artifact gates.

---

Install
-------

```sh
cargo build --release       # binary at target/release/awf
cargo install --path .      # installs to ~/.cargo/bin/awf
```

---

Concepts
--------

### Workflow

A reusable template: a named, ordered list of **stages** stored in
`.agents/workflows/<name>/WORKFLOW.toml`. A workflow does not carry any runtime
state — it describes _what to do_, not _where things are_.

### Stage

One step in a workflow. Each stage is one of two kinds:

| Kind      | What awf does                                                              |
| --------- | -------------------------------------------------------------------------- |
| `agent`   | Prints instructions (inline text or a prompt file) for the agent to follow |
| `command` | Executes a shell command via `sh -c`, recording exit code and duration     |

A stage may declare `produces` — a list of artifact filenames that must exist
before the flow can advance past it.

### Flow

A **flow** is a directory on disk that holds one instance of a workflow. Its
state is persisted in `<flow_dir>/flow_state.toml`; artifacts land in
`<flow_dir>/artifacts/`. You can have many concurrent flows from the same
workflow.

### Artifacts

Files produced by a stage. Declared in `produces = ["file.md", ...]` on the
stage. `awf` will refuse to advance past a stage until all declared artifacts
exist at `<flow_dir>/artifacts/<name>`.

### Scopes

Workflows are discovered across multiple **scopes** (search directories):

| Scope     | Default location       | Override                           |
| --------- | ---------------------- | ---------------------------------- |
| `project` | `.agents/workflows/`   | not overridable                    |
| `user`    | `~/.agents/workflows/` | `[scopes.user] workflow_dir = …`   |
| custom    | (none by default)      | `[scopes.<name>] workflow_dir = …` |

Use `scope:name` syntax when a workflow name appears in multiple scopes:

```sh
awf flow new my-flow --workflow=project:feature
```

---

Quick Start
-----------

### 1. Create a workflow

```sh
awf workflow new feature
# → .agents/workflows/feature/WORKFLOW.toml
```

Edit `WORKFLOW.toml` to define your stages. Validate:

```sh
awf workflow validate feature
```

### 2. Create a flow

```sh
awf flow new flows/my-feature --workflow=feature
```

`awf` prints the current stage and its instructions.

### 3. Drive the flow

For **agent stages**: read the printed instructions, do the work, then advance:

```sh
awf flow next-stage --flow=flows/my-feature
```

For **command stages**: run the command and advance automatically:

```sh
awf flow run --flow=flows/my-feature
# awf executes the shell command; on success it records completion
awf flow next-stage --flow=flows/my-feature
```

Check progress at any time:

```sh
awf flow status --flow=flows/my-feature
```

### 4. Archive when done

```sh
awf flow archive --flow=flows/my-feature
```

Archived flows are excluded from `awf flow list` unless you pass `--all`.

---

Workflow File Format
--------------------

```toml
name = "feature"
version = 1
description = "Research, design, and implement a feature."

# default_flow_dir is used by `awf flow new <bare-name>` when you pass a
# name without a path prefix. Supports template variables (see below).
default_flow_dir = "{project_root}/flows"

# Named shell commands reusable across stages.
[commands]
test = "cargo test"
lint = "cargo clippy -- -D warnings"

# ── Stage 1: agent stage with inline instructions ─────────────────────────────
[[stages]]
name = "research"
description = "Research the problem space."
produces = ["research.md"] # artifact must exist before advancing
skippable = true # allows `awf flow next-stage --skip`
kind = "agent"
args = { prompt = "Research the problem. Write findings to research.md." }

# ── Stage 2: agent stage that reads a prompt file ─────────────────────────────
[[stages]]
name = "design"
description = "Write a technical design."
produces = ["design.md"]
kind = "agent"
args = { prompt_file = "design.md" }
# prompt_file is relative to the workflow directory, or absolute.

# ── Stage 3: command stage using a named command ──────────────────────────────
[[stages]]
name = "test"
description = "Run the test suite."
kind = "command"
args = { command = "test" } # resolves to [commands].test

# ── Stage 4: command stage with an inline shell string ────────────────────────
[[stages]]
name = "lint"
description = "Run the linter."
kind = "command"
args = { run = "cargo clippy -- -D warnings" }
```

### Stage fields

| Field         | Type                    | Required | Description                                  |
| ------------- | ----------------------- | -------- | -------------------------------------------- |
| `name`        | string                  | yes      | Unique within the workflow                   |
| `description` | string                  | no       | Shown in status output                       |
| `kind`        | `"agent"` / `"command"` | yes      | Stage type                                   |
| `args`        | table                   | yes      | Kind-specific (see below)                    |
| `produces`    | array of strings        | no       | Artifact filenames gating advancement        |
| `skippable`   | bool                    | no       | Enables `next-stage --skip` (default: false) |

### Agent `args`

Exactly one of:

| Key           | Description                                             |
| ------------- | ------------------------------------------------------- |
| `prompt`      | Inline instruction text shown to the agent              |
| `prompt_file` | Path to a `.md` file (relative to workflow dir, or abs) |

### Command `args`

Exactly one of:

| Key       | Description                                                       |
| --------- | ----------------------------------------------------------------- |
| `command` | Key into the `[commands]` table (workflow-level or project-level) |
| `run`     | Inline shell string executed via `sh -c`                          |

---

CLI Reference
-------------

### `awf workflow`

```
awf workflow new <name>
```

Creates `.agents/workflows/<name>/WORKFLOW.toml` from a stub template. `<name>`
must be alphanumeric with dashes, underscores, or slashes.

```
awf workflow validate [<name>]
```

Validates one workflow (or all, if `<name>` is omitted). Checks: stage name
uniqueness, mutual exclusivity of `command`/`run` and `prompt`/`prompt_file`,
existence of referenced prompt files.

```
awf workflow list
```

Lists all discovered workflows across all scopes. Output: `scope:name`.

---

### `awf flow`

All flow commands accept `--flow=<path>` to identify the target flow directory.
If omitted, `awf` falls back to the `AWF_ACTIVE_FLOW` env var, then to
`active_flow` in the project config. Commands exit with an error if no flow can
be resolved.

Path forms for `--flow`:

- Absolute: `/path/to/flow`
- Relative (current dir): `./my-flow`
- Tilde: `~/flows/my-flow`

```
awf flow new <flow> --workflow=<workflow>
```

Creates a new flow directory at `<flow>` and initialises `flow_state.toml`.
`<flow>` can also be a bare name (e.g. `my-flow`) if the workflow sets
`default_flow_dir`. Prints stage list and first-stage instructions.

```
awf flow list [--all]
```

Scans `flow_search_dirs` recursively for flows. Excludes archived flows unless
`--all` is passed.

```
awf flow status [--flow=<path>]
```

Shows current stage, all stages with completion status, artifact paths, and next
action.

```
awf flow run [--flow=<path>]
```

Executes the current **command** stage via `sh -c` with inherited stdio. Records
pass/fail and duration in history. Fails if the current stage is an agent stage.

```
awf flow next-stage [--flow=<path>] [--force=<reason>] [--skip]
```

Advances to the next stage.

- For **agent stages**: auto-records completion before advancing.
- For **command stages**: requires the stage to have already been run via
  `awf flow run`.
- `--force=<reason>`: bypasses the stage-completion gate but **not** the
  artifact gate. The reason string is stored in history.
- `--skip`: skips the stage entirely without recording artifacts. Only allowed
  when the stage has `skippable = true`.
- On the final stage: marks it complete and reports the workflow as done.

```
awf flow history [--flow=<path>]
```

Shows a timeline of all stage completions, failures, and skips with timestamps.
Force-advance reasons are included.

```
awf flow validate [--flow=<path>]
```

Checks artifact consistency: all `produces` from completed stages must exist,
and no unexpected artifacts should be present outside completed/current stage
declarations.

```
awf flow archive [--flow=<path>]
```

Sets `archived = true` in `flow_state.toml`. The flow directory stays in place.
Archived flows are hidden from `awf flow list`.

---

Configuration
-------------

Configuration is read from two TOML files and merged:

| Priority | Source                            | Notes                               |
| -------- | --------------------------------- | ----------------------------------- |
| lowest   | `~/.config/awf.toml`              | User config (XDG_CONFIG_HOME aware) |
|          | `<project_root>/.config/awf.toml` | Project config                      |
| highest  | `AWF_ACTIVE_FLOW` env var         | Overrides `active_flow`             |

`flow_search_dirs`: lists are **concatenated** (user + project).\
`scopes`: project entries override user entries with the same name.\
`commands`: project entries override user entries with the same alias.

### Example `.config/awf.toml`

```toml
project_name = "my-project"

# Directories scanned recursively for flows by `awf flow list`.
flow_search_dirs = ["~/projects/my-project/flows"]

# Set this to avoid passing --flow on every command.
active_flow = "~/projects/my-project/flows/current"

# Named shell commands available to all workflows in this project.
[commands]
test = "cargo test --all-features"
lint = "cargo clippy -- -D warnings"

# Override the user-scope workflow directory.
[scopes.user]
workflow_dir = "~/my-workflows"

# Define a custom workflow scope.
[scopes.team]
workflow_dir = "~/shared/team-workflows"
```

---

Template Variables
------------------

Template variables expand inside `default_flow_dir` in `WORKFLOW.toml` and
inside `prompt` / `prompt_file` fields.

| Syntax           | Expands to                                    |
| ---------------- | --------------------------------------------- |
| `{project_root}` | Absolute path to the project root             |
| `{project_name}` | Value of `project_name` in `.config/awf.toml` |
| `{time:FORMAT}`  | Current local time formatted by `FORMAT`      |
| `\{`, `\}`       | Literal `{` / `}` (no expansion)              |

### Time format tokens

`YYYY` → year (4-digit) · `MM` → month · `DD` → day · `HH` → hour (24 h) · `mm`
→ minute · `ss` → second

Example:

```toml
default_flow_dir = "{project_root}/flows/{time:YYYY-MM-DD}"
```

---

Project Root Detection
----------------------

`awf` walks up the directory tree from the current directory looking for the
first of these markers (in priority order):

1. `.config/awf.toml` — explicit awf project config
2. `.agents/` directory — awf convention
3. `.jj/` directory — Jujutsu VCS root
4. `.git/` directory — Git VCS root

The first ancestor directory containing any of these is the project root.
