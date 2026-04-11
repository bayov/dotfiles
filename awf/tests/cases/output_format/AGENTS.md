output_format tests
===================

Visual snapshot tests for `awf` command output.

`.ansi.awf_test` rules
----------------------

- `$ export FORCE_COLOR=1` must be set before commands whose output is asserted.
- Files contain **literal ESC bytes (0x1b)** in `assert_stdout equals` lines.
- Use `assert_stdout equals` with `<...>` placeholders for dynamic values.
- Use `assert_stdout contains` only for setup commands (not the focus of the
  test).
