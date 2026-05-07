---
name: implement
description: Implement a task via TDD
argument-hint: <path-to-task-file>
disable-model-invocation: true
model: opus
effort: high
---

Implement a single task end-to-end using test-driven development.

The task file is passed as the argument. Read it, then follow its **References**
section to read the design contract it points to. Also read `GLOSSARY.md` (and
relevant sub-glossaries) — use those terms verbatim in module, function, and
test names. For each entry under the task's `Blocked by` section, read that task
file and confirm its acceptance criteria are all checked.

The acceptance criteria are the source of truth for **what** ships. The
referenced design doc is the source of truth for **how**. Don't relitigate
either; if implementation reveals a real gap, stop and kick back.

## Stop and kick back when

- A blocker task is unfinished → stop. Report that the task isn't ready because
  `{blocker_name}` is incomplete; let the user decide what to do next. Don't
  auto-switch to implementing the blocker.
- The task contradicts a referenced doc → back to whichever interview produced
  that doc (`design-interview`, `prd-interview`, or `plan-interview`). Don't
  paper over it in code.
- The referenced design is silent on a load-bearing question (a contract, an
  invariant, a failure mode) → back to the interview that owns it.
- The task itself is the wrong shape (too thick, too thin, wrong slice) → back
  to `decompose`.
- A new domain term surfaces that isn't in `GLOSSARY.md` and isn't already
  listed under the **Glossary Changes** section of a referenced doc → pause,
  agree on the term with the user, run `update-glossary` (or back to the
  relevant interview if the term is load-bearing). Don't ship code naming a
  concept the glossary doesn't.

## Process

### 1. Plan

Before writing any code, agree with the user on:

- The public interface(s) this slice introduces or changes — names, params,
  return shapes, error cases. Match the design doc; if it's underspecified, pin
  it down with the user before coding.
- The behaviors to test, ordered. Each behavior should map to an acceptance
  criterion or a load-bearing invariant from the design's Testing Strategy.
- The test level for each behavior (unit / integration / end-to-end), per the
  design's Testing Strategy.
- Which collaborators are mocked (system boundaries only) vs. used real.

You can't test everything. Prioritize: critical paths, invariants, failure modes
called out in the design. Confirm the plan with the user before starting the
loop.

### 2. Tracer bullet

Pick the behavior that proves the slice's path works end-to-end — usually the
golden-path acceptance criterion. Then:

```
RED:   write one failing test for that behavior
GREEN: write the minimum code to pass it
```

This wires every layer the slice touches before adding more behaviors.

### 3. Incremental loop

For each remaining behavior, in order:

```
RED:   write one failing test → confirm it fails for the right reason
GREEN: write the minimum code to pass → confirm it passes
```

Rules:

- One test at a time. No batching.
- Only enough code to pass the current test. No speculative branches, no
  imagined future tests.
- Tests describe behavior, not implementation. They should survive an internal
  refactor.
- Tests use public interfaces only. No reaching past the seam.
- Never refactor while RED. Get to GREEN first.
- No references to planning artifacts in code or tests. Don't mention task
  files, design docs, PRDs, plan files, or acceptance criteria by name in
  comments, docstrings, identifiers, or commit messages. The code must read as
  if it was written without any planning artifacts.

### 4. Refactor

When all planned behaviors are GREEN, look for:

- Duplication → extract.
- Shallow modules → deepen (push complexity behind the interface). Apply the
  design skill's deletion test: if a module vanishes and complexity doesn't
  reappear across callers, delete it.
- Pass-through layers / thin wrappers → merge into caller or callee.
- Existing code the new code exposes as problematic → fix in place if cheap,
  otherwise note it.

Run the full test suite after each refactor step. Never refactor while RED.

### 5. Done

Before declaring done:

- All acceptance criteria in the task file are checked off (`- [x]`).
- All tests pass; no skipped or `.only` tests.
- Lint / typecheck / format pass per the project's conventions.
- For UI changes: exercise the feature in a browser. Verify the golden path and
  at least one edge case. If you can't run the UI, say so explicitly rather than
  claiming success.
- Docs owed by this slice are updated in the same change:
  - Glossary entries the slice owns — apply via `update-glossary`.
  - `design.md` / `prd.md` cross-references if implementation diverged from the
    contract (with user approval).
  - READMEs, code examples, or in-repo docs that reference the changed behavior.
- Anything left undone is captured back in the task file's Out of scope or
  surfaced to the user — don't leave silent gaps.

Update the task file in place: tick checkboxes, append notes only when they
record a non-obvious decision (a constraint discovered during implementation, a
deviation from the design with the user's approval).

## Tests: what good looks like

Good tests:

- Exercise public interfaces — the same seam callers use.
- Describe user-visible or contract-visible behavior in their names.
- Survive internal refactors. If renaming a private function breaks a test, the
  test was wrong.
- Verify outcomes through the interface ("create then read it back"), not by
  poking at internal state or storage directly.

Avoid:

- Mocking your own modules. Mock only at system boundaries (external APIs,
  clock/randomness, sometimes the database — prefer a real test DB).
- Asserting on call counts, call order, or "this internal method was invoked."
  That's testing how, not what.
- Testing private methods. If something's worth testing, expose it through the
  public interface — or it shouldn't exist.
- Bulk-writing tests up front. Horizontal slicing produces tests of _imagined_
  behavior, not observed behavior.

## Mocking

Mock at boundaries the design doc identifies as external. For each mocked
boundary, confirm:

- The mock's contract matches what the real system actually returns (including
  error shapes).
- At least one test exercises the failure mode (timeout, 5xx, partial response)
  — not just the happy path.

If a "mock" is wrapping one of your own modules, that's a sign the boundary is
in the wrong place; redesign before continuing.

## Checklist

Work top-to-bottom. Don't skip; each item gates the next. Detail for each step
lives in the sections above.

- [ ] **Inputs read**: task, the docs it references, glossary, all `Blocked by`
      tasks.
- [ ] **Blockers complete**: every blocker's acceptance criteria are `[x]`.
- [ ] **No kick-back trigger fires** (see _Stop and kick back when_).
- [ ] **Plan agreed with user**: interfaces, ordered behaviors, test levels,
      mocked boundaries.
- [ ] **Tracer bullet GREEN**: one end-to-end test for the golden path passes.
- [ ] **Incremental loop**: every planned behavior added via RED → GREEN, one at
      a time.
- [ ] **Refactor pass**: duplication, shallow modules, pass-throughs addressed;
      full suite green after each step.
- [ ] **Acceptance criteria** in the task file all ticked `[x]`.
- [ ] **Quality gates pass**: full test suite, lint, typecheck, format. No
      skipped or `.only` tests.
- [ ] **UI verified** in a browser (golden path + one edge case), or explicit
      "couldn't run UI" note.
- [ ] **Docs updated in the same change**: glossary entries via
      `update-glossary`; design.md/prd.md cross-refs (with user approval) if
      behavior diverged; READMEs / examples that reference changed behavior.
- [ ] **No planning artifact leakage**: no references to task files, design
      docs, PRDs, or plan files in code, tests, comments, or identifiers. Code
      reads as if written without planning artifacts.
- [ ] **Gaps surfaced**: anything undone is in the task file's Out of scope or
      raised to the user — no silent omissions.
