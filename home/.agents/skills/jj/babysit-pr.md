# Babysit PR

Loop [fix-pr.md](fix-pr.md) over a single PR: keep monitoring it and addressing
blockers (failing CI, merge conflicts, bot + human review comments) until it's
mergeable, then stop. This is the autonomous-watch wrapper around fix-pr; it
adds no new mechanics, only the loop and its exit/pause rules.

Resolve repo + account and the PR number once via [fix-pr.md](fix-pr.md) §1–§2,
then loop.

## Done condition

Stop the loop when the PR is **mergeable**:

```bash
gh pr view $PR --json mergeable,mergeStateStatus,reviewDecision
```

- `mergeStateStatus == "CLEAN"` → mergeable; done.
- `BLOCKED` with `reviewDecision != "APPROVED"` and no failing checks /
  conflicts → only a human's approval is missing. Nothing to fix; surface and
  stop.

Otherwise there's actionable work — continue the loop.

## Loop

Each cycle:

1. **Poll** state: the `gh pr view` above, plus failing checks and unresolved
   threads ([fix-pr.md](fix-pr.md) §5).
2. **Classify and act:**
   - Conflicts (`mergeable == "CONFLICTING"` / `mergeStateStatus == "DIRTY"`) or
     out-of-date (`BEHIND`) → rebase onto base and resolve, then push. This is
     fix-pr §3's rebase path; for a plain `BEHIND` (no conflict) it's just
     [rebase.md](rebase.md) §1.
   - Failing CI **caused by this PR** and/or unresolved threads → run
     [fix-pr.md](fix-pr.md) end to end for this `$PR` (it addresses both, runs
     the CI gate, drafts pending replies).
   - Failing CI **unrelated to this PR** (infra/flake — see
     [Unrelated / flaky CI](#unrelated--flaky-ci)) → try to overcome it
     automatically before surfacing.
   - Nothing actionable, only a human approval pending → **done condition**;
     stop.
3. **Wait for the next signal:**
   - CI in flight → `gh pr checks $PR --watch` blocks until it settles.
   - Idle, waiting on reviewers → re-poll threads + `mergeable` on an interval
     (a few minutes); don't busy-spin. After a long idle with no change, stop
     and report that you're waiting on the reviewer.
4. Repeat.

## Unrelated / flaky CI

A failing check that this PR's diff cannot have caused (infra: missing
artifacts, cache/registry 5xx/400, runner/EC2 timeouts, dead test backend;
managed scans that fail with no findings; checks unchanged code paths). Don't
just surface it — **attempt one recovery pass**, then re-poll:

1. **Behind base** (`mergeStateStatus == "BEHIND"`, or the failure smells stale
   vs. a moved trunk) → fetch latest trunk and rebase the bookmark onto it
   ([rebase.md](rebase.md) §1; resolve per §3 if it conflicts), then push. The
   push re-triggers the full suite on fresh infra. Prefer this whenever the PR
   is behind — it both freshens CI and clears `BEHIND`.
2. **Not behind** → re-run the failed checks **once**. For a workflow whose
   setup must re-provision (e.g. EC2 brought up then torn down in the same run),
   re-run the **whole run** (`gh run rerun <run-id>`), not just the failed jobs
   — re-running failed jobs alone hits already-gone infra. For an app/status
   check that only re-runs on push (e.g. semgrep), an empty re-push or the
   rebase in (1) re-triggers it.

Then wait for the re-triggered checks (`gh pr checks $PR --watch`) and re-poll.
Recovered → continue toward the done condition. **Still failing after this one
recovery pass → it's now in scope for
[Stop and surface](#stop-and-surface-dont-keep-looping): note it and stop.** One
attempt per check — never loop re-runs.

## Stop and surface (don't keep looping)

- **Large / non-trivial fix** — any item that trips fix-pr §6's scope gate
  (broad refactor, behavior/API change, design call, sprawling edits). Pause,
  outline options, wait for the user.
- **Ambiguous review comment** — fix-pr already stops and asks; relay its
  question and resume once answered.
- **Thrash** — the same check keeps failing after two fix attempts. Stop;
  surface the log. Don't loop on it.
- **Unrelated CI failure that survives one recovery pass** — after the rebase or
  single re-run in [Unrelated / flaky CI](#unrelated--flaky-ci) it's still red.
  fix-pr leaves it untouched; note it and keep going on the rest, but it may
  block "mergeable". If it does, surface and stop. Don't re-run it again.

## Pending replies

Replies stay **pending drafts** every cycle — babysit never submits, same as
fix-pr. The user submits in GitHub. Note: code pushes re-trigger bot/CI review,
but unsubmitted reply drafts won't notify human reviewers, so human
re-engagement generally waits on the user submitting.

## Output

On exit, one status line: mergeable (done) | waiting on reviewer/approval |
paused for user input (with the question) | stopped on thrash (with the check).
Then the fix-pr final report ([fix-pr.md](fix-pr.md) §8) for the work done.
