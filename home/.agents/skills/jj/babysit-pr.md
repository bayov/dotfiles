# Babysit PR

Loop [fix-pr.md](fix-pr.md) over a single PR: keep monitoring it and addressing
blockers (failing CI, merge conflicts, bot + human review comments) until it's
mergeable **and codex is clean**, then stop. This is the autonomous-watch
wrapper around fix-pr; it adds no new mechanics, only the loop and its
exit/pause rules.

Codex re-reviews on **every push** ([Codex reviews](codex-reviews.md)), so each
cycle can draw a fresh review. The loop must catch those: ping-pong (fix → push
→ re-review → fix …) until codex reacts 👍, not just until the first review is
handled.

Resolve repo + account and the PR number once via [fix-pr.md](fix-pr.md) §1–§2,
then loop.

## Done condition

Every cycle, always fetch the full unresolved-thread list
([fix-pr.md](fix-pr.md) §5, all authors) - the done check reads that list, never
a separate codex-scoped query. Stop only when **all** hold: the PR is mergeable
(`gh pr view $PR --json mergeable,mergeStateStatus,reviewDecision` →
`mergeStateStatus == "CLEAN"`), codex is not mid-review, and the §5 list has no
unresolved **bot** thread ([Codex reviews](codex-reviews.md)).

- Codex mid-review (👀, or latest review on an older commit) → **not done**;
  wait and re-poll, like CI in flight. Never declare done off a stale reaction.
- Any unresolved **bot** thread in the §5 list → actionable; continue. A
  `COMMENTED` review on HEAD is a findings signal - go read its threads, don't
  read it as done.
- `BLOCKED`, `reviewDecision != "APPROVED"`, no failing checks / conflicts, no
  unresolved bot threads → only a human approval (or an unresolved human thread)
  is missing; surface and stop.

Otherwise there's actionable work — continue the loop.

## Loop

Each cycle:

1. **Poll** state: the `gh pr view` above; failing checks and unresolved threads
   ([fix-pr.md](fix-pr.md) §5); and codex's stance on HEAD
   ([Codex reviews](codex-reviews.md)).
2. **Classify and act:**
   - Conflicts (`CONFLICTING` / `DIRTY`) or out-of-date (`BEHIND`) → rebase onto
     base and resolve, then push (fix-pr §3's path; plain `BEHIND` is just
     [rebase.md](rebase.md) §1).
   - Failing CI **caused by this PR** and/or unresolved threads → run
     [fix-pr.md](fix-pr.md) end to end (it closes both, pushes the fixes, and
     settles codex's re-review on the pushed HEAD).
   - Failing CI **unrelated to this PR** (infra/flake — see
     [Unrelated / flaky CI](#unrelated--flaky-ci)) → try to overcome it
     automatically before surfacing.
   - Codex 👀 with nothing else actionable → fall through to wait.
   - Nothing actionable, codex-clean, only a human approval pending → **done**.
3. **Wait for the next signal:**
   - CI in flight → `gh pr checks $PR --watch --fail-fast` blocks but returns on
     the **first** failure - don't wait for the whole suite to settle. Act on
     that failure right away (loop back to step 2 / [fix-pr.md](fix-pr.md) §6);
     the fix's push re-triggers CI anyway, so there's no value in waiting for
     the remaining checks first. Clean settle (all green) → continue.
   - Codex 👀 → re-poll on an interval until it settles. Push landed but no
     reaction/review after a long idle → nudge once with `@codex review`.
   - Idle on human reviewers → re-poll threads + `mergeable` on an interval;
     don't busy-spin. After a long idle, stop and report you're waiting.
4. Repeat.

Every push (fix, rebase, or the empty re-push in
[Unrelated / flaky CI](#unrelated--flaky-ci)) restarts codex — re-confirm its
stance is on the **current** HEAD before declaring done.

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

Then watch the re-triggered checks (`gh pr checks $PR --watch --fail-fast`,
returning on the first failure) and re-poll. Recovered → continue toward the
done condition. **Still failing after this one recovery pass → it's now in scope
for [Stop and surface](#stop-and-surface-dont-keep-looping): note it and stop.**
One attempt per check — never loop re-runs.

## Stop and surface (don't keep looping)

- **Large / non-trivial fix** — any item that trips fix-pr §6's scope gate
  (broad refactor, behavior/API change, design call, sprawling edits). Pause,
  outline options, wait for the user.
- **Ambiguous review comment** (human, or a codex comment needing a human
  decision) — fix-pr stops and escalates; relay its question and resume once
  answered.
- **Thrash** — the same check keeps failing after two fix attempts. Stop;
  surface the log. Don't loop on it.
- **Unrelated CI failure that survives one recovery pass** — after the rebase or
  single re-run in [Unrelated / flaky CI](#unrelated--flaky-ci) it's still red.
  fix-pr leaves it untouched; note it and keep going on the rest, but it may
  block "mergeable". If it does, surface and stop. Don't re-run it again.

## Replies and codex threads

Same closings as fix-pr every cycle: **codex** threads resolved directly (no
user wait — this is what lets the ping-pong run autonomously to 👍); **human**
replies stay **pending drafts** that babysit never submits. Pushes re-trigger
bot/CI, but unsubmitted drafts don't notify humans, so human re-engagement waits
on the user.

## Output

On exit, one status line: mergeable + codex 👍 (done) | waiting on codex (👀) |
waiting on reviewer/approval | paused for user input (with the question) |
stopped on thrash (with the check). Then the fix-pr final report
([fix-pr.md](fix-pr.md) §8) for the work done, including codex's final stance.
