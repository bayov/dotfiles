TODO
====

- handle home/.claude/statusline.py
- Remove unnecessary ruff lint rules

Awf
---

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
- Support out-of-workflow skills (e.g. in `.agents/skills`)

Infeasible
----------

- Terminal support numpad keys (alt+numpad) to resize
  - Currently, this doesn't work because there's a problem with WezTerm
    establishing a better protocol with Fish shell that allows alt+numpad as
    separate keybinds to alt+arrows.
