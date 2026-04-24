awf Test Suite
==============

Tests live under `tests/cases/` as `.awf_test` files. The custom runner in
`tests/awf_test_runner.rs` parses and executes them using
[libtest-mimic](https://docs.rs/libtest-mimic), so they appear as individual
test cases in `cargo test` output.

```
tests/
  awf_test_runner.rs    # Runner and parser
  cases/
    workflow/           # awf workflow subcommand tests
    flow/               # awf flow subcommand tests
    config/             # Configuration loading and merging tests
    output_format/      # ANSI color/formatting tests (run with FORCE_COLOR=1)
```

Run all tests:

```sh
cargo test
```

Run a subset by name (the custom harness treats extra args as a name filter):

```sh
cargo test --test awf_test_runner -- flow/e2e_agent_session
cargo test --test awf_test_runner -- workflow/
```

---

`.awf_test` DSL
---------------

Each file is a sequence of **directive lines** and **content lines**.

```
# comment line (ignored)

$ <directive>           ← directive
| <content>             ← content attached to the preceding directive
|                       ← empty content line (blank line in the content)
```

Lines that are blank or start with `#` are ignored. Every other line must start
with `$` or `|`.

---

Directives
----------

### `write_file <path>`

Creates or overwrites a file. The content is the following `|`-prefixed lines.
Parent directories are created automatically.

```
$ write_file .agents/workflows/feature/WORKFLOW.toml
| name = "feature"
| version = 1
| ...
```

Paths are relative to the test's temporary project directory. Absolute paths and
`..` are not allowed.

### `delete <path>`

Deletes a file or directory. Directories are removed recursively.

```
$ delete flows/my-flow/artifacts/design.md
```

### `assert_exists <path>`

Fails the test if the path does not exist.

```
$ assert_exists .agents/workflows/feature/WORKFLOW.toml
```

### `assert_not_exists <path>`

Fails the test if the path exists.

```
$ assert_not_exists flows/my-flow/flow_state.toml
```

### `export KEY=value`

Sets an environment variable for all subsequent `awf` commands. Values may
contain built-in path variables (see below).

```
$ export AWF_ACTIVE_FLOW=$FLOWS_DIR/my-flow
```

### `unset KEY`

Removes an environment variable set by a previous `export`.

```
$ unset AWF_ACTIVE_FLOW
```

### `awf <args>`

Runs the `awf` binary with the given arguments. By default, the expected exit
code is 0 and both stdout and stderr must be empty. Override with the `assert_*`
directives below.

```
$ awf flow new my-flow --workflow=feature
```

### `assert_exit_code <code>`

Asserts the preceding `awf` command exited with the given code. Must appear
after an `awf` directive.

```
$ awf workflow validate missing-workflow
$ assert_exit_code 1
```

### `assert_stdout <mode>`

Asserts something about stdout of the preceding `awf` command. Requires a mode
(see [Assert Modes](#assert-modes)), optionally followed by content on `|`
lines.

```
$ awf workflow list
$ assert_stdout contains
| project:feature
```

### `assert_stderr <mode>`

Same as `assert_stdout` but for stderr.

```
$ awf flow next-stage --skip --flow=$FLOWS_DIR/my-flow
$ assert_stderr contains
| Skipped stage "design"
```

---

Assert Modes
------------

| Mode           | Behaviour                                                                                                   |
| -------------- | ----------------------------------------------------------------------------------------------------------- |
| `contains`     | Stdout/stderr must contain the given text. If both sides parse as JSON, uses partial JSON matching instead. |
| `not_contains` | Stdout/stderr must **not** contain the given text.                                                          |
| `equals`       | Full equality (after trimming). `<...>` placeholders match any substring within a single line.              |
| `empty`        | Output must be empty (no content lines needed).                                                             |
| `json`         | Output must be valid JSON and contain all keys/values in the expected JSON (partial match).                 |
| `toml`         | Output is parsed as TOML then compared with partial JSON matching.                                          |
| `xml`          | Output must be wrapped in `<data>…</data>`; inner JSON is compared with partial JSON matching.              |

### `equals` with `<...>` placeholders

`<...>` matches zero or more characters within a single line. Useful for paths
or timestamps that vary between runs.

```
$ awf flow status --flow=$FLOWS_DIR/my-flow
$ assert_stdout equals
|     Flow: <...>/my-flow
| Workflow: feature
|   Status: active (stage: research)
```

Lines without `<...>` must match exactly. The expected and actual outputs must
have the same number of lines.

### JSON partial matching

`assert_stdout json` checks that the actual JSON contains all the keys and
values in the expected JSON. Extra keys in the actual output are allowed. Array
lengths must match exactly.

Special wildcard strings:

- `"<any_string>"` — matches any JSON string
- `"<any_number>"` — matches any JSON number
- `"<any_array>"` — matches any JSON array

---

Built-in Path Variables
-----------------------

The test runner provides three path variables automatically. Use them in
directive arguments and content lines:

| Variable       | Expands to                                    |
| -------------- | --------------------------------------------- |
| `$PROJECT_DIR` | Temporary project root (contains `.agents/`)  |
| `$FLOWS_DIR`   | `$PROJECT_DIR/flows/`                         |
| `$USER_DIR`    | `$PROJECT_DIR/user-home/` (simulated `$HOME`) |

These are substituted in all directive arguments and `|` content before the
value is used.

---

Color / ANSI Tests
------------------

Tests in `tests/cases/output_format/` verify ANSI color codes. To enable color
output, set `FORCE_COLOR=1` in the test:

```
$ export FORCE_COLOR=1
$ awf flow status --flow=$FLOWS_DIR/my-flow
$ assert_stdout equals
| ...ANSI codes included...
```

When `FORCE_COLOR=1` is active, the runner performs a **two-phase check**:

1. Strip ANSI from both sides and assert content — gives a readable failure
   message when text is wrong.
2. Compare with ANSI codes intact — reports a styling mismatch with a hint to
   update the expected output.

---

Adding a Test Case
------------------

1. Create a `.awf_test` file under the appropriate `tests/cases/` subdirectory.
2. Start with a `# comment` describing the scenario.
3. Use `write_file` directives to set up the project state.
4. Issue `awf` commands and assert output.
5. Run `cargo test <your-test-name>` to verify.

Example skeleton:

```
# Test: workflow validate rejects duplicate stage names

$ write_file .agents/workflows/bad/WORKFLOW.toml
| name = "bad"
| version = 1
|
| [[stages]]
| name = "step"
| kind = "agent"
| args = { prompt = "Do something." }
|
| [[stages]]
| name = "step"
| kind = "agent"
| args = { prompt = "Duplicate." }

$ awf workflow validate bad
$ assert_exit_code 1
$ assert_stderr contains
| duplicate stage name
```
