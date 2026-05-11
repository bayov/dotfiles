---
axis: tests
---

# Tests review

Adequacy and quality of tests for the behavior added or changed by the diff.
Covers both "are tests present" and "are tests well-shaped."

## What to look for

### Coverage of new behavior

- Every new public function/endpoint/handler has at least one test that
  exercises its golden path.
- Each branch, error case, and edge case the code explicitly handles has a
  corresponding test (or is intentionally untested with reason).
- A diff that adds production code with **zero** new tests, but isn't a trivial
  refactor or pure type change, is itself a finding.

### Test level

- The test level (unit / integration / e2e) matches the design's Testing
  Strategy if a referenced design doc exists, or the project's prevailing
  pattern for similar code.
- Behaviors that require multiple components in concert are exercised at the
  integration or e2e level, not faked at the unit level with mocks.
- Behaviors that are purely a single function's logic are exercised at the unit
  level, not bloated into an e2e test.

### Test quality

- **Tests describe behavior**, not implementation. Names like
  `archive_note_hides_it_from_default_list` not `test_archive_function`.
- **Public interfaces only**. Tests call the same seam as real callers, not
  private functions, not internal state.
- **Outcomes, not interactions**. Assertions on observable results ("create then
  read it back returns X"), not on which internal method was invoked or how many
  times. Strong call-count assertions are an anti-pattern unless the count is
  the contract.
- **Survives refactors**. Renaming a private symbol must not break a test.

### Mocking

- Mocks only at system boundaries: external APIs, clock/randomness, network,
  filesystem when not test-owned. Prefer real test databases over mocked DBs.
- No mocking the project's own modules (mocking a sibling unit means the
  boundary is in the wrong place or the test is at the wrong level).
- Each mocked boundary has at least one test that exercises its **failure** mode
  (timeout, 5xx, partial response, schema mismatch) — not just the happy path.

### Hygiene

- No `.only`, `it.only`, `test.only`, `fdescribe`, `fit`.
- No skipped or `xfail` tests added without a documented condition for removal.
- No commented-out tests.
- New tests deterministic: no real wall-clock dependency, no real network/DNS,
  no real random without seed, no test ordering dependence.

### Tests as documentation

- Test names read as specifications of behavior. A reader skimming test names
  should learn what the system does.
- Long arrange blocks → factor out builders/fixtures; tests should foreground
  the behavior under examination.

## What this axis does NOT cover

- Whether the production code under test is correct (`production.md`).
- Whether tests catch security regressions specifically (`security.md`, though
  "no test for the auth check" is a tests-axis finding).

## Severity calibration

- **Critical**: production code added with no tests and no exemption reason;
  test added with `.only` that would silently skip the full suite in CI; test
  asserting on an internal mock counter in place of an actual behavior
  assertion.
- **Major**: a new behavior has a happy-path test but no failure-mode test for
  the boundary it crosses; tests mocking the project's own modules; brittle
  assertions on internal state; non-deterministic test (real time / real
  network).
- **Minor**: test name doesn't describe behavior; long arrange that could use a
  builder; test at the wrong level (unit where integration would fit better, or
  vice versa).

## Output

Per the dispatcher's schema. `recommendation` names the test to add (which
behavior, at which level) or the specific test to restructure.
