TODO
====

- handle home/.claude/statusline.py
- Remove unnecessary ruff lint rules

Awf
---

- Better control for how to define output directory for a flow (state and
  artifacts).
- awf should be configurable using a file (.config/awf.toml) and env vars.
- --output-format should work for all commands.
- awf should support changing --output-format as config & env var.
- --output-format should print newline at the end
- workflow default directory should be .agents/workflows/
- instead of workflows/foo.toml and workflows/foo/agent_instructions.md, have
  workflows/foo/WORKFLOW.toml.
- `awf workflow validate` should validate all workflows.
- WORKFLOW.toml should support repo-wide commands (e.g. `format`, `test`)
  defined in .config/awf.toml.
- Implement `dynamic_task_lists_plan.md`
- Agent should have .awf-staging/ folder to write artifacts to before
  `awf artifact put` moves them into the flow directory.
- `awf flow next` -> `awf flow next-step`
- `awf flow advance` -> `awf flow next-stage`

Infeasible
----------

- Terminal support numpad keys (alt+numpad) to resize
  - Currently, this doesn't work because there's a problem with WezTerm
    establishing a better protocol with Fish shell that allows alt+numpad as
    separate keybinds to alt+arrows.
