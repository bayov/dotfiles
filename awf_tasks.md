`awf` Implementation Tasks
==========================

Conventions from `AGENTS.md`:

- **Tests**: unit tests go in `foo.tests.rs` (same dir, included via
  `#[cfg(test)] #[path = "foo.tests.rs"] mod tests;`); integration tests under
  `awf/tests/`
- **Error types**: single `Error` enum + `Result<T>` alias in `result.rs`;
  `#[non_exhaustive]`; no `unwrap()`/`expect()`
- **Docs**: doc-comment all non-trivial items; module-level `//!` docs where
  intent isn't obvious
- **Tracing**: `#[instrument]` on every significant function; structured fields,
  no dynamic strings in messages
- **Format**: run `bayov-format` before marking done
- **SLAP**: each function operates at one abstraction level; separate logic from
  I/O

---

Layer 1: Model
--------------

- [ ] **M1** Init Rust project — `cargo new --bin awf`; add deps: `clap`,
      `serde`, `toml`, `chrono`, `thiserror`, `tracing`; confirm `cargo build`
      clean
- [ ] **M2** `result.rs` — `#[non_exhaustive] enum Error` with `thiserror`;
      `pub type Result<T> = std::result::Result<T, Error>`; add variants as
      needed in later tasks
- [ ] **M3** `model/id.rs` — `validate_id(s: &str) -> Result<()>` enforcing
      `[a-zA-Z0-9_/\-]+`; doc-comment with format and error conditions; tests in
      `id.tests.rs`: valid/invalid inputs with concrete examples (`"add-oauth"`,
      `"infra/deploy"`, `""`, `"has space"`)
- [ ] **M4** `model/workflow.rs` — `Workflow`, `Stage` structs with serde;
      `Step` as an enum with
      `Command { command: Option<String>, run: Option<String> }` and
      `Agent { prompt: Option<String>, instructions: Option<String> }`; enforce
      mutual exclusivity in a `validate()` method; defaults: `repeatable=false`,
      `produces=[]`, `requires=[]`; tests in `workflow.tests.rs`: parse
      round-trip covering all fields and defaults
- [ ] **M5** `model/flow.rs` — `FlowState`, `StageHistoryEntry`, `StepLogEntry`
      structs with serde; `StepStatus` enum (`Pass`/`Fail`); tests in
      `flow.tests.rs`: TOML round-trip with both command and agent log entries

---

Layer 2: Store
--------------

- [ ] **S1** `store/resolve.rs` —
      `find_awf_root(start: &Path) -> Result<PathBuf>` walks up dir tree until
      `.awf/` found; doc-comment walking logic; tests in `resolve.tests.rs`:
      found case, not-found case (returns `Error::AwfRootNotFound`)
- [ ] **S2** `store/resolve.rs` —
      `resolve_state_dir(awf_root: &Path) -> Result<PathBuf>` — priority:
      `AWF_STATE_DIR` env var → `.awf/config.toml` `state_dir` → project-local
      `.awf`; expand `~`; doc each fallback; tests: one per resolution path
      using `tempfile`
- [ ] **S3** `store/resolve.rs` —
      `resolve_flow_id(flag: Option<&str>) -> Result<String>` — `--flow` flag →
      `AWF_ACTIVE_FLOW` env → `Error::FlowIdRequired`; tests: each path
- [ ] **S4** `store/fs.rs` —
      `load_workflow(awf_root: &Path, name: &str) -> Result<Workflow>` — reads
      `.awf/workflows/<name>.toml`; doc error on missing/malformed; test with
      fixture
- [ ] **S5** `store/fs.rs` —
      `list_workflows(awf_root: &Path) -> Result<Vec<String>>` — scans
      `.awf/workflows/*.toml`, strips `.toml` suffix; test: empty dir, multiple
      files
- [ ] **S6** `store/fs.rs` —
      `create_workflow_template(awf_root: &Path, name: &str) -> Result<()>` —
      validates name via `validate_id`; errors `Error::WorkflowAlreadyExists` if
      file exists; writes stub TOML with comment placeholders; test both error
      and success
- [ ] **S7** `store/fs.rs` —
      `load_flow(state_dir: &Path, flow_id: &str) -> Result<FlowState>` — reads
      `flows/active/<id>/state.toml`; test with fixture
- [ ] **S8** `store/fs.rs` —
      `save_flow(state_dir: &Path, state: &FlowState) -> Result<()>` —
      write-then-rename for atomicity; test round-trip
- [ ] **S9** `store/fs.rs` —
      `create_flow(state_dir: &Path, flow_id: &str, workflow: &str, first_stage: &str) -> Result<()>`
      — validates id; errors `Error::FlowAlreadyExists` if active or archived
      dir exists; creates dirs and initial `state.toml`; test duplicate
      (active), duplicate (archived), success
- [ ] **S10** `store/fs.rs` —
      `list_flows(state_dir: &Path, include_archived: bool) -> Result<Vec<FlowSummary>>`
      — `FlowSummary { id, workflow, current_stage, archived }`; test:
      active-only, with-archived
- [ ] **S11** `store/fs.rs` —
      `put_artifact(state_dir: &Path, flow_id: &str, name: &str, src: &Path) -> Result<()>`
      — copies `src` into `artifacts/<name>`; doc that name must be a filename,
      not a path; test
- [ ] **S12** `store/fs.rs` —
      `artifact_exists(state_dir: &Path, flow_id: &str, name: &str) -> bool`;
      test present/absent
- [ ] **S13** `store/fs.rs` —
      `read_artifact(state_dir: &Path, flow_id: &str, name: &str) -> Result<String>`;
      test
- [ ] **S14** `store/fs.rs` —
      `archive_flow(state_dir: &Path, flow_id: &str) -> Result<()>` — renames
      `flows/active/<id>` → `flows/archived/<id>`, sets `archived = true` and
      saves state; errors if not found; test
- [ ] **S15** `store/fs.rs` —
      `load_prompt_file(awf_root: &Path, workflow_name: &str, filename: &str) -> Result<String>`
      — reads `.awf/workflows/<workflow>/<filename>`; test

---

Layer 3: Engine
---------------

Engine functions take only model types — no filesystem access. All artifact
presence checks receive a pre-built `&dyn ArtifactStore` trait or a
`HashSet<String>` of existing artifact names.

- [ ] **E1** `engine/actions.rs` — `NextAction` enum: `RunStep(&Step)`,
      `CompleteAgentStep(&Step)`, `ProduceArtifact(String)`, `CanAdvance`,
      `Done`; doc each variant
- [ ] **E2** `engine/actions.rs` —
      `next_action(workflow: &Workflow, flow: &FlowState, artifacts: &HashSet<String>) -> NextAction`
      — pure fn; priority order per spec; `#[instrument]`; tests in
      `actions.tests.rs`: one test per `NextAction` variant using concrete
      fixture data (real stage/step names)
- [ ] **E3** `engine/actions.rs` —
      `can_advance(workflow: &Workflow, flow: &FlowState, artifacts: &HashSet<String>) -> Result<(), AdvanceBlocker>`
      — `AdvanceBlocker` enum: `StepFailed(String)`, `StepIncomplete(String)`,
      `MissingArtifact(String)`; tests: each blocker case and the success case
- [ ] **E4** `engine/state.rs` — `EnterResult` enum: `Ok`,
      `MissingRequires(Vec<String>)`,
      `AlreadyCompleted { produces_contents: Vec<(String, String)> }`;
      `can_enter_stage(workflow, flow, stage_name, artifacts) -> EnterResult`;
      tests: missing requires, already completed non-repeatable, repeatable,
      first visit
- [ ] **E5** `engine/state.rs` —
      `advance_stage(workflow: &Workflow, flow: FlowState) -> Result<FlowState>`
      — moves `current_stage` to next in array order; appends
      `StageHistoryEntry` with `exited_at` on current; errors
      `Error::AlreadyOnLastStage` if no next; `#[instrument]`; tests: normal
      advance, last stage error
- [ ] **E6** `engine/state.rs` —
      `record_step_pass(flow: FlowState, stage: &str, step: &Step, ...) -> FlowState`
      and `record_step_fail(...)` — pure builders appending to `step_log`; tests
      with concrete step names and timestamps
- [ ] **E7** `engine/command.rs` —
      `CommandResult { exit_code: i32, duration_ms: u64 }`;
      `run_command(shell_cmd: &str) -> Result<CommandResult>` — spawns via
      `sh -c`; doc that stderr/stdout are inherited (not captured) in v1;
      `#[instrument(fields(cmd = shell_cmd))]`; integration test (under
      `awf/tests/`) with a known-pass (`true`) and known-fail (`false`) command

---

Layer 4: CLI
------------

- [ ] **C1** `main.rs` + `cli/mod.rs` — Clap top-level with `workflow` and
      `flow` subcommand groups; global `--output-format <human|json|toml|xml>`
      (default `human`); compiles cleanly; module-level `//!` doc on
      `cli/mod.rs`
- [ ] **C2** `cli/output.rs` — `OutputFormat` enum;
      `fn render<T: Serialize + Display>(val: &T, fmt: OutputFormat) -> String`;
      tests in `output.tests.rs`: each format with a concrete struct
- [ ] **C3** `cli/workflow/new.rs` — `awf workflow new <name>` handler; calls
      `create_workflow_template`; prints path on success; renders
      `Error::WorkflowAlreadyExists` as user-friendly message
- [ ] **C4** `cli/workflow/validate.rs` — `awf workflow validate <name>` — loads
      workflow; checks every `command` step key exists in `[commands]`; checks
      every `prompt` file exists; prints `OK` or lists all errors
- [ ] **C5** `cli/workflow/list.rs` — `awf workflow list` — tabular output: name
      column
- [ ] **C6** `cli/flow/new.rs` — `awf flow new <name> --workflow <workflow>` —
      validates id, loads workflow, calls `create_flow`; prints
      `Created flow <id> (workflow: <w>, stage: <s>)`
- [ ] **C7** `cli/flow/list.rs` — `awf flow list [--all]` — tabular output: id,
      workflow, stage, (archived flag if `--all`)
- [ ] **C8** `cli/flow/status.rs` — `awf flow status` — prints flow id, current
      stage, each step with status (`pending`/`pass`/`fail`) for the current
      stage visit only
- [ ] **C9** `cli/flow/next.rs` — `awf flow next` — calls `next_action`; for
      agent steps loads and prints prompt file content with `load_prompt_file`;
      for command steps prints shell string; each case ends with the exact CLI
      command to run next
- [ ] **C10** `cli/flow/run.rs` — `awf flow run <step>` — resolves step by name
      in current stage; errors if not a command step; calls `run_command`; calls
      `record_step_pass/fail`; saves state; prints `PASS`/`FAIL` with duration
- [ ] **C11** `cli/flow/step.rs` — `awf flow step done <step>` — errors if step
      not found or not `agent` type; calls `record_step_pass`; saves state;
      prints confirmation
- [ ] **C12** `cli/flow/artifact.rs` — `awf flow artifact put <name> <file>` —
      calls `put_artifact`; prints `Artifact <name> registered.`
- [ ] **C13** `cli/flow/advance.rs` — `awf flow advance [--force]` — calls
      `can_advance` unless `--force` (logs forced); calls `advance_stage`; saves
      state; prints new stage; if on last stage prints done message
- [ ] **C14** `cli/flow/history.rs` — `awf flow history` — renders
      `stage_history` + `step_log` merged in chronological order
- [ ] **C15** `cli/flow/archive.rs` — `awf flow archive` — calls `archive_flow`;
      prints destination path

---

Layer 5: Integration Tests (`awf/tests/`)
-----------------------------------------

Use `assert_cmd` + `tempfile`. All tests use the `feature` workflow fixture with
real stage/step names. No system time — inject fixed timestamps or test state
directly.

- [ ] **T1** Fixture helper: `setup_feature_workflow(tmp: &TempDir)` — writes
      `.awf/workflows/feature.toml` + all prompt `.md` files; reusable across
      all tests
- [ ] **T2** `awf workflow new` creates file; second call errors with duplicate
      message
- [ ] **T3** `awf workflow validate` passes on valid; reports missing command
      key; reports missing prompt file
- [ ] **T4** `awf flow new` creates state; `awf flow status` shows `design`
      stage, `write-design [pending]`
- [ ] **T5** Full happy path through `design` stage: `artifact put` →
      `step done` → `advance` enters `implement`
- [ ] **T6** `awf flow run` PASS and FAIL cases; re-run after failure reflects
      new pass status
- [ ] **T7** `awf flow advance` blocked when a command step has `fail` status;
      unblocked after rerun
- [ ] **T8** `awf flow advance` blocked when `produces` artifact missing;
      unblocked after `artifact put`
- [ ] **T9** `requires` gate: `awf flow advance` into `implement` blocked when
      `design.md` artifact absent
- [ ] **T10** `--force` on `advance` bypasses gate and succeeds;
      `awf flow history` shows forced marker
- [ ] **T11** Duplicate flow name blocked when active flow exists; blocked when
      archived flow exists
- [ ] **T12** `awf flow next` output is deterministic at each named state
      (failing step, incomplete agent, missing artifact, can-advance)
- [ ] **T13** `awf flow status --output-format json` produces valid JSON; keys
      stable across runs
- [ ] **T14** `awf flow archive` moves dirs; subsequent `awf flow status` errors
      with not-found; `awf flow list --all` shows archived entry
