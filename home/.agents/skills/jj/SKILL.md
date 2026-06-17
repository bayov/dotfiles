---
name: jj
description: Manage jj repos and PRs in Cato.
disable-model-invocation: true
---

## jj workflows

Identify the relevant workflows based on the user's request. Multiple workflows
may apply.

- [new-pr.md](new-pr.md) → Create new PR
- [update-pr.md](update-pr.md) → Update existing
- [fix-pr.md](fix-pr.md) → Address a PR's failing CI + unresolved review
  comments
- [babysit-pr.md](babysit-pr.md) → Loop fix-pr, watching a PR until mergeable
- [rebase.md](rebase.md) → Rebase PRs and/or local commits
- [sync.md](sync.md) → Fetch latest trunk and rebase all PRs and commits onto it

## Delegate to subagents

These workflows produce large volumes of `jj`, `gh`, test, and CI output that
bloat your context. Keep your context clean by **delegating the heavy execution
to subagents** (Agent tool, `general-purpose`). Every delegation prompt must
tell the subagent to first read this `SKILL.md` (for the gh-account, pushing,
and convention rules) **and** the specific workflow file(s), then carry out the
task with all specifics passed in (PR number, branch, Linear ID, user
instructions). The subagent absorbs the command/test/CI output and returns a
**concise summary** (what changed, PR URL, pass/fail, anything needing a
decision).

Pick the granularity by workflow shape — do **not** hand an entire long-running
loop to one subagent, or that subagent's context just explodes instead:

- **Bounded workflows** ([new-pr](new-pr.md), [update-pr](update-pr.md),
  [fix-pr](fix-pr.md), [rebase](rebase.md), [sync](sync.md)) — run to
  completion, so delegate the whole workflow to **one** subagent.
- **Loop / watch workflows** ([babysit-pr](babysit-pr.md)) — the loop is
  unbounded (poll → classify → fix → push → re-review, over many CI/codex
  cycles). **Run the loop yourself** — polling state, classifying, and applying
  the exit/pause rules are cheap (a `gh pr view`, a state check). Delegate each
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

Subagents follow the same discipline: when a delegated unit itself contains
heavy or independently-parallelizable sub-work, the subagent should delegate
that to its own subagents rather than absorbing it all. Keep nesting shallow —
each hop adds latency and loses fidelity (the result is a summary of a summary)
— so spawn a child only when the sub-work is genuinely heavy, not by reflex.

### Model selection

Set the Agent tool's `model` to match the task — don't pay for Opus (slower,
more expensive, more thinking) on simple work:

- **`sonnet`** (default) — mechanical or well-scoped work: running tests/CI
  gates, `jj`/`gh` execution, log fetch+triage, rebases with no conflicts,
  status polls, drafting routine PR text.
- **`opus`** — genuinely hard reasoning: resolving merge conflicts, addressing
  code-review comments, fixing failing code, non-trivial refactors, any design
  call. When unsure whether a task is hard, start with `sonnet`; escalate to a
  fresh `opus` subagent only if it stalls or the task proves to need it.

Apply the same split when a subagent spawns its own children.

## GitHub account

Several gh accounts are authed; the right one is fixed by the repo's remote
owner, not by which clone you're in. `GH_TOKEN` overrides the active account
per-command without a global `gh auth switch`. Resolve owner + repo + account
once:

```bash
url=$(jj git remote list | awk '$1=="origin"{print $2}')
OWNER=$(printf '%s' "$url" | sed -E 's#\.git$##; s#^.*[:/]([^/]+)/[^/]+$#\1#')
REPO=$(printf  '%s' "$url" | sed -E 's#\.git$##; s#^.*/([^/]+)$#\1#')
echo "$OWNER/$REPO"
```

| owner         | gh account                |
| ------------- | ------------------------- |
| hakasecurity  | `hxhagai`                 |
| cato-networks | `hagai-akibayov_catonian` |

Apply the token inline on every `gh`/`gh api` call, substituting the resolved
account:

```bash
GH_TOKEN=$(gh auth token --user {account}) gh pr view ...
```

`$OWNER`/`$REPO`/`$GH_TOKEN` in the workflow snippets are placeholders. Unknown
owner → stop and ask. (Scopes `gh` only; jj/git push auth — SSH or https
credential helper — is separate.)

## Pushing

Pushes must be fast-forward: never amend or rewrite an already-pushed commit —
new edits go in commits on top (update-pr, fix-pr). The exception is
rebase/sync, which rewrite history by design and force-update the push. A
non-fast-forward rejection outside a rebase → stop and ask.

**Before editing files, check whether `@` is already pushed.** In jj the working
copy is itself a commit, so file edits land in `@` automatically. If `@` is a
pushed commit, those edits silently rewrite pushed history and you must later
split them into a separate commit to keep the push fast-forward. Avoid this:
when `@` is pushed, run `jj new` first so edits land in a fresh commit on top.
Check with:

```bash
jj log -r '@ & ::remote_bookmarks()' --no-graph -T commit_id | grep -q . \
  && echo "@ is pushed — run 'jj new' before editing" || echo "@ is unpushed — safe to edit"
```

## Conventions

**No LLMisms in any generated text** (PR titles, bodies, commit descriptions,
review replies): no em-dashes (`—`) — use a period, colon, comma, or parens
instead; no greetings, praise, filler, or sign-offs. Plain ASCII punctuation.

### PR title & body

Each PR is tied to a Linear issue ID (`{team_id}-{number}`, e.g. `ENG-10333`).

**PR title**: `{linear_issue_id} {pr_title}`. Sentence case. ≤80 chars.

Examples:

- `ENG-10333 Move unique-count gauge emission to a cron`
- `HS-10346 Cache shared proto generation to improve CI performance`

**PR body**: As short as possible. The goal is that a reviewer grasps what the
PR is about in one read.

- Open with ONE sentence capturing the essence (e.g. "Move CD Docker layer cache
  from `type=gha` to AWS ECR.").
- Add a second sentence ONLY for a genuinely non-obvious caveat, risk, or
  backward-compat note. Stop there. Most PRs need just the one sentence.
- Never restate the diff. If a reviewer can learn it by reading the code, leave
  it out.
- Avoid "Summary"/"Changes"/"Risk"/"Verification"/"Test" headers unless
  requested by the user.
- Reference related PRs inline (e.g., "uses API introduced in #15053").
- For PRs/commits/files in **other** repos, link with `[...](full_url)`, not
  `owner/repo#123`: repos are private under different accounts, so the shorthand
  doesn't render.
- Omit the body entirely when the title already says it all.
- Wrap code/identifiers in backticks.
- Wrap paths in links (e.g.,
  [`foo/bar.py`](https://github.com/{owner}/{repo}/blob/{commit_sha}/foo/bar.py)).
- When highlighting a code snippet, don't wrap link to ensure widget renders
  natively in GitHub:
  https://github.com/{owner}/{repo}/blob/{commit_sha}/foo/bar.py#L42-L56
  (without backticks).

Examples:

```
Bumps ruff 0.14.7 -> 0.15.15. All `*.py` changes are automatic ruff fixes.
```

```
Move CD Docker layer cache from GH storage to AWS ECR (`ci-build-cache` repo).
```

### Commit description

Short, imperative mood, ≤72 chars, no Linear issue key.

- Good: `Remove foo reference from MONOREPO-MAP`
- Bad: `HS-10349 - removed foo reference from monorepo map`

No need for an extended description; a one-line title suffices. Commits are
squash-merged anyway, so only PR descriptions are durable.

## Before opening or updating a PR

Run from repo root, scoped to each service touched:

- `just pre-commit` - Global pre-commit hooks (format, lint).
- `just {service_path}/ pre-commit` - Per-service pre-commit. Run after global.
- `just {service_path}/ {test_recipe}` - Relevant per-service tests. Varies by
  service.

If the change touches:

- FastAPI routes or protos → `just generate-api-clients` (**expensive**: Only
  run if generated client files are missing or stale/broken).
- `schema/` ORM → `just migrations "{message}"`.
- Go module deps changed → run `go mod tidy` in each affected service directory
  first; only run `just generate-api-clients` if `go mod tidy` still fails or
  generated client files are missing (**expensive**).
