# Proactive Systems — Heartbeat + Daily Reflection

**Status:** Future (post-MVP). **Depends on:** MVP, `01-gmail-integration.md`,
`02-google-calendar-integration.md`, `03-discord-outbound.md`. (Heartbeat is the
first place the integrations get exercised together. Without at least Gmail it
has nothing to ingest; without Calendar it has no deadline signal; without
Discord-out it has no notification channel.)

## What to build

A scheduled heartbeat that gathers context, reasons with Claude, writes events
(raw inputs + proposed actions), and delivers advisories via Discord DM. The
heartbeat is the **sole writer of MEMORY.md**.

A daily reflection job (08:00 Israel time) that consolidates the day's session
summaries into MEMORY.md and a daily briefing.

## Files

```
src/sb/scripts/
├── heartbeat.py             # exposed as sb-heartbeat
└── memory_reflect.py        # exposed as sb-reflect
```

Schedule (deferred to `08-deployment-systemd.md` for systemd; cron line shown
here is the canonical reference):

```cron
# Active hours: 08:00–19:00 Israel time (UTC+3 summer, Sun–Thu)
*/30 5-16 * * 0-4  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-heartbeat
```

## This PRD adds new event types

| Event type          | Source          | When written                               |
| ------------------- | --------------- | ------------------------------------------ |
| `email_thread`      | gmail           | Heartbeat ingestion                        |
| `calendar_event`    | calendar        | Heartbeat ingestion                        |
| `action_proposed`   | heartbeat/agent | Agent generates a proposal                 |
| `action_responded`  | user            | Hagai accepts/rejects/modifies via Discord |
| `event_processed`   | agent           | After processing an input event            |
| `notification_sent` | heartbeat/bot   | When a Discord DM goes out                 |

## This PRD introduces the projection layer

The MVP shipped only the `events` table. This PRD is the first one with real
consumers for derived state, so it introduces the projection tables, the
`project_event()` dispatcher, agent identity, and replay — all in one place,
born with their first writers.

### Projection schema (added to `schema/`)

```sql
-- Agents that produce events / actions. Lazy-registered on first use.
CREATE TABLE agents (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT NOT NULL,
    harness       TEXT NOT NULL,
    model         TEXT NOT NULL,
    version       TEXT,
    prompt_hash   TEXT,
    first_seen    TIMESTAMP NOT NULL,
    last_seen     TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX agents_identity ON agents(harness, model, version, prompt_hash);

-- Actions proposed by agents (latest response wins; full history in events).
CREATE TABLE actions (
    id              TEXT PRIMARY KEY,
    type            TEXT NOT NULL,
    created_at      TIMESTAMP NOT NULL,
    agent_id        INTEGER NOT NULL REFERENCES agents(id),
    source_event_id INTEGER REFERENCES events(id),
    content         TEXT NOT NULL,
    status          TEXT,
    response_notes  TEXT,
    responded_at    TIMESTAMP
);

CREATE INDEX actions_pending  ON actions(created_at DESC) WHERE status IS NULL;
CREATE INDEX actions_by_agent ON actions(agent_id, created_at DESC);

-- Which agent processed which event.
CREATE TABLE event_processing (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id        INTEGER NOT NULL REFERENCES events(id),
    agent_id        INTEGER NOT NULL REFERENCES agents(id),
    started_at      TIMESTAMP NOT NULL,
    completed_at    TIMESTAMP,
    status          TEXT NOT NULL,
    output_refs     TEXT,
    error           TEXT
);

CREATE UNIQUE INDEX event_processing_uniq ON event_processing(event_id, agent_id);

-- Notifications sent.
CREATE TABLE notifications (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    channel         TEXT NOT NULL,
    sent_at         TIMESTAMP NOT NULL,
    related_id      TEXT,
    related_type    TEXT,
    message         TEXT NOT NULL,
    source_event_id INTEGER REFERENCES events(id)
);

CREATE INDEX notifications_related ON notifications(related_id, sent_at DESC);

CREATE TABLE projection_cursors (
    name           TEXT PRIMARY KEY,
    last_event_id  INTEGER NOT NULL DEFAULT 0,
    updated_at     TIMESTAMP NOT NULL
);

CREATE VIEW completed_actions AS
    SELECT * FROM actions WHERE status IS NOT NULL;
```

### `write_event` upgrade — insert + dispatch in one transaction

```python
def write_event(event: Event) -> int | None:
    with conn:
        row = conn.execute(
            "INSERT INTO events (...) VALUES (...) "
            "ON CONFLICT(source, source_id) DO NOTHING RETURNING id",
            (...),
        ).fetchone()
        if row is None:
            return None
        event_id = row[0]
        project_event(conn, event._replace(id=event_id))
        return event_id
```

`project_event(conn, event)` dispatches by `event.type`:

- `action_proposed` → `INSERT OR IGNORE INTO actions`
- `action_responded` → `UPDATE actions SET status, response_notes, responded_at`
- `event_processed` → `INSERT INTO event_processing`
- `notification_sent` → `INSERT INTO notifications`
- everything else → no-op

Future PRDs extend the dispatcher (e.g. `05-chat-interface.md` adds
`chat_message` → `chat_messages`).

### Rebuilding projections (replay)

```python
def rebuild_projections(conn):
    with conn:
        for table in ["actions", "event_processing", "notifications"]:
            conn.execute(f"DELETE FROM {table}")
        conn.execute("UPDATE projection_cursors SET last_event_id = 0")
        for event in conn.execute("SELECT * FROM events ORDER BY id"):
            project_event(conn, event)
```

Exposed via `sb-db --rebuild` (already documented in MVP Phase 9 as a planned
op; the implementation lands here).

### Agent identity (lazy registration)

At heartbeat startup the harness computes its
`(harness, model, version, prompt_hash)` tuple, runs
`INSERT OR IGNORE INTO
agents`, and resolves the resulting `id` for the run. New
prompt = new `prompt_hash` = new row. This is what makes
`event_processing(event_id, agent_id)` meaningful: bumping the heartbeat prompt
re-marks every prior input event as unprocessed for the new identity, and the
next run reprocesses with better judgment.

## Heartbeat — gather phase

```python
def gather_context() -> dict:
    # sync_human_vault() (defined in MVP Phase 4.5) does two things:
    # (a) fast-forwards the local submodule, (b) writes one
    # `human_vault_commit` event per new commit. Heartbeat now runs it on
    # cadence rather than relying on a manual `sb-sync-human-vault`.
    if sync_human_vault():
        # Reindex human-vault delta before reasoning, otherwise chat / RAG
        # reads a corpus older than the events the heartbeat is about to
        # ingest. mtime-incremental + delete reconciliation; cheap.
        subprocess.run(
            [
                sys.executable,
                "-m",
                "sb.scripts.memory_index",
                "--source",
                "human-vault",
            ],
            check=False,
        )
    return {
        "unread_emails": gmail.list_important_unread(max_results=20),
        "upcoming_deadlines": calendar.list_events_with_deadlines(),
        "events_today": calendar.list_upcoming_events(days_ahead=1),
        "active_risks": parse_memory_risks(),
        "active_projects": parse_memory_projects(),
        "human_vault_commits": fetch_unprocessed_human_vault_commits(),
    }


def fetch_unprocessed_human_vault_commits() -> list[dict]:
    """Commits not yet processed by *this* agent identity. Catches both
    fresh commits and stragglers from a previous run that crashed before
    writing `event_processed`."""
    rows = conn.execute(
        """
        SELECT e.id, e.source_id, e.occurred_at, e.payload
        FROM events e
        LEFT JOIN event_processing ep
          ON ep.event_id = e.id AND ep.agent_id = ?
        WHERE e.type = 'human_vault_commit' AND ep.id IS NULL
        ORDER BY e.id
    """,
        (current_agent_id,),
    ).fetchall()
    out = []
    for event_id, sha, occurred_at, payload_json in rows:
        payload = json.loads(payload_json)
        # Read content from disk via `git show <sha>:<path>` — gives the
        # exact post-commit version even if later commits in the batch
        # modified the file again. Vault is the source of truth; don't
        # duplicate diffs into the DB.
        contents = {}
        for f in payload["files"]:
            if f["status"].startswith("D"):
                continue
            try:
                contents[f["path"]] = subprocess.check_output(
                    [
                        "git",
                        "-C",
                        str(HUMAN_VAULT_PATH),
                        "show",
                        f"{sha}:{f['path']}",
                    ],
                    text=True,
                )
            except subprocess.CalledProcessError:
                contents[f["path"]] = "<unreadable>"
        out.append(
            {
                "event_id": event_id,
                "sha": sha,
                "subject": payload["subject"],
                "occurred_at": occurred_at,
                "files": payload["files"],
                "contents": contents,
            }
        )
    return out
```

## Ingest phase

```python
def ingest_events(context: dict):
    for email in context["unread_emails"]:
        write_event(
            Event(
                source="gmail",
                type="email_thread",
                source_id=email.source_id,  # idempotent on re-ingest
                occurred_at=email.received_at,
                payload=asdict(email),
            )
        )
    for ev in context["upcoming_deadlines"]:
        write_event(
            Event(
                source="calendar",
                type="calendar_event",
                source_id=ev.id,
                occurred_at=ev.updated,
                payload=asdict(ev),
            )
        )
```

## Reasoning phase (Claude SDK)

```python
from anthropic import Anthropic

client = Anthropic()


def reason_over_context(context: dict) -> str:
    prompt = f"""
You are Hagai's engineering second brain operating in Advisor mode.
Hagai is a Staff Software Engineer / Software Architect in Israel.

Analyze this snapshot and produce a brief advisory (max 300 words):
1. [P0/P1/P2] risks needing immediate attention
2. Deadlines in the next 48 hours
3. Emails that need a reply action (flag subject + sender)
4. Cross-team blockers to surface
5. Human-vault changes worth reacting to (new/modified notes — update
   dossiers, link AI-vault analysis, switch to Workflow B for any meeting
   Hagai already wrote up)
6. One suggested action for the most critical item

Context:
{json.dumps(context, indent=2, default=str)}

Rules:
- NEVER suggest sending anything — only proposing actions for review
- Be terse. One line per item. Use bullet points.
- If nothing needs attention, say "All clear."
"""
    response = client.messages.create(
        model="claude-sonnet-4-6",
        max_tokens=500,
        system=open(Path.home() / "second-brain/work/ai-vault/SOUL.md").read(),
        messages=[{"role": "user", "content": prompt}],
    )
    return response.content[0].text
```

## Human-vault commit processing (same cycle)

After reasoning, close the loop on each consumed commit. Without this, the next
heartbeat would re-surface the same commits forever.

```python
def mark_human_vault_commits_processed(commits: list[dict], output_refs: dict):
    for c in commits:
        conn.execute(
            "INSERT OR IGNORE INTO event_processing "
            "(event_id, agent_id, started_at, completed_at, status, output_refs) "
            "VALUES (?, ?, ?, ?, 'success', ?)",
            (
                c["event_id"],
                current_agent_id,
                c["occurred_at"],
                now_utc(),
                json.dumps(output_refs.get(c["sha"], {})),
            ),
        )
```

`output_refs` captures what the agent did in response — e.g.
`{"<sha>": {"updated_dossier": "dossiers/people/alice.md", ...}}`.

**Reprocessing with a smarter agent.** Because `event_processing` is keyed by
`(event_id, agent_id)`, a new agent identity (different `prompt_hash`) sees
every commit as unprocessed and re-reacts with its better judgment. Old agent's
outputs stay as immutable history.

## Action generation

When the heartbeat flags emails needing a reply:

1. Fetch full thread via `gmail.get_thread(thread_id)`.
2. Search human vault: `sb-search "<topic>" --source human-vault` for similar
   past context.
3. Query
   `completed_actions WHERE type = 'email_draft' ORDER BY responded_at DESC
   LIMIT 20`
   — last N proposals + responses become learning context.
4. Generate action content via Claude.
5. `write_event(Event(type="action_proposed", payload={action_id, type,
   content, agent_id}, source_event_id=email_event_id))`.
6. Send Discord DM, then `write_event(Event(type="notification_sent", ...))`.

## AI-vault writes (heartbeat is the primary writer of dossiers/briefings)

Beyond MEMORY.md, the heartbeat freely writes to `ai-vault/`:

- `dossiers/people/<name>.md` — append/update rolling synthesis when a person
  appears in new comms.
- `dossiers/projects/<slug>.md` — update project state.
- `briefings/meetings/<YYYY-MM-DD>-<slug>.md` — pre-meeting briefings for
  tomorrow's calendar.
- `analysis/<topic>.md` — speculative themes worth re-reading.
- `logs/<YYYY-MM-DD>-<run>.md` — reasoning trace.
- `questions/open.md` — clarifications to surface to Hagai later.

Use `[[wikilinks]]` for navigability in Obsidian.

## MEMORY.md update (sole writer)

After processing events:

- Add new risks, projects, decisions surfaced this run.
- Apply eviction if any section exceeds its cap: priority-desc then
  recency-desc.
- Enforce `max_bytes.MEMORY` from config.toml.

## Notification dedupe

The `notifications` projection table replaces md5-hash diffing. Each action
triggers at most one Discord DM:

```python
def notify_about_action(action_id: str, message: str):
    if conn.execute(
        "SELECT 1 FROM notifications WHERE related_id = ? AND related_type = 'action' LIMIT 1",
        (action_id,),
    ).fetchone():
        return
    discord_client.notify(message)
    write_event(
        Event(
            source="heartbeat",
            type="notification_sent",
            payload={
                "channel": "discord_dm",
                "related_id": action_id,
                "related_type": "action",
                "message": message,
            },
        )
    )
```

## `memory_reflect.py` — daily at 08:00 Israel

```cron
0 5 * * *  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-reflect
```

Process:

1. `SELECT * FROM events WHERE type = 'session_summary' AND occurred_at >= date('now', '-1 day')`.
2. Ask Claude: "Extract decisions, risks, and key facts worth remembering
   long-term."
3. Update MEMORY.md (respecting caps + eviction).
4. Write `ai-vault/briefings/daily/<YYYY-MM-DD>.md` (yesterday's themes, today's
   open items, [[wikilinks]] to relevant dossiers).
5. Refresh dossiers for any people/projects surfaced today.
6. For durable knowledge worth preserving: emit a `promote-to-human-vault`
   action via `write_event(action_proposed)` (search/dedup/reformulate
   discipline from MVP Phase 4.5 applies).

## Advisor-mode behaviors

- Heartbeat proposes actions for review — never acts.
- All Discord notifications are advisory.
- No autonomous sending, posting, or purchasing.

## Estimated cost

~$0.05/heartbeat × 22 runs/day × 22 workdays/month ≈ **$24/month**.

## Acceptance — "done"

- `sb-heartbeat` runs end-to-end against real Gmail/Calendar/Discord.
- One Discord DM per actionable item; never duplicate.
- MEMORY.md updates respect caps.
- `event_processing` rows close out every consumed `human_vault_commit`.
- `sb-reflect` writes a daily briefing and updates MEMORY.md.

**Dependencies:** All MVP phases + Gmail/Calendar/Discord-out integrations.
**Estimated complexity:** High.
