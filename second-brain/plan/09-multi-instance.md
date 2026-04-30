Multi-Instance (`personal` alongside `work`)
============================================

**Status:** Future (post-MVP). **Depends on:** MVP. The MVP already pays the
design tax to support multiple instances (`find_instance()`, per-instance state
under `~/.local/share/second-brain/<instance>/`, instance-marker =
`config.toml`). This PRD is a small acceptance test that the design holds up
when a second instance is actually bootstrapped — and a place to track
per-instance divergence as it shows up.

What to build
-------------

Bootstrap a `personal` instance against the same `~/dotfiles/second-brain/`
codebase, with its own AI vault, MEMORY/SOUL/USER, cloud DB, and (if Hagai
chooses) human-vault. Verify all CLIs resolve the right instance from cwd.

Bootstrap
---------

```bash
uv run --project ~/dotfiles/second-brain \
       python ~/dotfiles/second-brain/bootstrap.py personal
```

This creates `~/second-brain/personal/` (separate git repo) and
`~/.local/share/second-brain/personal/` (separate replica + RAG DB). Bootstrap
prompts re-run; sensible defaults differ:

- **SOUL.md tone**: less Staff-Engineer-formal, more personal/life context.
- **USER.md**: personal Gmail address, personal Discord ID, no team map.
- **DB**: a separate Turso DB (or local-sqlite file) — `personal` and `work`
  must not share `DATABASE_URL`. Bootstrap warns if the supplied URL matches an
  existing instance's URL.
- **Human vault**: optional. If Hagai keeps a `~/notes/personal/` repo, point
  Phase 4.5 at it; otherwise skip the GitLab section.

Verification checklist
----------------------

After bootstrap:

```bash
cd ~/second-brain/personal && sb-db --status      # personal DB, not work
cd ~/second-brain/work     && sb-db --status      # work DB
cd ~/second-brain/personal && sb-search "..."     # only personal vault chunks
cd ~/second-brain/work     && sb-search "..."     # only work vault chunks
```

Hooks: launching Claude Code in `~/second-brain/personal/` injects the personal
SOUL/USER/MEMORY; launching it in `~/second-brain/work/` injects the work
versions. The same `.claude/hooks` symlink resolves both because the hooks call
`find_instance(cwd)` per fire.

Per-instance state surfaces
---------------------------

| Surface                       | Per-instance? | Path                                                      |
| ----------------------------- | ------------- | --------------------------------------------------------- |
| AI vault                      | yes           | `~/second-brain/<instance>/ai-vault/`                     |
| Cloud DB / replica            | yes           | `~/.local/share/second-brain/<instance>/cloud-replica.db` |
| RAG DB                        | yes           | `~/.local/share/second-brain/<instance>/rag.db`           |
| OAuth credentials             | yes           | `~/.local/share/second-brain/<instance>/credentials.json` |
| OAuth token                   | yes           | `~/.local/share/second-brain/<instance>/token.json`       |
| `secrets.toml`                | yes           | `~/.local/share/second-brain/<instance>/secrets.toml`     |
| Logs                          | yes           | `~/.local/log/second-brain/<instance>/*.log`              |
| Code (package, hooks, skills) | shared        | `~/dotfiles/second-brain/`                                |
| Schema                        | shared        | `~/dotfiles/second-brain/schema/0001_init.sql`            |

Cron lines duplicate per instance (one set per instance dir):

```cron
*/15 * * * *  cd ~/second-brain/work     && uv run --project ~/dotfiles/second-brain sb-index ...
*/15 * * * *  cd ~/second-brain/personal && uv run --project ~/dotfiles/second-brain sb-index ...
```

Same for heartbeat/reflect — different schedules per instance are fine (personal
might not run during work hours).

systemd chat unit: one unit per instance, parametric:

```ini
# ~/.config/systemd/user/second-brain-chat@.service
[Service]
WorkingDirectory=%h/second-brain/%i
Environment=SECOND_BRAIN_INSTANCE=%h/second-brain/%i
ExecStart=uv run --project %h/dotfiles/second-brain sb-chat
EnvironmentFile=%h/second-brain/%i/.env
Restart=always
```

```bash
systemctl --user enable --now second-brain-chat@work
systemctl --user enable --now second-brain-chat@personal
```

Acceptance — "done"
-------------------

- Bootstrap succeeds for `personal` without touching the `work` instance.
- Each CLI resolves the instance from cwd correctly; no cross-talk.
- Search results, MEMORY.md edits, and event writes go to the correct DB.
- Where `work` had a feature, `personal` either has it independently or the
  bootstrap section was deliberately skipped (and that skip persists across
  re-runs).

**Dependencies:** MVP. Helpful but not required: the integration / heartbeat
PRDs, since multi-instance value compounds when proactive systems are running.
**Estimated complexity:** Low (mostly verifying the existing design works).
