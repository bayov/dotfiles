TODO
====

- handle home/.claude/statusline.py
- Remove unnecessary ruff lint rules

Awf
---

- Review functionality
- Improve human output format.
- Support custom flow dir templates (e.g. {project}/{YYYY-MM-DD}).
- Coverage for all output format as tests.
- Add README.md's
- Support out-of-workflow skills (e.g. in `.agents/skills`)
- Implement `dynamic_task_lists_plan.md`
- Define `research -> scope -> design -> decompose -> implement` workflow

Infeasible
----------

- Terminal support numpad keys (alt+numpad) to resize
  - Currently, this doesn't work because there's a problem with WezTerm
    establishing a better protocol with Fish shell that allows alt+numpad as
    separate keybinds to alt+arrows.
