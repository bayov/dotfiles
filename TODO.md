TODO
====

- handle home/.claude/statusline.py
- Remove unnecessary ruff lint rules

Awf
---

- Review functionality
- Implement `dynamic_task_lists_plan.md`
- Define `research -> scope -> design -> decompose -> implement` workflow

Second Brain
------------

- Make sure we translate Hebrew to English for all incoming data to the second
  brain (whether from integrations or anywhere else).

Infeasible
----------

- Terminal support numpad keys (alt+numpad) to resize
  - Currently, this doesn't work because there's a problem with WezTerm
    establishing a better protocol with Fish shell that allows alt+numpad as
    separate keybinds to alt+arrows.
