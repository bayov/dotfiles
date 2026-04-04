TODO
====

- handle home/.claude/statusline.py
- Remove unnecessary ruff lint rules

Awf
---

- Better control for how to define output directory for a workflow (state and
  artifacts).

Infeasible
----------

- Terminal support numpad keys (alt+numpad) to resize
  - Currently, this doesn't work because there's a problem with WezTerm
    establishing a better protocol with Fish shell that allows alt+numpad as
    separate keybinds to alt+arrows.
