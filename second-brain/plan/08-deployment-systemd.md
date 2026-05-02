# Deployment — cron + systemd User Services

**Status:** Future (post-MVP). **Depends on:** Whichever of
`04-heartbeat-and-reflection.md` / `05-chat-interface.md` you've shipped — those
are the things this PRD schedules. The MVP runs everything on demand; this PRD
wires it to keep running unattended.

## What to build

Wire the recurring jobs (memory indexer, heartbeat, daily reflection,
human-vault sync) to cron — and the long-running chat bot to a systemd user
service. End-to-end smoke after wiring.

## Cron jobs

Each line `cd`s into the instance dir; `find_instance()` then resolves from cwd.
Logs are per-instance.

All scheduled scripts run via console scripts under
`uv run --project ~/dotfiles/second-brain ...`. uv resolves the project venv
without requiring activation (cron does not inherit one), and console scripts
(`sb-index`, `sb-heartbeat`, `sb-reflect`) are declared in the package
`pyproject.toml`.

```cron
# Memory indexer — every 15 minutes
*/15 * * * *  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-index >> ~/.local/log/second-brain/work/index.log 2>&1

# Heartbeat — every 30 min, Israeli working hours (Sun–Thu, 08:00–19:00 local = 05:00–16:00 UTC summer)
*/30 5-16 * * 0-4  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-heartbeat >> ~/.local/log/second-brain/work/heartbeat.log 2>&1

# Daily reflection — 08:00 Israel time (05:00 UTC summer)
0 5 * * *  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-reflect >> ~/.local/log/second-brain/work/reflect.log 2>&1
```

**Israel work week:** Sunday–Thursday. Cron day-of-week `0-4` (Sun=0, Thu=4).

## systemd user service for the chat bot

```ini
# ~/.config/systemd/user/second-brain-chat.service
[Service]
WorkingDirectory=%h/second-brain/work
Environment=SECOND_BRAIN_INSTANCE=%h/second-brain/work
ExecStart=uv run --project %h/dotfiles/second-brain sb-chat
EnvironmentFile=%h/second-brain/work/.env
Restart=always
```

```bash
mkdir -p ~/.config/systemd/user
systemctl --user daemon-reload
systemctl --user enable --now second-brain-chat
systemctl --user status second-brain-chat
```

## Python environment setup

The project lives at `~/dotfiles/second-brain/` with a uv-managed venv at
`~/dotfiles/second-brain/.venv`. All runtime invocations (hooks, cron, systemd)
use `uv run --project ~/dotfiles/second-brain ...` so no activation is needed.

```bash
# One-time: sync the venv from pyproject.toml.
uv sync --project ~/dotfiles/second-brain
```

Add `.venv/` to `.gitignore`. Adding a new dep is one line in `pyproject.toml`
followed by `uv sync`.

## OAuth first-run

```bash
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain python -m sb.scripts.integrations.shared_auth
# Token saved; auto-refreshes after that.
```

## Log rotation

```
# /etc/logrotate.d/second-brain-user (or run logrotate as user)
~/.local/log/second-brain/*/*.log {
    daily
    rotate 7
    compress
    missingok
}
```

## Operational commands

```bash
# Snapshot the cloud DB to a portable file (.sql text or .db file)
sb-db --dump > ~/backups/cloud-$(date +%F).sql

# Wipe and rebuild projections from events (after schema change or new agent)
sb-db --rebuild

# Show current state: events, agents, cursors, sync health.
sb-db --status
```

`--status` sync-health panel — when something feels stale ("did the agent see my
note from this morning?"), one CLI answers it. Sections:

- **Events:** `MAX(ingested_at)` per `source` — last time
  gmail/calendar/heartbeat/manual wrote anything.
- **Human vault:** most recent `human_vault_synced` event — `head_after` SHA,
  `advanced` flag, age. Older than ~1 heartbeat cycle ⇒ submodule fetch is
  failing silently.
- **RAG index:** `MAX(indexed_at)` per `source` in `chunks` + `COUNT(*)` chunks
  per source. Frozen `indexed_at` ⇒ indexer cron stopped firing.
- **Replica:** libSQL embedded-replica last-sync timestamp. Stale-replica age
  tells you if writes are backed up.
- **Projections:** `projection_cursors.last_event_id` vs `MAX(events.id)` —
  drift means the projector died mid-replay.
- **Session hooks:** most recent `session_summary` event timestamp, split by
  `source = 'session_compact' | 'session_end'`. Soft-fail in the hooks means a
  broken DB write surfaces only as silently-missing events; this panel is the
  tripwire. If no `session_summary` for several days despite active Claude
  sessions, check `~/.local/log/second-brain/<instance>/hooks.log`.

Implementation: plain SQL + one `os.stat` on the replica file. No new state.

## Swap-away from Turso

The embedded replica is the built-in escape hatch:

```bash
# 1. Capture a final snapshot
sb-db --dump > ~/backups/cloud-final.sql

# 2. Promote the embedded replica to be the source of truth
cp ~/.local/share/second-brain/work/cloud-replica.db \
   ~/.local/share/second-brain/work/cloud.db

# 3. Update DATABASE_URL in .env from libsql://... to file:///...
# Same db.py works in both modes — no code changes.
```

## Cost estimate (with everything wired up)

| Component                                         | Monthly Cost      |
| ------------------------------------------------- | ----------------- |
| Claude API (Sonnet 4.6, ~22 heartbeats/day)       | ~$24              |
| Claude API (chat interface, varies)               | ~$5–15            |
| Turso (free tier sufficient at single-user scale) | $0                |
| Obsidian                                          | Free              |
| Infrastructure                                    | $0 (local)        |
| **Total**                                         | **~$29–39/month** |

## Acceptance — "done"

- `crontab -l` shows the three lines and they fire on schedule.
- `systemctl --user status second-brain-chat` is green.
- A 24-hour soak shows the indexer ticked every 15 min, heartbeat ticked during
  work hours, reflection ran once.
- `sb-db --status` returns an all-green panel.
- Logs rotate (verify after 8 days).

**Dependencies:** All MVP phases + heartbeat + (optionally) chat. **Estimated
complexity:** Low (mostly config).
