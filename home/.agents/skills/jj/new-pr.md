# New PR

Create new PR from local edits/commits.

## 1. Prepare working-copy state

The edits to publish should already be on `@` and/or its ancestors up to the
base.

## 2. Resolve the Linear issue

The new PR must be tied to a Linear issue. Identify the issue ID:

1. **User supplied an issue ID** (e.g. `HS-10349`) → use it.
2. **`.task-reference` exists at repo root** → resolve the Linear issue ID by
   running
   `~/dotfiles/home/.agents/skills/jj/scripts/resolve-task-reference.sh`. If
   `.task-reference` exists but resolving fails, stop and ask the user to
   `/linear-sync` first.
3. **Otherwise** → create a new Linear issue:
   `~/dotfiles/home/.agents/skills/jj/scripts/create-issue.sh "{title}" "{team}"`.
   It prints the new issue identifier (e.g. `HS-1234`). Pick `{team}` per
   [Team routing](#team-routing).

### Team routing

Pick the team that owns the **code being changed**, not the team of the person
reporting the bug or asking for the feature. Match against the top-level
directories first; fall through to the topic rules; default to `R&D` only if
nothing else fits.

The **team name** below (leftmost column, minus the prefix) is what
`create-issue.sh "{title}" "{name}"` expects. It must match a Linear team name
exactly as it appears in your workspace (the script matches `viewer.teams`).
Only teams you belong to are listed; code owned by other teams (end-users /
browser extension, AI Scout, AI Security for Apps / Guard policy UI) defaults to
`R&D`, since you cannot file to a team you are not a member of.

| Team (issue prefix)              | Owns                                                                                                                                              | Typical changes                                                                                                                  |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Aim Engine** (`ENG-`)          | `engine/`, `classifiers_monitoring/`                                                                                                              | Detectors, classifiers, PII/secrets/PHI/TSI logic, model serving, regex/perf, detection metrics                                  |
| **AI Security Platform** (`FW-`) | `firewall/`, `ai_security_graphql/`, Outpost, Guards/FirewallKey, config-sync                                                                     | Firewall gateway, GraphQL, Outpost deployment, Guard/Key migrations, CRDs, MFE↔FE alignment                                      |
| **R&D** (`HS-`)                  | `libraries/`, `schema/`, `protos/`, `openapi/`, `deployment/`, `justfiles/`, `.github/`, monorepo root, generated-client tooling, repo-wide CI/CD | Shared infra, CI/CD, monorepo tooling, deploys, cross-cutting refactors, and any product area owned by a team you cannot file to |

**Tie-breakers:**

- A change touches multiple product directories → `R&D` if it's shared infra
  (`libraries/`, `schema/`, protos, CI). Otherwise pick the team whose feature
  is the _driver_ of the change; the others are ride-alongs.
- A change in `discovery/api_server/` (the monolith) → route by the app under
  `apps/` it touches. A Guard/firewall app → `AI Security Platform`; a detection
  app → `Aim Engine`; anything owned by a team you cannot file to (e.g.
  `apps/ai_scout_be`) or pure monolith plumbing (auth, dependencies, shared
  routers) → `R&D`.
- A change in `web/webapp_workspace/webapp/` → route by the product surface the
  page/feature belongs to. Webapp-wide chrome, or a page owned by a team you
  cannot file to (e.g. a Scout page → AgenTeam, a Guard policy page → AI
  Security for Apps) → `R&D`.
- Bug fix → team that owns the code path, even if the bug was reported by
  another team.
- Genuine ambiguity → R&D, and surface the call to the user.

On `team 'X' not found` (exit 2), retry once with `R&D` as the team argument and
tell the user team selection failed. Ask the user to confirm the team only if
both attempts fail.

## 3. Resolve the base

If the relevant local changes/commits sit cleanly on top of `trunk()`, then the
base is `trunk()`. If they sit on top of another PR bookmark, the base is the
bookmarked PR, which will make the new PR a **stacked PR**. Follow
[Stacked PR Guidelines](stacked-pr-guidelines.md).

## 4. Publish

1. If `.task-reference` exists at repo root, `rm .task-reference` now (before
   any describe/snapshot, so the marker doesn't land in the published commit).
2. Ensure every commit in the PR has a meaningful description
   ([Commit description](SKILL.md#commit-description)). `jj describe -m "..."`
   any that don't.
3. Create new bookmark `{linear-issue-id-lowercase}-{kebab-title}`. ≤80 chars;
   trim the title, never the key. Example:
   `eng-10333-move-unique-count-gauge-emission-to-a-cron`.
4. `jj git push --bookmark {bookmark}`. On "Non-tracking remote bookmark
   exists", run `jj bookmark track {bookmark}@origin` and retry.
5. `gh pr create --head {bookmark} --title "..." --body "..."` with title and
   body per [Conventions](SKILL.md#conventions).
   - For a new stacked PR:
     - Use `gh pr create ... --base {base-bookmark}` to stack on the parent PR.
     - Include the `### PR Stack` section per
       [Stacked PR Guidelines](stacked-pr-guidelines.md).

## 5. Output

- The PR URL.
- Single sentence summary about the created/resolved Linear issue.
