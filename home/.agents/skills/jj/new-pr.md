# New PR

Edits exist locally, no PR yet.

## 1. Check working-copy state

The edits to publish are already on `@` or its ancestors above the base.

- Edits sit cleanly above `trunk()` (or the intended base) → §2.
- `@` has unrelated dirty changes mixed with the intended edits → split via
  [recovery](recovery.md) before continuing. Push only the intended changes.
- `@` is detached or in an unexpected place → [recovery](recovery.md) first.

## 2. Resolve the Linear issue

Pick the first branch that applies:

1. **User supplied an issue key** (e.g. `HS-10349`) → use it. Skip the rest of
   this section.
2. **`.task-reference` exists at repo root** → read the task file path from it.
   The path looks like `$PLAN_DIR/<slug>/tasks/<task-basename>.md`; extract the
   `<slug>` segment. Then read `$PLAN_DIR/.linear.json` and look up
   `slugs["<slug>"].tasks["<task-basename>.md"]` — that's the sub-issue key. If
   `$PLAN_DIR/.linear.json` is missing, the slug entry is missing, or the task
   key is missing, stop and ask the user — the user likely needs to run the
   `linear-sync` skill first.
3. **Otherwise** → create a new issue:
   `just create-issue --title "<title>" --non-interactive --skip-branch --team "<team>"`.
   Always pass `--skip-branch`. Pick `<team>` per §2.1. Re-state the chosen team
   and the one-line reason to the user before running.

   On `Default team 'X' not found` or `Team 'X' not found`, retry once with
   `--team "R&D"` and tell the user team selection failed. Ask the user to
   confirm the team only if both attempts fail.

### 2.1 Team routing

Pick the team that owns the **code being changed**, not the team of the person
reporting the bug or asking for the feature. Match against the top-level
directories first; fall through to the topic rules; default to `R&D` only if
nothing else fits.

| Team (issue prefix)              | Owns                                                                                                                                              | Typical changes                                                                                                                 |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **Aim Engine** (`ENG-`)          | `engine/`, `classifiers_monitoring/`                                                                                                              | Detectors, classifiers, PII/secrets/PHI/TSI logic, model serving, regex/perf, detection metrics                                 |
| **AI Security for Apps** (`FW-`) | `firewall/`, `ai_security_graphql/`, Outpost, Guards/FirewallKey                                                                                  | Firewall gateway, GraphQL, Outpost deployment, Guard/Key migrations, CRDs, MFE↔FE alignment                                     |
| **Aim for End-Users** (`AEU-`)   | `end-users/`, `interceptors/`, `event_logs_consumers/`, `web/browser_extension_workspace/`                                                        | Browser extension, MITM proxy, Zscaler/Cato NSS consumers, extension deployment, reverse-proxy routing                          |
| **AgenTeam** (`AGNT-`)           | `endpoints/ai-scout/`, `discovery/api_server/api_server/apps/ai_scout_be`, agent tracing                                                          | AI Scout (CLI + backend), Scout hooks, agent sessions/traces, agent UI views                                                    |
| **R&D** (`HS-`)                  | `libraries/`, `schema/`, `protos/`, `openapi/`, `deployment/`, `justfiles/`, `.github/`, monorepo root, generated-client tooling, repo-wide CI/CD | Shared infra, CI/CD, monorepo tooling, deploys, cross-cutting refactors, anything that doesn't clearly belong to a product team |

**Tie-breakers:**

- A change touches multiple product directories → `R&D` if it's shared infra
  (`libraries/`, `schema/`, protos, CI). Otherwise pick the team whose feature
  is the _driver_ of the change; the others are ride-alongs.
- A change in `discovery/api_server/` (the monolith) → route by the app under
  `apps/` it touches. `apps/ai_scout_be` → AgenTeam; webapp BFF changes for a
  User-Protection feature → Aim for End-Users; etc. Pure monolith plumbing
  (auth, dependencies, shared routers) → R&D.
- A change in `web/webapp_workspace/webapp/` → route by the product surface the
  page/feature belongs to. A Scout page → AgenTeam; a Guard policy page → AI
  Security for Apps. Webapp-wide chrome → R&D.
- Bug fix → team that owns the code path, even if the bug was reported by
  another team.
- Genuine ambiguity → R&D, and surface the call to the user.

## 3. Publish

1. If `.task-reference` exists at repo root, `rm .task-reference` now — before
   any describe/snapshot — so the marker doesn't land in the published commit.
2. Ensure every commit in the PR has a meaningful message
   ([naming](SKILL.md#naming-and-writing)). `jj describe` any that don't.
3. Create bookmark `<issue-key-lower>-<kebab-title>` at the tip
   ([naming](SKILL.md#naming-and-writing)).
4. `jj git push --bookmark <name>`. On `Non-tracking remote bookmark
   exists`,
   run `jj bookmark track <name>@origin` and retry.
5. `gh pr create --head <bookmark-name> --base "$TRUNK"` with title and body per
   [naming](SKILL.md#naming-and-writing). `$TRUNK` = repo default branch
   ([trunk](SKILL.md#trunk)); or omit `--base` to let GitHub default to it.
6. Output the PR URL.
