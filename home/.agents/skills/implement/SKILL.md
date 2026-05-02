---
name: implement
description: Implement a task via TDD
disable-model-invocation: true
---

Implement a single task file (`plan/{slug}/tasks/{task_name}.md`) end-to-end
using test-driven development.

The task file is the source of truth for **what** ships in this slice. The
design doc is the source of truth for **how**. Don't relitigate either; if
implementation reveals a real gap, stop and kick back.

## Input

Before implementing, read:

- `plan/{slug}/tasks/{task_name}.md` — the task. Acceptance criteria,
  References, Out of scope, and Blocked by are authoritative.
- `plan/{slug}/design.md` — components, interfaces, data model, migration plan,
  and testing strategy. Don't redesign here.
- `plan/{slug}/prd.md` — user stories the slice advances. Use these to phrase
  test names in user-visible terms.
- `GLOSSARY.md` (and relevant sub-glossaries) — use these terms verbatim in
  module, function, and test names.
- Any task files listed under `Blocked by` — confirm their acceptance criteria
  are all checked. If not, stop.

## Stop and kick back when

- A blocker task is unfinished → resume `implement` on that task first.
- The task contradicts the design or PRD → back to `design-interview` or
  `prd-interview`. Don't paper over it in code.
- The design is silent on a load-bearing question (a contract, an invariant, a
  failure mode) → back to `design-interview`.
- The task itself is the wrong shape (too thick, too thin, wrong slice) → back
  to `decompose`.

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
