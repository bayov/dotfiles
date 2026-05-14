---
name: jj
description: Open Linear issues and GitHub PRs.
disable-model-invocation: true
---

# jj workflow

Use jj for repo operations. The skill captures _already-made_ edits into Linear
issues and PRs — it does not write code. Work the decision tree top-down:
identify the task, check state, take the matching action.

## Identify the task

Pick exactly one entry point from the user's request:

- **New PR** — edits exist locally, no PR yet → [new-pr.md](new-pr.md)
- **Update PR** — edits exist locally, a PR already covers this work →
  [update-pr.md](update-pr.md)
- **Stacked PR** — open a new PR on top of an existing one →
  [stacked-pr.md](stacked-pr.md)
- **Rebase** — pull latest `main` and rebase the current PR (or stack) onto it →
  [rebase.md](rebase.md)
- **Sync** — pull latest `main` and rebase every local PR bookmark in the
  workspace onto it → [sync.md](sync.md)
- **Recovery** — a command failed or the repo is in an unexpected state →
  [recovery.md](recovery.md)

PRs are the unit. Whether a PR contains one commit or several is an
implementation detail; never restructure commits unless the user asks.

**Before any jj mutation**, capture the current op ID with
`jj op log -n 1 --no-graph -T 'id.short()'` and hold onto it. Report it at
end-of-turn so the user can `jj op restore <id>` if anything went wrong.

Bookmark and PR text rules apply across all paths →
[naming](#naming-and-writing). End-of-turn surfacing rules →
[surface](#surface-to-the-user-at-end-of-turn).

## Naming and writing

**Bookmark**: `<issue-key-lower>-<kebab-title>`. Cap at 80 chars; trim the
title, never the key. Example:
`hs-10349-remove-foo-reference-from-monorepo-map`.

**Commit title**: imperative mood, ≤72 chars, no issue key.

- Good: `Remove foo reference from MONOREPO-MAP`
- Bad: `HS-10349 - removed foo reference from monorepo map`

**Commit body**: optional, one short paragraph on _why_. Omit when the title
suffices.

**PR title**: `HS-NNNN <Sentence-case title>`. Examples:

- `HS-10333 Move unique-count gauge emission to a cron`
- `HS-10346 Cache shared proto generation to improve CI performance`

**PR body**:

- Description of the change, if not obvious from the title or commit messages.

## Before opening or updating a PR

Run from repo root, scoped to each service touched:

- `just <service-path>/ pre-commit` — lint, type-check, format
- `just <service-path>/ <test-recipe>` — recipe name varies by service (e.g.
  `unit-test`, `unit-tests`); check the service's justfile. Cached, so re-runs
  are cheap.

If the change touches:

- FastAPI routes or protos → `just generate-api-clients` (**expensive** — only
  run if generated client files are missing or stale/broken)
- `schema/` ORM → `just migrations "<message>"`
- Go module deps changed → run `go mod tidy` in each affected service directory
  first; only run `just generate-api-clients` if `go mod tidy` still fails or
  generated client files are missing (**expensive**)

Stop and surface failures to the user; do not push past a red pre-commit or
failing tests.

## Surface to the user at end of turn

Always report the pre-mutation op ID captured at the start (`jj op restore <id>`
to roll back).

Also mention when you:

- Auto-tracked a bookmark
- Auto-advanced a bookmark across multiple commits
- Used `--team` override
- Split a commit to exclude unrelated changes
- Rescued from detached-HEAD or stale working copy
- Skipped a path that would have required force-push or `--ignore-immutable`
- Stopped before rebasing upper PRs in a stack
- Resolved the Linear issue from a `.task-reference` marker (state which task
  and which Linear issue) instead of creating a new one

## IMPORTANT: AVOID

- Do not write or modify code as part of this skill.
- Do not run `jj rebase` to reorganize a stack (exception:
  [stacked-pr §3](stacked-pr.md#3-after-the-bottom-pr-merges)).
- Do not resolve conflicts — stop and ask the user.
- Do not call the Linear API directly; go through `just create-issue`.
- Do not auto-rebase upper PRs in a stack after updating one below.
- Do not restructure existing commits (squash/split/reorder) unless the user
  asks.
- Do not move a bookmark backward or to a non-fast-forward target.
