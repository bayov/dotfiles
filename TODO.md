TODO
====

- handle home/.claude/statusline.py
- Remove unnecessary ruff lint rules

Awf
---

- Do tests use fake ram filesystem to simulate dir/file creation, etc?
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
