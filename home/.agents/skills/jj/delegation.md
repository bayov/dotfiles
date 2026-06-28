# Delegating to subagents (orchestrator only)

This file is for the **top-level skill invocation** (the orchestrator).
Subagents do not read it: a subagent is handed a workflow and is the executor of
it. The non-recursion rules below reach subagents through the prompt you write
for them, not by them re-reading this file.

These workflows produce large volumes of `jj`, `gh`, test, and CI output that
bloat your context. Keep your context clean by **delegating the heavy execution
to subagents** (Agent tool, `general-purpose`). Every delegation prompt must
tell the subagent to first read [`SKILL.md`](SKILL.md) (for the gh-account,
pushing, and convention rules) **and** the specific workflow file(s), then carry
out the task with all specifics passed in (PR number, branch, Linear ID, user
instructions). State plainly in the prompt that the subagent is the executor of
that workflow and must run it inline itself: it must NOT spawn a further
subagent to re-run the same workflow (it may still delegate strictly smaller
sub-units of heavy work). The subagent absorbs the command/test/CI output and
returns a **concise summary** (what changed, PR URL, pass/fail, anything needing
a decision).

Pick the granularity by workflow shape. Do **not** hand an entire long-running
loop to one subagent, or that subagent's context just explodes instead:

- **Bounded workflows** ([new-pr](new-pr.md), [update-pr](update-pr.md),
  [fix-pr](fix-pr.md), [rebase](rebase.md), [sync](sync.md)) run to completion,
  so delegate the whole workflow to **one** subagent.
- **Loop / watch workflows** ([babysit-pr](babysit-pr.md)): the loop is
  unbounded (poll, classify, fix, push, re-review, over many CI/codex cycles).
  **Run the loop yourself.** Polling state, classifying, and applying the
  exit/pause rules are cheap (a `gh pr view`, a state check). Delegate each
  iteration's **heavy unit of work** to its own **fresh** subagent: a fix-pr
  run, a rebase+resolve, a CI-log fetch+triage. A fresh subagent per cycle means
  no single context accumulates across cycles; you keep only the loop state plus
  each cycle's concise summary.

General rule: delegate any bounded unit of heavy work (implementation,
fix/refactor, test runs, `jj`/`gh` execution, log triage); keep orchestration
and decisions with yourself. Run independent units in parallel (one message,
multiple Agent calls). Reserve inline execution for one-off read-only checks
(e.g. a single `jj log` or `jj st`). When a unit needs a user decision, the
subagent surfaces it; you relay it and re-delegate with the answer.

Recursion terminates because each hop strictly shrinks the task: you delegate a
whole workflow, the executor may delegate only narrower sub-units of it, and so
on down to leaf work that is done inline. A subagent may spawn its own children
for genuinely heavy, narrower sub-units (e.g. an implementation split into
parallel pieces, a big test run), but never re-delegates the **same** workflow
(the trap that loops forever). So every subagent prompt you write must say: you
are the executor of this workflow, run it inline, do not spawn a subagent to
re-run it.

## Model selection

Set the Agent tool's `model` to match the task — don't pay for Opus (slower,
more expensive, more thinking) on simple work:

- **`sonnet`** (default) — mechanical or well-scoped work: running tests/CI
  gates, `jj`/`gh` execution, log fetch+triage, rebases with no conflicts,
  status polls, drafting routine PR text.
- **`opus`** — genuinely hard reasoning: resolving merge conflicts, addressing
  code-review comments, fixing failing code, non-trivial refactors, any design
  call. When unsure whether a task is hard, start with `sonnet`; escalate to a
  fresh `opus` subagent only if it stalls or the task proves to need it.

Apply the same split when a subagent spawns children for narrower sub-work.
