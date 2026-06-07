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
- [rebase.md](rebase.md) → Rebase PRs and/or local commits
- [sync.md](sync.md) → Fetch latest trunk and rebase all PRs and commits onto it

## GitHub account

Several gh accounts are authed; the right one is fixed by the repo's remote
owner, not by which clone you're in. `GH_TOKEN` overrides the active account
per-command without a global `gh auth switch`. Resolve the owner from origin,
then prefix **every** `gh` command with the matching token.

```
owner=$(jj git remote list | awk '$1=="origin"{print $2}' \
  | sed -E 's#\.git$##; s#^.*[:/]([^/]+)/[^/]+$#\1#')
```

| owner         | gh account                |
| ------------- | ------------------------- |
| hakasecurity  | `hxhagai`                 |
| cato-networks | `hagai-akibayov_catonian` |

Then: `GH_TOKEN=$(gh auth token --user {account}) gh pr ...`. Unknown owner →
stop and ask. (Scopes `gh` only; jj/git push auth — SSH or https credential
helper — is separate.)

## Conventions

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
