---
name: verify-task
description: Verify a single task's implementation
argument-hint: <path-to-task-file> [pr <number> | range <revset> | code diff]
disable-model-invocation: true
model: opus
effort: high
---

Verify that a single task's slice was implemented faithfully, with attention
independent from the model that wrote the code. Report-only; never edit code.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset).

> **Glossary root:** `$GLOSSARY_DIR` env var (defaults to the project root).

## Input

- **Task file** (required, first argument): the task to verify. Read it in full.
  Follow its **References** section and read every doc it cites (`prd.md` /
  `design.md` / `plan.md`). Read all `Blocked by` task files too — their state
  matters for cross-task verification.
- **Glossary**: read `$GLOSSARY_DIR/GLOSSARY.md` and any relevant
  sub-glossaries.
- **Diff range** — determine the diff to verify against:
  - If the argument contains `pr <number>`: fetch the PR's diff via
    `gh pr diff <number>` and use that.
  - If the argument contains `range <revset>`: use the given jj revset.
  - Otherwise (default): auto-detect. Identify the bookmark at or near `@` and
    use that bookmark's commits vs `trunk()`. If no bookmark exists on the stack
    yet, fall back to `trunk()..@`.

### Refuse to run when

- `@` has no description (the change is mid-edit; verification expects a
  discrete deliverable). Tell the user to describe `@` and re-invoke.
- The task file's `Blocked by` references a task whose ACs aren't all `[x]`.
  Surface the unfinished blocker; verification is premature.

## Checks

Run each check against the diff and the task file. For each gap, record what is
missing and which kick-back skill is the right next step.

### Acceptance criteria

- Every AC in the task file's **Acceptance criteria** section is `- [x]`.
- For each ticked AC, the diff contains code and at least one test that
  demonstrates the behavior. A ticked AC with no corresponding test → gap.
- A test that claims to cover an AC but exercises a trivial assertion (no real
  invariant) → gap.

### Scope

- Nothing in the diff lives outside the task's stated behavior or the task's
  **Out of scope** section unless it is owned by another task under the same
  plan (cross-check sibling task files). Out-of-scope creep → gap.

### Planning-artifact leakage

The shipped code must read as if planning artifacts never existed. Grep the diff
for:

- Path fragments: `plan/`, `tasks/`, `prd.md`, `design.md`, `plan.md`,
  `plan_interview`.
- The task slug, the task filename, or the plan slug in identifiers, file names,
  comments, docstrings, or commit messages.
- Verbatim AC text used as identifiers or comments.

Any match → gap.

### Glossary and docs

- Every term in the **Glossary Changes** sections of the docs the task
  references that the task is supposed to own is present in
  `$GLOSSARY_DIR/GLOSSARY.md` (or the relevant sub-glossary) with a definition.
- `design.md` / `prd.md` cross-refs are updated if implementation diverged from
  the contract (look for divergence signals in the diff).
- READMEs or in-repo examples that reference changed behavior are updated.

### Tests

- Test level for each behavior matches the design's Testing Strategy (unit /
  integration / e2e).
- No `.only`, no `.skip`, no `xfail` without a documented condition.
- Mocks only at system boundaries (no mocking the project's own modules).
- At least one failure-mode test per mocked external boundary.

### Naming

- New module, function, and test names use `$GLOSSARY_DIR/GLOSSARY.md` terms
  verbatim. Drift → gap (kick back to `update-glossary` or rename in place).

## Output

Two verdicts only: `PASS` or `FAIL`.

### PASS

One line confirming the task verified clean. Optionally note any minor
observations under "Notes:" — these are informational only, not gaps.

### FAIL

```
Verdict: FAIL

Gaps:
- <category>: <one-sentence description>. → re-run `<kick-back-skill>`.
- ...

Out-of-scope creep: <none | list>.
Planning-artifact leakage: <none | list of file:line>.
```

Each gap names exactly one kick-back skill. Common mappings:

| Gap kind                              | Kick-back skill                         |
| ------------------------------------- | --------------------------------------- |
| Missing test for ticked AC            | `implement`                             |
| AC unticked but task otherwise done   | `implement`                             |
| Planning-artifact leakage             | `implement` (remove the reference)      |
| Missing glossary term                 | `update-glossary`                       |
| Glossary term drift in code names     | `update-glossary` (or rename in place)  |
| Out-of-scope creep                    | `decompose` (slice was wrong-shaped)    |
| Code contradicts referenced design    | `design-interview` (design has a gap)   |
| Acceptance criteria contradict design | `plan-interview` / `prd-interview`      |
| Blocker incomplete                    | Stop; invoke `implement` on the blocker |

## IMPORTANT: AVOID

- Do not edit code, tests, or docs. Verification is judgment, not action.
- Do not re-run the test suite or lint as a substitute for reading the diff;
  trust prior CI results. Verification is about _what was shipped vs. what was
  promised_, not about re-running quality gates.
- Do not silently overlook a gap because it is "small." Every gap goes in the
  list with its kick-back skill.
- Do not propose a `PASS` if any AC is unticked.
