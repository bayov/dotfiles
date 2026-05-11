---
name: verify-plan
description: Verify a plan
argument-hint: <path-to-plan>
disable-model-invocation: true
model: opus
effort: medium
---

Verify that a whole plan landed: every PRD story advanced, every design
component touched, every glossary term shipped, every task complete. Doc-vs-doc
check; does **not** re-verify per-task code (trust `verify-task` for that).

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset).

> **Glossary root:** `$GLOSSARY_DIR` env var (defaults to the project root).

## Input

The plan slug is the first argument. Read everything under `$PLAN_DIR/{slug}/`:

- Whichever of `prd.md`, `design.md`, `plan.md` exist.
- Every file under `$PLAN_DIR/{slug}/tasks/`.
- `$GLOSSARY_DIR/GLOSSARY.md` and any relevant sub-glossaries.

## Checks

### Task completion

- Every file under `tasks/` has every acceptance criterion marked `- [x]`.
- Any task with unticked ACs → gap.

### Story coverage (when prd.md or plan.md exists)

- Every user story (`S1`, `S2`, ...) in the PRD/plan's user stories section is
  referenced by at least one task in its **References** section.
- A story not explicitly listed as out-of-scope in the PRD/plan and not
  referenced by any task → gap.

### Design component coverage (when design.md or plan.md exists)

- Every component the design marks as needing new behavior is touched by at
  least one task (referenced in some task's **References** or **Acceptance
  criteria**).
- A component with new behavior promised in the design but not in any task →
  gap.

### Glossary Changes

- Every entry in any **Glossary Changes** section of `prd.md` / `design.md` /
  `plan.md` is owned by exactly one task (referenced in that task's References
  or acceptance criteria).
- Each such entry has a corresponding definition present in
  `$GLOSSARY_DIR/GLOSSARY.md` (or sub-glossary).
- Entries with no owning task → gap (kick back to `decompose`).
- Entries with an owning task but no glossary definition → gap (kick back to
  `update-glossary`).
- Entries owned by more than one task → gap (overlap; kick back to `decompose`).

### Dependency graph

- Build the graph from each task's **Blocked by** section.
- No cycles. Cycle → gap.
- Every blocker referenced exists under `tasks/`. Dangling blocker → gap.
- Every blocker's ACs are `[x]` if its dependents' ACs are `[x]`. Dependent
  marked complete with an incomplete blocker → gap.

### Dangling references

- Every task's **References** section names docs/stories/components that
  actually exist in the plan's docs.
- Reference to a deleted story / component / decision → gap.

## Output

`PASS` or `FAIL`, same shape as `verify-task`.

### PASS

One line. Plan is fully landed.

### FAIL

```
Verdict: FAIL

Gaps:
- <category>: <description>. → <kick-back-skill>.
- ...
```

Kick-back mappings:

| Gap kind                                | Kick-back skill                       |
| --------------------------------------- | ------------------------------------- |
| Task with unticked ACs                  | `implement` (against that task)       |
| Story uncovered by any task             | `decompose` (add a slice)             |
| Story silently out-of-scope             | `prd-interview` (make it explicit)    |
| Design component uncovered              | `decompose` (or `design-interview`)   |
| Glossary entry with no owning task      | `decompose`                           |
| Glossary entry owned but undefined      | `update-glossary`                     |
| Glossary entry owned by >1 task         | `decompose` (resolve overlap)         |
| Dependency cycle or dangling blocker    | `decompose` (fix graph)               |
| Dependent `[x]` with incomplete blocker | Inconsistency; re-check task states   |
| Dangling reference in a task            | Edit the task in place or `decompose` |

## IMPORTANT: AVOID

- Do not re-run `verify-task`. This skill trusts ticked ACs at face value.
- Do not edit task files, the plan, or the glossary. Report only.
- Do not skip gaps because the plan is "mostly done." Coverage gaps left open
  become silent debt.
