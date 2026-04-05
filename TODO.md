TODO
====

- handle home/.claude/statusline.py
- Remove unnecessary ruff lint rules

Awf
---

- Support out-of-workflow skills (e.g. in `.agents/skills`)
- Support custom flow dir templates (e.g. {project}/{YYYY-MM-DD}).
- Improve human output format.
- Coverage for all output format as tests.
- Add README.md's
- Implement `dynamic_task_lists_plan.md`

Infeasible
----------

- Terminal support numpad keys (alt+numpad) to resize
  - Currently, this doesn't work because there's a problem with WezTerm
    establishing a better protocol with Fish shell that allows alt+numpad as
    separate keybinds to alt+arrows.
