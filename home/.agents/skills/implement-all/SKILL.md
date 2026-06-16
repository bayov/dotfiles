---
name: implement-all
description: Implement every task in a decomposed plan, building and babysitting a PR stack, orchestrating via subagents
argument-hint: <plan-slug-or-tasks-dir>
disable-model-invocation: true
---

Implement an entire decomposed plan end-to-end: walk its tasks in dependency
order, implement each via the `implement` skill, open one PR per task as a
**stack**, and babysit each PR to mergeable. This is a long-running flow.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset). Tasks live
> under `$PLAN_DIR/{slug}/tasks/{NN}-{name}.md`, numbered in dependency order by
> `decompose`.

## You are an orchestrator. Delegate everything.

The main agent (you) is a coordinator only. Your context window must stay clean
for the whole run. Therefore:

- **Never** edit code, run TDD, read a diff, read CI logs, read full task
  bodies, or run a repo-mutating `jj`/`gh` command yourself.
- **Every** unit of work goes to a fresh subagent (Agent tool, `general-purpose`
  unless noted): enumeration, each task's implementation, each jj operation,
  each babysit, each rebase. Subagents return a **compact summary** (a few lines
  / a small object) — never paste their diffs or logs back into your context.
- You hold only the **ledger** (below) plus the current step. When in doubt
  about state, re-read the ledger file rather than recalling from context.

Subagents follow the existing skills by **reading the skill file directly** (the
`implement`/`jj` skills are `disable-model-invocation`, so they can't be invoked
as tools from a subagent). Give each subagent the absolute path to the file it
must follow.

## Serialization (critical)

The jj working copy and operation log are **shared** across all subagents in
this repo. Running two repo-mutating subagents at once corrupts state. Rule:

- **At most one repo-mutating subagent runs at a time.** Implementation, jj
  positioning, new-pr, fix/babysit, and rebase are all serial. Do **not**
  background them (`run_in_background`) and do not fan them out with `parallel`.
- Only read-only enumeration (an `Explore` subagent reading task files) may run
  freely.

Babysit's done condition is **mergeable**, not merged — it returns once CI is
green and conflicts are cleared even while human approval is pending. So serial
execution still progresses; it never blocks waiting for a human to click merge.

## The ledger

Single source of truth, survives context summarization. Path:
`$PLAN_DIR/{slug}/.implement-all.md`. Create it after enumeration; update it
after every subagent returns. Schema:

```md
# implement-all: {slug}

slug: {slug} · plan_dir: {abs path} · trunk: {branch}

| # | task file | blocked by | base      | bookmark  | PR  | status    |
| - | --------- | ---------- | --------- | --------- | --- | --------- |
| 1 | 01-foo.md | —          | trunk     | eng-1-foo | url | mergeable |
| 2 | 02-bar.md | 01-foo     | eng-1-foo | eng-2-bar | url | pr-open   |
| 3 | 03-baz.md | —          | trunk     | —         | —   | pending   |
```

`status` ∈ `pending` → `implemented` → `pr-open` → `mergeable`, plus
`paused({why})` and `blocked({why})`. `base`/`bookmark`/`PR` fill in as they're
created.

## Process

### 0. Resolve inputs & enumerate (read-only subagent)

Resolve the slug/tasks dir from the argument and `$PLAN_DIR`. Then spawn one
`Explore` subagent to enumerate — keeps every task body out of your context:

> Read `{tasks_dir}/*.md` (numbered task files) in order. For each, return: the
> filename, its one-line title, and the task names under its `Blocked by`
> section (empty if none). Also report whether the dependency graph is acyclic
> and whether every named blocker exists as a file. Return a compact table only,
> no task bodies.

Validate: graph acyclic, every blocker present. If not → stop and send the user
back to `decompose`. Resolve `trunk` once
(`gh repo view --json defaultBranchRef -q .defaultBranchRef.name`, or ask a
subagent to). Write the ledger with all tasks `pending`.

**Resuming:** if the ledger already exists, read it and skip any task already at
`pr-open`/`mergeable`; resume at the first incomplete row.

### 1. Build the stack (serial, one task at a time)

For each `pending` task in ledger order:

**Compute the base.** `base` = the bookmark of this task's highest-numbered
already-built blocker; if it has no blockers, `base = trunk()`. This makes
dependent tasks stack and independent tasks branch from trunk (separate PRs —
fine). If a task has multiple blockers that are **not** themselves a chain (so
the result can't be a single linear stack) → **stop and ask** the user which
parent to base it on; don't guess.

**1a. Position** — jj subagent (follows `~/.agents/skills/jj/SKILL.md`):

> Position the jj working copy for new edits on top of `{base}`: run
> `jj new
> {base}` unless `@` is already an empty commit whose parent is
> `{base}`. Follow the "Pushing" / "@ is pushed" safeguards in
> `~/.agents/skills/jj/SKILL.md`. Return one line confirming `@`'s parent.

**1b. Implement** — implementation subagent, **model opus**, follows the
implement skill:

> Implement the single task at `{abs task path}` by reading and following
> `~/.agents/skills/implement/SKILL.md` exactly (TDD, write `.task-reference`,
> tick acceptance criteria, run quality gates). Do NOT create, push, or open any
> PR and do NOT run jj/gh — leave all published-history work to the caller; just
> leave the finished edits in the working copy. Return a compact summary: done
> vs kicked-back, and if kicked-back the trigger and reason. No diffs.

The implement skill's interactive "agree the plan with the user" step: the
subagent surfaces its plan back to you; relay it to the user and pass their
answer back via `SendMessage`. Don't approve plans on the user's behalf.

If the subagent **kicks back** (blocker incomplete, design contradiction,
glossary gap, wrong-shaped task — see the implement skill's _Stop and kick back
when_) → set the row `blocked({reason})`, **stop the whole flow**, surface, and
let the user decide. Don't auto-fix or skip ahead.

Mark the row `implemented`.

**1c. Open the PR** — jj subagent (follows `~/.agents/skills/jj/new-pr.md`):

> Open a PR for the edits currently in the working copy by reading and following
> `~/.agents/skills/jj/new-pr.md`. The PR base is `{base}` (`trunk()` → normal
> PR; a bookmark → **stacked** PR: also follow
> `~/.agents/skills/jj/stacked-pr-guidelines.md` and include the `### PR Stack`
> section). `.task-reference` is present — resolve the Linear issue from it.
> After pushing, return only: the new bookmark name and the PR URL.

Record `base`, `bookmark`, `PR`; set the row `pr-open`. Move to the next task.

When every row is `pr-open`, the stack exists. Refresh each PR's `### PR Stack`
section if the final order differs from what each was opened with — delegate one
jj subagent to update the stack sections across the bookmarks
(`~/.agents/skills/jj/stacked-pr-guidelines.md`). Report the full stack (PR URLs
bottom→top) to the user before babysitting.

### 2. Babysit the stack (serial, bottom-up)

Walk PRs from the stack **bottom** (trunk-based) upward. For each:

**2a. Babysit** — subagent (follows `~/.agents/skills/jj/babysit-pr.md`):

> Babysit PR `{url}` (bookmark `{bookmark}`) by reading and following
> `~/.agents/skills/jj/babysit-pr.md` until its done/stop condition. Return only
> a compact status: `mergeable` | `waiting-on-reviewer` | `paused({question})` |
> `stopped({reason})`, and whether you pushed any new commits to the bookmark
> (yes/no).

- `mergeable` / `waiting-on-reviewer` → set row `mergeable`; continue upward.
- `paused` / `stopped` → set row `paused({why})`, surface the question/reason,
  and **stop** (resume later from the ledger once the user answers, relaying via
  `SendMessage`).

**2b. Restack if the base moved.** If 2a reported it **pushed new commits** to
this PR, every PR above it in the stack is now behind. Delegate one rebase
subagent to restack the descendants:

> Restack the non-merged PRs above `{bookmark}` onto its updated tip by reading
> and following `~/.agents/skills/jj/rebase.md` §2 (resolve conflicts per §3).
> Push each rebased bookmark. Return the bookmarks pushed and any unresolved
> conflicts.

Then continue to the next PR up.

### 3. Done

When every row is `mergeable` (or terminally `paused`/`blocked`), report a final
table: each task → PR URL → status. State plainly which PRs are mergeable now,
which wait on human approval, and which are paused/blocked with the reason.
Merging is the user's to do (stacks merge bottom-up; see
`~/.agents/skills/jj/stacked-pr-guidelines.md`).

## Stop and surface (don't push through)

- An implement subagent kicks back → §1b: stop, surface, let the user decide.
  Never auto-implement a missing blocker or paper over a design contradiction.
- A task can't be linearized into the stack (divergent multi-blocker) → ask
  which parent to base on.
- A babysit pauses (ambiguous review comment, large fix beyond fix-pr's scope
  gate, thrash, unrelated CI that survives one recovery pass) → relay and stop.
- A jj/rebase subagent reports a non-fast-forward rejection or a genuinely
  ambiguous semantic conflict → stop and ask; don't force past it.

## Checklist

- [ ] **Enumerated** via a read-only subagent; graph acyclic, blockers present.
- [ ] **Ledger** written at `$PLAN_DIR/{slug}/.implement-all.md`; updated after
      every subagent returns.
- [ ] **Orchestrator stayed clean**: no code edits, diffs, CI logs, or
      repo-mutating commands run in the main context — all delegated.
- [ ] **Serialized**: never more than one repo-mutating subagent at a time; none
      backgrounded.
- [ ] **Per task**: positioned → implemented (opus subagent, implement skill) →
      stacked PR opened. Kick-backs stopped the flow.
- [ ] **Stack coherent**: `### PR Stack` sections correct across all PRs.
- [ ] **Babysat bottom-up**: each PR to mergeable; descendants restacked after
      any base moved.
- [ ] **Final report**: task → PR → status; pauses/blocks surfaced with reasons.
