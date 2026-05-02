# Hagai's Second Brain — MVP PRD

**Status:** MVP (vertical slice). **Goal:** ship a working end-to-end second
brain that captures events, persists session memory, searches both vaults by
meaning + keyword, and proposes durable knowledge into a real human-vault GitLab
repo via MR. Everything else (automated integrations, proactive heartbeat, chat
surface, full security hardening, systemd deployment) is deferred to focused
follow-up PRDs in this directory.

**Scope of this MVP:**

- Phase 1 — Foundation (memory layer + cloud DB + event log)
- Phase 2 — Hooks (context persistence)
- Phase 3 — Memory Search (hybrid RAG over both vaults)
- Phase 4 (slim) — **Manual event entry** (`sb-event add` CLI). No Gmail /
  Calendar / Discord auto-collectors yet — those land in their own PRDs.
- Phase 4.5 — Human Vault Integration (the one real integration: GitLab MR)
- Phase 5 (slim) — **Two skills only**: `vault-structure` + the
  `promote-to-human-vault` skill (the latter lives with 4.5)
- Phase 9 (slim) — `sb-scheduler` daemon + manual operational surface.

**Deferred guardrail.** The original plan included a slim Phase 8 with one
PreToolUse hook that regex-blocked `git push origin main` from inside
`human-vault/`. PreToolUse is dropped from the MVP entirely (it sits on the hot
path of every tool call, and the MVP has no other consumer that would amortize
the `uv run` startup cost). The "don't push to main" restriction is held by the
soft guardrail in SOUL.md plus GitLab branch-protection on `main` — both of
which are sufficient when the only push surface is `glab` invoked from
`promote.py`. Hard PreToolUse-level enforcement returns in
`07-security-hardening.md`.

**Architecture principle:** A cloud SQL DB (Turso/libSQL) is the single source
of truth, with an append-only `events` table at its core. The MVP ships **only**
the `events` table; later PRDs add derived projection tables (`actions`,
`event_processing`, `notifications`, `chat_messages`, …) alongside the writers
that produce them, so each table is born with a real consumer. Projections are
rebuildable by replaying the event log. The DB is accessed via libSQL embedded
replica — reads come from a local SQLite file (sub-ms), writes go to the cloud
primary and sync back; every machine running the agent gets a continuous cold
backup as a side effect. The substrate is swappable: plain SQL only, one `db.py`
access layer, `[db]` block in `secrets.toml`. The system stays reprocessable
with better agents as they are released.

---

## Phase 1: Foundation (Memory Layer + Cloud DB)

**What to build:** The `bootstrap.py` scaffolder, AI-vault skeleton, core memory
files (SOUL/USER/MEMORY), cloud DB (Turso/libSQL with embedded replica), and
configuration (`config.toml`, `secrets.toml`). Every subsequent phase depends on
these.

### Project layout

The system splits **code** (lives in dotfiles, single source of truth across all
brain instances) from **content** (per-instance repos under `~/second-brain/`).
Today there is one instance — `work`; a `personal` instance is planned (see
`09-multi-instance.md`), bootstrapped against the same dotfiles tree.

**Code repo — `~/dotfiles/second-brain/`** (a subdirectory of Hagai's existing
dotfiles repo):

```
~/dotfiles/second-brain/
├── bootstrap.py                     # scaffolds a new ~/second-brain/<instance>/
├── pyproject.toml                   # declares the `sb` package + console scripts
│                                    #   (sb-index, sb-search, sb-db, sb-rag-db,
│                                    #    sb-event, sb-context-check, sb-scheduler)
├── src/sb/                          # ordinary importable Python package
│   ├── instance.py                  #   find_instance() — marker-file ascent
│   └── scripts/                     #   db.py, memory_index.py, event_cli.py,
│                                    #   git_io.py (commit_ai_vault_if_dirty),
│                                    #   scheduler.py, ...
├── claude/
│   ├── hooks/                       # → <instance>/.claude/hooks/
│   └── skills/                      # → <instance>/.claude/skills/
├── schema/
│   └── 0001_init.sql                # → <instance>/schema.sql
└── starter/                         # COPIED at bootstrap; diverges per instance
    ├── ai-vault/
    │   ├── SOUL.md, USER.md, MEMORY.md
    │   └── .obsidian/
    ├── config.toml                  # → <instance>/config.toml
    ├── claude-settings.json         # → <instance>/.claude/settings.json
    ├── secrets.example.toml         # → <instance>/secrets.toml (gitignored)
    └── gitignore                    # → <instance>/.gitignore
```

**Content instance — `~/second-brain/work/`** (a standalone git repo):

```
~/second-brain/work/
├── ai-vault/                        # COPIED from starter/ai-vault/
│   ├── SOUL.md, USER.md, MEMORY.md
│   ├── .obsidian/
│   └── briefings/, dossiers/, meetings/, drafts/, analysis/, logs/, questions/
├── config.toml                      # COPIED; also serves as instance marker; committed
├── secrets.toml                     # COPIED + edited; gitignored
├── human-vault/                     # git submodule → AI's GitLab clone (Phase 4.5)
├── .claude/
│   ├── hooks           → ~/dotfiles/second-brain/claude/hooks/
│   ├── skills          → ~/dotfiles/second-brain/claude/skills/
│   └── settings.json                # COPIED (per-instance permissions)
├── schema.sql          → ~/dotfiles/second-brain/schema/0001_init.sql
└── .gitignore                       # COPIED
```

#### `find_instance()` — instance discovery

A single helper at `src/sb/instance.py` is the only mechanism by which code
locates the instance directory:

```python
# src/sb/instance.py
import os
from pathlib import Path

INSTANCE_MARKER = "config.toml"


def find_instance(start: Path | None = None) -> Path:
    """Locate the second-brain instance directory.

    Resolution order:
      1. `SECOND_BRAIN_INSTANCE` env var, if set.
      2. Marker-file ascent from `start` (default: `Path.cwd()`) — walk up
         until a directory containing `config.toml` is found.

    Raises `RuntimeError` if neither produces a hit.
    """
    if env := os.environ.get("SECOND_BRAIN_INSTANCE"):
        return Path(env).expanduser().resolve()
    current = (start or Path.cwd()).resolve()
    for parent in (current, *current.parents):
        if (parent / INSTANCE_MARKER).is_file():
            return parent
    raise RuntimeError(
        f"No second-brain instance found above {start or Path.cwd()} "
        f"(no `{INSTANCE_MARKER}` marker, and SECOND_BRAIN_INSTANCE is unset)"
    )
```

Claude Code hooks receive `cwd` in their JSON payload and pass it to
`find_instance`; ad-hoc CLI invocations rely on cwd-ascent.

**Resolution outside an instance dir.** When Claude is launched from a path that
isn't under an instance (e.g. Hagai opens it in `~/notes/work/` to edit a note),
cwd-ascent fails and the hooks soft-fail to a vanilla session with no memory
injection. To get memory injection from anywhere, set `SECOND_BRAIN_INSTANCE` in
the shell rc — e.g. for fish:

```fish
# ~/.config/fish/config.fish
set -gx SECOND_BRAIN_INSTANCE $HOME/second-brain/work
```

Bootstrap does **not** modify shell rc; this is a one-time manual step.

**Code/content split — rationale:**

- **Single source of truth for code.** Edit once and
  `uv run --project ~/dotfiles/second-brain` picks the change up immediately for
  every instance.
- **Audit trail per instance.** Each `~/second-brain/<instance>/` is its own git
  repo. Its history captures only AI-vault content writes.
- **Starter files diverge.** SOUL/USER/MEMORY/`config.toml`/settings are
  **copied** at bootstrap, not symlinked.
- **Instance is movable.** Symlinks are relative; the package is found via uv's
  `--project` flag rather than any in-instance path.

**Bootstrap:**

```bash
uv run --project ~/dotfiles/second-brain python ~/dotfiles/second-brain/bootstrap.py work
```

`bootstrap.py` does, idempotently:

1. `git init ~/second-brain/<instance>/` if absent.
2. Copy `starter/` into the instance.
3. Create relative symlinks (`.claude/hooks`, `.claude/skills`, `schema.sql`).
4. **Interactively prompt for required + optional fields**, then render
   `secrets.toml`, `USER.md`, and `config.toml` from their starter templates.
   See "Bootstrap prompts" below.
5. Run `sb-db --init` (cloud DB schema; idempotent) and `sb-rag-db --init`
   (local RAG DB schema). Bootstrap invokes both as Python subprocesses with
   `cwd=<instance>` and `SECOND_BRAIN_INSTANCE=<instance>` so `find_instance()`
   resolves correctly.
6. Install + enable the `sb-scheduler@<instance>` systemd-user unit (idempotent;
   see Phase 9).
7. `git -C ~/second-brain/<instance> commit -m "Initial scaffold"`.

#### Bootstrap prompts

The script reads each starter file, finds `{{placeholders}}`, prompts for each
one, and writes the rendered file to the instance.

- **Required vs. optional.** Required prompts loop until non-empty. Optional
  prompts accept empty input and leave a `# TODO` marker.
- **Defaults.** Where a sensible default exists (timezone, paths), the prompt
  shows it in `[brackets]` and accepts Enter to keep it.
- **Re-runnable.** If the instance exists, `bootstrap.py` parses existing
  `secrets.toml` / `USER.md` / `config.toml`, pre-fills prompts, and re-prompts
  only for unset / `# TODO` entries. `--reconfigure` forces re-prompting.
- **Phase-gated sections.** GitLab (4.5) prompts can be skipped — the script
  asks `Configure GitLab integration now? [y/N]` and writes only the relevant
  block if yes.
- **Secrets stay out of git.** `secrets.toml` is gitignored. `config.toml` is
  committed (initial commit by bootstrap; subsequent edits committed by Hagai).

| Group        | Prompt                                                                        | Required           | Target                                 |
| ------------ | ----------------------------------------------------------------------------- | ------------------ | -------------------------------------- |
| Profile      | Full name                                                                     | required           | `USER.md`                              |
| Profile      | Role (default: `Staff Software Engineer / Software Architect`)                | required (default) | `USER.md`                              |
| Profile      | Timezone (default: `Asia/Jerusalem`)                                          | required (default) | `USER.md`                              |
| Profile      | Working hours (default: `09:00–18:00 local`)                                  | required (default) | `USER.md`                              |
| DB           | DB mode: `turso` (embedded replica) or `local-sqlite`                         | required           | `secrets.toml [db]`                    |
| DB           | Replica path (default: `~/.local/share/second-brain/<inst>/cloud-replica.db`) | required (default) | `secrets.toml [db] replica_path`       |
| DB (turso)   | Sync URL (e.g. `libsql://my-db.turso.io`)                                     | required if turso  | `secrets.toml [db] sync_url`           |
| DB (turso)   | Auth token                                                                    | required if turso  | `secrets.toml [db] auth_token`         |
| Anthropic    | API key (used by `sb-index` for non-English translation)                      | required           | `secrets.toml [anthropic] api_key`     |
| GitLab (4.5) | Configure GitLab human-vault now? `[y/N]`                                     | optional           | gates the next three                   |
| GitLab       | `gitlab_host` (default: `gitlab.com`)                                         | required if y      | `config.toml [human_vault]`            |
| GitLab       | Repo (e.g. `donbayov/work-notes`)                                             | required if y      | `config.toml [human_vault]`, `USER.md` |
| GitLab       | Personal access token                                                         | required if y      | `secrets.toml [gitlab] token`          |

A `bootstrap.py --check <instance>` mode validates that no `# TODO` markers
remain and exits non-zero otherwise.

`ai-vault/` is a regular folder versioned in the content repo. `human-vault/` is
a git submodule pointing to the GitLab repo backing `~/notes/work` (see Phase
4.5).

### AI vault structure

```
~/second-brain/work/ai-vault/
├── .obsidian/
├── .gitignore
├── SOUL.md                   # Agent persona, hard rules, promotion discipline
├── USER.md                   # Hagai's profile, account IDs, team map
├── MEMORY.md                 # Live index: projects, risks, decisions
├── briefings/
│   ├── meetings/             # one .md per upcoming meeting
│   └── daily/                # rolling daily snapshots (used by future heartbeat PRD)
├── meetings/                 # AI-digested meeting transcripts (Workflow A)
├── dossiers/
│   ├── people/               # one .md per person
│   └── projects/             # one .md per project
├── drafts/                   # final-form candidates pending promotion (MR)
├── analysis/                 # speculative — risk hypotheses, themes
├── logs/                     # reasoning traces
└── questions/                # open clarifications queued for Hagai
```

The AI vault is an **Obsidian vault** — open it directly in Obsidian to inspect
what the agent is doing. Internal `[[wikilinks]]` between dossiers, meetings,
and analysis form a navigable graph. Hagai is a read-only observer; the AI is
sole writer.

**No vault map file.** The agent discovers human-vault structure on demand by
listing directories and reading files via the indexer/search. A precomputed
`human-vault-map.md` was considered and dropped — it would either need to be
hand-maintained (friction) or auto-generated by a heuristic (drift). On-demand
discovery is cheap enough at MVP scale.

### Editorial model

- **Human vault** (`~/notes/work/`, mirrored read-only at
  `~/second-brain/work/human-vault/`): canonical record of Hagai's knowledge.
  Curated by Hagai directly. AI proposes changes only via MR through the
  `promote-to-human-vault` skill (Phase 4.5).
- **AI vault** (`~/second-brain/work/ai-vault/`): the AI's working memory +
  scratchpad. AI writes freely.

Content moves AI → human only via MR with reformulation, never via direct copy.

### Meeting notes — both workflows supported

- **Workflow A (transcript-driven):** AI ingests a transcript and writes a
  digest at `ai-vault/meetings/YYYY-MM-DD-name.md`. If outcomes are durable, AI
  promotes to human vault.
- **Workflow B (Hagai-driven):** Hagai writes the note directly in
  `~/notes/work/`. AI reads it and may add an analysis note in
  `ai-vault/analysis/` linking back via wikilink. No promotion.

The AI defaults to A; switches to B if it detects Hagai already wrote a note for
the same meeting (via path/title match or memory_search).

### Promotion (AI vault → human vault)

Durable AI-vault content is promoted to a MR against the human vault.
**Reformulation is mandatory.** Before opening a MR the agent MUST:

1. Search the human vault: `memory_search.py --source human-vault "<topic>"`
2. Read top-level `human-vault/` directory listing to decide placement
3. Reformulate to match human-vault voice, density, naming conventions
4. If overlap with an existing note, propose a merge edit — not a new file
5. Include in MR description: trigger, AI-vault source path, placement
   rationale, dedup-check result

See the `promote-to-human-vault` skill in Phase 4.5.

### Audit trail

The instance repo's git history (scoped to `ai-vault/`) is the audit trail for
SOUL/USER/MEMORY edits and AI-vault writes. Raw event data (manual entries,
future integrations, session summaries) lives in the cloud DB — not in git.

**Commit policy — invariant.** Every bounded unit of agent work that may write
to `ai-vault/` MUST end by calling `commit_ai_vault_if_dirty(reason)`. In the
MVP that's the two flush hooks (`pre-compact-flush.py` and
`session-end-flush.py`). Future PRDs (heartbeat, event processor, chat) inherit
the same obligation. The helper lives at `src/sb/scripts/git_io.py`:

```python
def commit_ai_vault_if_dirty(reason: str) -> str | None:
    """Stage ai-vault/ paths in the instance repo and commit if dirty.

    Other paths (config.toml, secrets.toml, .claude/, schema.sql, the
    human-vault submodule pointer) are NOT staged — those are setup state, not
    agent output. Returns the new commit sha, or None if the tree was clean.
    """
    instance = find_instance()
    subprocess.run(["git", "-C", str(instance), "add", "ai-vault/"], check=True)
    diff = subprocess.run(
        ["git", "-C", str(instance), "diff", "--cached", "--quiet"], check=False
    )
    if diff.returncode == 0:
        return None
    subprocess.run(
        ["git", "-C", str(instance), "commit", "-m", reason], check=True
    )
    return subprocess.check_output(
        ["git", "-C", str(instance), "rev-parse", "HEAD"], text=True
    ).strip()
```

**Load-bearing detail.** The bootstrap step
`git config submodule.human-vault.ignore all` is what makes this safe: without
it, every `git add ai-vault/` followed by a commit would also pick up changes to
the `human-vault` submodule pointer whenever `sync_human_vault()` had
fast-forwarded it, accidentally pinning revisions in the parent repo.

**Setup files** (`config.toml`, `.claude/settings.json`, `schema.sql` symlink)
are committed only by `bootstrap.py`. If Hagai edits them by hand he commits
them by hand. The agent never stages anything outside `ai-vault/`.

### SOUL.md starter content

The block below is **illustrative**, not load-bearing. `bootstrap.py` copies it
verbatim, but every instance is expected to edit it to match its own scope. Hard
rules in particular should reflect the actual surfaces this brain touches —
don't carry rules about systems that aren't in scope.

```markdown
# Agent Soul

## Identity

I am Hagai's engineering second brain. I think like a Staff Engineer: I surface
risks early, flag blockers across teams, and prefer directness over
pleasantries.

## Hard Rules (Advisor Mode)

- NEVER send emails, post messages, or act on behalf of Hagai without explicit
  approval.
- NEVER post to social media.
- I propose actions. Present for review. Wait.

## Vault Boundaries

- AI vault (`~/second-brain/work/ai-vault/`): my working memory. I am sole
  writer. Hagai is read-only.
- Human vault (`~/notes/work/`, mirrored at `~/second-brain/work/human-vault/`):
  Hagai's curated knowledge. Read-only for me. Changes only via the
  `promote-to-human-vault` skill (MR).

## Vault Editorial Freedom (AI vault)

The AI vault is mine to write freely. Use the standard subfolders and add new
ones when a clear category emerges.

## Promotion Triggers (AI vault → human vault)

Promote only when one of these triggers fires (closed enum, must match the
`PromotionTrigger` Literal in `promote.py`):

1. **decision-crystallized** — a meaningful decision was made and reasoned
   about.
2. **risk-tracked** — a project risk worth recording surfaced.
3. **incident-lesson-learned** — surprising failure or near-miss worth
   remembering.
4. **user-request** — Hagai explicitly said "save this."

(`pattern-recurred` is deferred — it requires a cross-session "themes"
projection that the MVP doesn't have. Reintroduced in
`04-heartbeat-and-reflection.md`.)

Never promote: speculative analysis, draft reasoning, transient briefings,
operational logs.

## Promotion Discipline

Before opening an MR, I MUST:

1. Search the human vault (`memory_search.py --source human-vault`).
2. Read the **top-3 results**. If any covers the same fact → propose a merge
   edit on the existing file. If related but distinct → extend the relevant
   section. Otherwise → new file.
3. List `human-vault/` top-level directories to choose folder placement.
4. Reformulate to match human-vault voice, density, naming conventions, and link
   style.
5. Include in the MR description: trigger, AI-vault source path, placement
   rationale, dedup result.

I MUST NEVER:

- Push directly to `main` in human-vault.
- Force-push any branch.
- Modify `.git/config` or remotes.
- Promote AI-vault prose verbatim.

## Communication Style

- Concise. Technical substance only.
- Flag risks with severity: [P0 blocker / P1 risk / P2 watch].

## Actions

- I propose actions; Hagai accepts, rejects, or modifies them.
- Rejections ALWAYS carry a one-line note.

## MEMORY.md Rules

- I am the sole writer of MEMORY.md.
- Each section has a hard cap (see config.toml). When full, evict by
  priority-desc then recency-desc.
- Every entry is a one-liner only.
```

### USER.md starter content

```markdown
# {{full_name}}'s Profile

- **Role:** {{role}}
- **Timezone:** {{timezone}}
- **Working hours:** {{working_hours}}

## Accounts

- GitLab: {{gitlab_upstream_repo_or_TODO}} (host: {{gitlab_host_or_TODO}})

## Team Map

<!-- Populate manually -->
<!-- Format: Name | Role | Timezone | Notes -->
```

### MEMORY.md starter content

```markdown
# Active Memory

## Active Projects

<!-- Max 7 bullets. One-liners only. Evict: priority-desc then recency-desc. -->

## Open Risks

<!-- Max 10 bullets. Format: [P0/P1/P2] Risk — owner -->

## Key Decisions

<!-- Max 10 bullets. Format: YYYY-MM-DD | Decision -->

## Team Context

<!-- Max 20 bullets. -->
```

### config.toml

```toml
[context]
max_bytes.SOUL = 4096
max_bytes.USER = 6144
max_bytes.MEMORY = 8192

[memory]
max_active_projects = 7
max_open_risks = 10
max_key_decisions = 10
max_team_context = 20

[hooks]
transcript_tail_max_bytes = 50000
assistant_text_head_bytes = 2048
assistant_text_tail_bytes = 4096
tool_args_summary_bytes = 256

[search]
vector_weight = 0.7
candidate_k = 50
```

### Context size tests

`check_context_size.py` reads `config.toml [context] max_bytes.*`, checks
`len(content.encode("utf-8"))` for each injected file, exits non-zero with a
clear message if any exceed their limit.

### Database

The system uses two SQLite-compatible databases:

- **Cloud DB (source of truth) — Turso/libSQL.** Holds the event log and all
  projections. Accessed via libSQL embedded replica: a local SQLite file at
  `~/.local/share/second-brain/<instance>/cloud-replica.db` mirrors the cloud
  primary. Reads hit the local file (sub-ms); writes go to the primary and sync
  back. Configured in the instance's `secrets.toml`:

  ```toml
  # Turso (embedded replica) — production
  [db]
  replica_path = "/home/bayov/.local/share/second-brain/work/cloud-replica.db"
  sync_url = "libsql://my-db.turso.io"
  auth_token = "..."

  # Local-only fallback — omit sync_url and auth_token
  [db]
  replica_path = "/home/bayov/.local/share/second-brain/work/cloud.db"
  ```

  `db.py` connects via the libSQL Python client roughly as:

  ```python
  cfg = secrets["db"]
  if "sync_url" in cfg:
      conn = libsql.connect(
          cfg["replica_path"],
          sync_url=cfg["sync_url"],
          auth_token=cfg["auth_token"],
      )
  else:
      conn = sqlite3.connect(cfg["replica_path"])
  ```

  Plain SQL only — no Turso-specific extensions, so a swap requires no code
  changes.

- **Local RAG DB — plain SQLite.** Vector + keyword search index at
  `~/.local/share/second-brain/<instance>/rag.db`. Pure cache, regenerable via
  `memory_index.py`.

#### Cloud DB schema

```sql
-- Append-only event log — the source of truth.
-- This is the ONLY table the MVP ships. Projection tables (`actions`,
-- `event_processing`, `notifications`, `chat_messages`, `agents`,
-- `projection_cursors`) are introduced in the PRDs that produce their first
-- writer — see `04-heartbeat-and-reflection.md` and `05-chat-interface.md`.
CREATE TABLE events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source          TEXT NOT NULL,         -- 'manual', 'session_compact', 'session_end', 'human-vault'
    type            TEXT NOT NULL CHECK (type IN (
                        'note',                 -- escape hatch for ad-hoc manual entry
                        'session_summary',
                        'human_vault_commit',
                        'human_vault_synced'
                    )),
    source_id       TEXT,                  -- external dedupe key — nullable
    occurred_at     TIMESTAMP NOT NULL,
    ingested_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    payload         TEXT NOT NULL,         -- JSON-as-text
    metadata        TEXT,
    schema_version  INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX events_source_dedupe
    ON events(source, source_id) WHERE source_id IS NOT NULL;
CREATE INDEX events_type_occurred ON events(type, occurred_at);
```

#### Event types (MVP)

`events.type` is a closed enum (DB-enforced via `CHECK`). MVP values:

| Event type           | Source                        | When written                                        |
| -------------------- | ----------------------------- | --------------------------------------------------- |
| `note`               | manual                        | `sb-event add` CLI — escape hatch for ad-hoc entry  |
| `session_summary`    | session_compact / session_end | Hook on compact / session end                       |
| `human_vault_synced` | human-vault                   | After `sync_human_vault()` runs                     |
| `human_vault_commit` | human-vault                   | One per new commit observed by `sync_human_vault()` |

`note` is the deliberate escape hatch: anything Hagai wants to capture without
shipping a migration goes there with discriminating `payload.tags`. When a
`note` shape recurs enough to deserve its own type, it's promoted to a real enum
value with a migration.

Future PRDs extend the CHECK list with: `email_thread`, `calendar_event`,
`action_proposed`, `action_responded`, `event_processed`, `notification_sent`,
`chat_message`.

#### Writing events

All writers go through `write_event`:

```python
def write_event(event: Event) -> int | None:
    with conn:
        row = conn.execute(
            "INSERT INTO events (source, type, source_id, occurred_at, payload, metadata) "
            "VALUES (?, ?, ?, ?, ?, ?) "
            "ON CONFLICT(source, source_id) DO NOTHING "
            "RETURNING id",
            (
                event.source,
                event.type,
                event.source_id,
                event.occurred_at,
                json.dumps(event.payload),
                json.dumps(event.metadata) if event.metadata else None,
            ),
        ).fetchone()
        return row[0] if row else None  # None == duplicate (idempotent skip)
```

The MVP has no projections, so `write_event` is a pure insert. The
`project_event(conn, event)` dispatcher and the projection tables it writes to
are introduced in `04-heartbeat-and-reflection.md`, where the first projection
consumers (`actions`, `event_processing`, `notifications`) land. From that PRD
onward, `write_event` performs the insert and dispatch in a single transaction.

**Dependencies:** None — this is the foundation. **Estimated complexity:** Low

---

## Phase 2: Hooks (Context Persistence)

**What to build:** Three Python hook scripts that inject memory into every
Claude conversation and write session events to the event log on compaction or
session end.

**Key files:**

```
~/dotfiles/second-brain/claude/hooks/
├── _setup.py
├── session-start-context.py
├── pre-compact-flush.py
└── session-end-flush.py
```

**Hook lifecycle mapping.** `SessionStart` for inject; `PreCompact` and
`SessionEnd` for the two flush moments. **Not** `Stop` (fires every turn).

**Hook configuration in `.claude/settings.json`:**

```json
{
    "hooks": {
        "SessionStart": [
            {
                "type": "command",
                "command": "uv run --project $HOME/dotfiles/second-brain python .claude/hooks/session-start-context.py"
            }
        ],
        "PreCompact": [
            {
                "type": "command",
                "command": "uv run --project $HOME/dotfiles/second-brain python .claude/hooks/pre-compact-flush.py"
            }
        ],
        "SessionEnd": [
            {
                "type": "command",
                "command": "uv run --project $HOME/dotfiles/second-brain python .claude/hooks/session-end-flush.py"
            }
        ]
    }
}
```

**Stdin contract.** Every hook receives a JSON payload on stdin with at least:

```json
{
    "session_id": "abc-123-...",
    "transcript_path": "/home/bayov/.claude/projects/.../<session>.jsonl",
    "cwd": "/home/bayov/second-brain/work",
    "hook_event_name": "SessionStart",
    "source": "startup"
}
```

`_setup.py`:

```python
import json, pathlib, sys, tomllib

from sb.instance import find_instance
from sb.scripts.db import write_event, Event

_PAYLOAD = json.load(sys.stdin) if not sys.stdin.isatty() else {}
SESSION_ID = _PAYLOAD.get("session_id", "unknown")
TRANSCRIPT = (
    pathlib.Path(_PAYLOAD["transcript_path"])
    if "transcript_path" in _PAYLOAD
    else None
)
CWD = _PAYLOAD.get("cwd", str(pathlib.Path.cwd()))
HOOK_EVENT = _PAYLOAD.get("hook_event_name", "unknown")

INSTANCE_DIR = find_instance(pathlib.Path(CWD))
with (INSTANCE_DIR / "config.toml").open("rb") as f:
    config = tomllib.load(f)
```

`_setup.py` also wraps each hook's `main()` in soft-fail boilerplate (catch all
exceptions, write traceback to `~/.local/log/second-brain/<instance>/hooks.log`,
exit 0 — never block Claude).

### `session-start-context.py` — inject memory

Reads `config.toml [context] max_bytes.*` and injects each listed file into the
system prompt, wrapped in XML-like tags. Inject list is config-driven.

For MVP, entries are `SOUL`, `USER`, `MEMORY`.

Output format:

```json
{
    "hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": "<soul>\n{SOUL.md}\n</soul>\n\n<user>\n{USER.md}\n</user>\n\n<memory>\n{MEMORY.md}\n</memory>"
    }
}
```

`SessionStart` fires on `startup`, `resume`, `clear`, and `compact`. Behavior is
identical for all four — files are re-read every time.

### `pre-compact-flush.py` — flush before compaction

Reads `TRANSCRIPT`, distills it via the role-aware helper below, writes one
`session_summary` event, then commits any AI-vault changes accumulated during
the session-so-far:

```python
distilled, byte_range = distill_transcript(TRANSCRIPT, config["hooks"])
write_event(
    Event(
        source="session_compact",
        type="session_summary",
        source_id=None,
        occurred_at=now_utc(),
        payload=distilled,
        metadata={
            "session_id": SESSION_ID,
            "harness": "claude-code",
            "model": _PAYLOAD.get("model"),
            "cwd": CWD,
            "window": "pre-compact",
            "transcript_byte_range": byte_range,  # [start, end] in TRANSCRIPT
        },
    )
)
commit_ai_vault_if_dirty(f"session {SESSION_ID} [pre-compact]")
```

### `session-end-flush.py`

Identical to `pre-compact-flush.py` except `source="session_end"`,
`metadata.window = "session-end"`, and the commit reason is
`f"session {SESSION_ID} [session-end]"`.

**On the two events being complementary.** A long session that compacts mid-way
will produce two `session_summary` events: one at PreCompact (capturing "what
was about to be forgotten") and one at SessionEnd (capturing "what was active at
exit"). The transcript JSONL keeps growing across compaction, so the SessionEnd
distill re-walks content that overlaps the PreCompact event. This is by design —
the 50KB tail cap means a long session's earlier turns survive only in the
PreCompact event. The `metadata.window` and `metadata.transcript_byte_range`
fields let downstream consumers stitch the two windows without guessing.

### Transcript distillation (`distill_transcript`)

Bounded role-aware truncation:

| Entry type         | Treatment                                                                                  |
| ------------------ | ------------------------------------------------------------------------------------------ |
| User text          | Full content. No truncation.                                                               |
| Assistant text     | If > head+tail: keep first 2048 + last 4096 bytes joined by `[... N chars truncated ...]`. |
| Assistant tool_use | `{tool_name, args_summary}` truncated to 256 bytes.                                        |
| User tool_result   | **Dropped.** The next assistant turn already distills what mattered.                       |

Walk JSONL from the end backwards, accumulating until serialized payload
approaches 50KB. Return `(payload, byte_range)` where `byte_range` is
`[first_included_byte_offset, file_size]` in `TRANSCRIPT` — written into
`metadata.transcript_byte_range` so consumers can dedupe overlap between the
PreCompact and SessionEnd events.

Payload shape:

```json
{
    "messages": [
        { "role": "user|assistant", "content": "...", "timestamp": "..." }
    ],
    "stats": {
        "message_count": 47,
        "first_message_at": "...",
        "last_message_at": "..."
    }
}
```

### Tests

```
~/dotfiles/second-brain/tests/hooks/
├── conftest.py
├── test_distill.py
├── test_session_start.py
├── test_pre_compact.py
└── test_session_end.py
```

**Dependencies:** Phase 1. **Estimated complexity:** Medium

---

## Phase 3: Memory Search (Hybrid RAG)

**What to build:** A local search pipeline that queries every vault present in
the instance by meaning (semantic) and keyword, merged for best results. Runs in
a **separate, local-only SQLite file**
(`~/.local/share/second-brain/<instance>/rag.db`).

**Scope of this phase:** ship the indexer + search library + CLI. **No edits to
the Phase 2 hooks.** `memory_search.py` is importable Python _and_ a CLI.

**Language scope:** Multilingual via `BAAI/bge-m3` (1024-dim, ~560MB, 100+
languages incl. Hebrew). To bias toward English representations and keep
cross-language search tight, non-English chunk text is translated to English
before embedding (see "Translation pipeline" below); the original text is
preserved on the chunk row for audit and FTS. FTS5's `unicode61` tokenizer
covers non-Latin script for keyword search natively.

**Key files:**

```
src/sb/scripts/
├── rag_db.py
├── embeddings.py
├── memory_index.py
└── memory_search.py
```

**Library setup:** add `fastembed` and `sqlite-vec` to `pyproject.toml`. FTS5 is
built into Python's stdlib `sqlite3`.

### `rag_db.py` — schema

The RAG DB is a pure cache, so schema bumps **drop and rebuild**.
`PRAGMA user_version` is checked on every connection.

```sql
CREATE TABLE chunks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_path     TEXT NOT NULL,
    source_tag      TEXT NOT NULL,        -- 'ai-vault' | 'human-vault'
    chunk_index     INTEGER NOT NULL,
    char_start      INTEGER NOT NULL,
    char_end        INTEGER NOT NULL,
    text            TEXT NOT NULL,        -- original-language text (audit fidelity)
    lang            TEXT NOT NULL,        -- ISO 639-1, e.g. 'en', 'he' (from langdetect)
    translated_text TEXT,                 -- English translation; NULL when lang='en'
    mtime_ns        INTEGER NOT NULL,
    file_hash       TEXT NOT NULL,
    embed_model     TEXT NOT NULL,
    indexed_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX chunks_path_idx ON chunks(source_path, chunk_index);
CREATE INDEX chunks_tag_idx ON chunks(source_tag);

-- bge-m3 is 1024-dim. Embeddings are computed on `translated_text` when
-- present, else `text` — the vector space stays English-biased.
CREATE VIRTUAL TABLE vec_chunks USING vec0(embedding float[1024]);

-- FTS indexes BOTH the original `text` and the `translated_text` so keyword
-- search hits queries phrased in either language. unicode61 handles non-Latin
-- script for the original-language column.
CREATE VIRTUAL TABLE fts_chunks USING fts5(
    text, translated_text, content='', tokenize='unicode61'
);

CREATE TABLE indexed_files (
    source_path  TEXT PRIMARY KEY,
    source_tag   TEXT NOT NULL,
    mtime_ns     INTEGER NOT NULL,
    file_hash    TEXT NOT NULL,
    chunk_count  INTEGER NOT NULL,
    indexed_at   TIMESTAMP NOT NULL
);
```

### `embeddings.py`

```python
from fastembed import TextEmbedding

# bge-m3 is multilingual; 1024 dims; ~560MB on disk. Slower than bge-small
# (~3-4×) but irrelevant for hook-time / scheduled indexing of personal notes.
model = TextEmbedding(model_name="BAAI/bge-m3")


def embed_batch(texts: list[str]) -> list[list[float]]:
    return [v.tolist() for v in model.embed(texts, batch_size=32)]


def embed_query(query: str) -> list[float]:
    return next(model.query_embed(query)).tolist()
```

### Translation pipeline

`memory_index.py` runs each chunk through a small detect-and-maybe-translate
pass before persisting. Detection is `langdetect` (pure Python, no model
download). When the detected language is not English, the indexer calls Claude
Haiku via the Anthropic API to translate the chunk to English; the result is
stored in `chunks.translated_text`. The English-or-translated text is what gets
embedded and added to `fts_chunks.translated_text`; the original goes to
`chunks.text` and `fts_chunks.text`.

```python
from anthropic import Anthropic
from langdetect import detect, LangDetectException

_anthropic = Anthropic()  # reads ANTHROPIC_API_KEY


def detect_lang(text: str) -> str:
    try:
        return detect(text)  # ISO 639-1
    except LangDetectException:
        return "en"  # short / ambiguous → assume English, skip translation


def translate_to_english(text: str) -> str:
    msg = _anthropic.messages.create(
        model="claude-haiku-4-5-20251001",
        max_tokens=2048,
        messages=[
            {
                "role": "user",
                "content": (
                    "Translate the following text to English. Return only the "
                    "translation, no preamble. Preserve markdown structure, "
                    "wikilinks ([[...]]), and code blocks verbatim.\n\n" + text
                ),
            }
        ],
    )
    return msg.content[0].text
```

**API key** lives in `secrets.toml [anthropic] api_key`; bootstrap prompts for
it. Translation is per-chunk-once: re-indexing a file with unchanged hash reuses
the stored `translated_text`. Cost at MVP volume (personal notes, mostly English
with occasional Hebrew) is negligible.

**Trade-off accepted.** This couples `sb-index` to the Anthropic API at index
time. Offline indexing of non-English chunks is not supported in MVP. If this
becomes a concern, swap `translate_to_english` for a local NLLB-200 model with
no other code changes.

### `memory_index.py` — vault discovery + incremental pipeline

Vaults are **discovered at runtime**, not configured. Missing directories
(human-vault before Phase 4.5 mounts the submodule) are silently skipped with
one INFO log line.

```python
KNOWN_VAULTS = ("ai-vault", "human-vault")


def discover_vaults() -> list[tuple[Path, str]]:
    instance_dir = find_instance()
    return [
        (instance_dir / name, name)
        for name in KNOWN_VAULTS
        if (instance_dir / name).is_dir()
    ]


def run_index(source_filter: str | None = None):
    for path, source_tag in discover_vaults():
        if source_filter and source_tag != source_filter:
            continue
        index_vault(path, source_tag=source_tag)
        reconcile_deletes(path, source_tag=source_tag)
```

**File-scope rules:**

- Extension allowlist: `.md` only.
- Path exclusions: `.obsidian/`, `.git/`, `.gitmodules/`, `.trash/`, `.claude/`.
- Don't follow symlinks.
- Skip files with <50 chars of post-frontmatter body.
- Always include `SOUL.md`, `USER.md`, `MEMORY.md`.

**Skip logic (per file):**

1. Look up `(source_path)` in `indexed_files`.
2. If `mtime_ns` matches → skip.
3. Else hash; if `file_hash` matches → update `mtime_ns` only.
4. Else re-chunk and re-embed.

**Frontmatter:** YAML frontmatter is stripped before chunking.

**Chunking** (~400 tokens with 50-token overlap, paragraph-aware):

1. Tokenize with the **BGE tokenizer**.
2. Parse into "blocks": paragraphs, fenced code blocks, tables, list items.
3. Greedy-pack blocks into chunks until next block would exceed 400 tokens.
4. **Atomic units:** code blocks, tables, list items don't split mid-block.
5. **Overlap:** last 50 tokens of previous chunk prepended to next chunk.
6. **Heading context:** prepend nearest H1/H2/H3 trail when chunk starts
   mid-section.
7. Wikilinks `[[Target]]` kept verbatim.
8. Strip before embedding: HTML comments, image refs (keep alt), reference-style
   links.
9. Warn at >100 chunks per file.

**Per-file isolation:** wrap in `try/except`; log + continue. Indexer ends with
summary counters and exits non-zero only if `errors>0`.

**Reconcile deletes:** after upsert, list every `source_path` in `chunks` for
the vault, diff against on-disk file set, `DELETE` orphan chunks.

**Concurrency:** exclusive `flock` on `rag.db.lock` at startup
(`fcntl.flock(LOCK_EX | LOCK_NB)`). On contention exit 0 with info log.

**CLI:**

```bash
sb-index                          # All discovered vaults, full incremental pass
sb-index --source human-vault     # One vault only
sb-index --verbose
```

### `memory_search.py` — hybrid search

```python
@dataclass
class SearchHit:
    source_path: str
    source_tag: str
    chunk_index: int
    score: float
    vector_score: float
    bm25_score: float
    snippet: str
    text: str


def search(
    query: str, source: str | None = None, limit: int = 5
) -> list[SearchHit]: ...
```

**Hybrid scoring (union-then-rescore):**

1. Top-50 from each side independently (vec-sqlite + FTS5 BM25).
2. Union; for any chunk missing from one side's top-50, fetch its actual score
   on that side.
3. Vector normalization: L2 distance → similarity via `1 / (1 + distance)`.
4. BM25 normalization: min-max within candidate set, then invert.
5. Fuse: `vector_weight * vec + (1 - vector_weight) * bm25` (default 0.7).
6. Sort desc; tie-break on lower `chunk_index`, then more recent `mtime_ns`.
7. **Adjacent-chunk dedup:** drop the lower-scored of two results with same
   `source_path` and `|chunk_index_a − chunk_index_b| ≤ 1`.
8. Return top-`limit`.

**Snippet:** FTS5 `snippet()` for BM25-dominant hits; first 200 chars otherwise.

**CLI:**

```bash
sb-search "cross-team blockers" --limit 5
sb-search "..." --source ai-vault
sb-search "..." --json
```

**MVP operational note:** indexing runs every 5 minutes via the `sb-scheduler`
daemon (Phase 9). Hagai can also invoke `sb-index` ad-hoc for mid-cycle
freshness. The scheduler is started by bootstrap and managed as a systemd-user
service.

**Dependencies:** Phase 1, Phase 2. **Estimated complexity:** Medium

---

## Phase 4 (slim): Manual Event Entry

**What to build:** A CLI that writes events to the cloud DB, exercising the same
`write_event()` write path that future automated integrations will use. The MVP
ships zero auto-collectors; Hagai adds events by hand to test and seed the
system.

**Key file:**

```
src/sb/scripts/
└── event_cli.py            # exposed as `sb-event`
```

### Why a manual CLI

The events table is the spine of the system. Future PRDs (Gmail, Calendar,
Discord-out, heartbeat, chat) all bottom out in `write_event()`. Shipping that
path with a manual writer:

- Validates the event-log + projection-replay design end to end.
- Lets Hagai feed signal in (notes, observations, hand-curated risks) while the
  auto-collectors are still paper.
- Gives a stable surface — every future integration becomes "wire your collector
  to the same `write_event()` call."

The integration registry / template / `query.py` skeleton from the original
Phase 4 is **not** built in MVP. It's added in the first integration PRD
(`01-gmail-integration.md`) where it has at least one consumer. Building it in
advance with no callers would be premature abstraction.

### `sb-event` CLI

```bash
# Add a manual note event. occurred_at defaults to now (UTC).
sb-event add note --title "Risk: vendor X SLA" --body "Meeting Tue, escalate"

# Add a manual note from a file.
sb-event add note --title "..." --body-file ./draft.md

# With explicit timestamp.
sb-event add note --title "..." --body "..." --occurred-at 2026-04-30T14:00:00Z

# List recent events (JSON or table).
sb-event list --limit 20
sb-event list --type note --since 2026-04-29 --json

# Show one event.
sb-event show <id>
```

Implementation:

```python
# src/sb/scripts/event_cli.py
import argparse, json, sys, uuid
from datetime import datetime, timezone
from sb.scripts.db import write_event, Event, conn


def cmd_add(args):
    payload = {
        "title": args.title,
        "body": args.body
        or (open(args.body_file).read() if args.body_file else ""),
        "tags": args.tag or [],
    }
    occurred_at = args.occurred_at or datetime.now(timezone.utc).isoformat()
    # No idempotency: each invocation = a new event. A retry (e.g. after
    # Ctrl-C) creates a duplicate row, which is fine for manual entry.
    eid = write_event(
        Event(
            source="manual",
            type=args.type,  # 'note' for MVP — see events.type CHECK constraint
            source_id=None,
            occurred_at=occurred_at,
            payload=payload,
            metadata={"entered_by": "cli"},
        )
    )
    print(f"event {eid} written")


def cmd_list(args):
    rows = conn.execute(
        "SELECT id, source, type, occurred_at, payload FROM events "
        "WHERE (? IS NULL OR type = ?) AND (? IS NULL OR occurred_at >= ?) "
        "ORDER BY id DESC LIMIT ?",
        (args.type, args.type, args.since, args.since, args.limit),
    ).fetchall()
    if args.json:
        json.dump([dict(r) for r in rows], sys.stdout, default=str)
    else:
        for r in rows:
            print(r)
```

**Event types in MVP.** Only `note` is documented, but the schema accepts
arbitrary type strings — Hagai can experiment freely. Future PRDs introduce
typed payloads (`email_thread`, `calendar_event`, …) with structured payload
shapes.

### Acceptance — MVP Phase 4 "done"

- `sb-event add note --title X --body Y` writes a row to `events`.
- `sb-event list` returns it.
- `sb-event` exit code is non-zero on validation errors (missing title, bad
  timestamp).
- Each `sb-event add` invocation writes a new event. No idempotency: a re-submit
  (Ctrl-C retry) creates a duplicate row. Acceptable for a manual CLI; callers
  that need dedup are introduced by the integration PRDs and supply their own
  stable `source_id`.
- `sb-search` finds the event via the AI-vault if Hagai chose to also write the
  note into `ai-vault/` (RAG indexes vault files, not the events table — by
  design; the events table is the spine, the vault is the searchable surface).

**Dependencies:** Phase 1. **Estimated complexity:** Low

---

## Phase 4.5: Human Vault Integration (Submodule + Promotion via MR)

**What to build:** Mount the GitLab repo backing `~/notes/work/` as a git
submodule inside the project (`~/second-brain/work/human-vault/`) so the agent
has read access, and a `promote-to-human-vault` skill that proposes durable
knowledge from the AI vault to the human vault via MR — with mandatory
reformulation and dedup checks. The two vaults stay fully independent: content
moves AI → human only via MR, never via direct copy.

**Single-repo mode (canonical).** One GitLab user, one repo. Hagai pushes
directly to `main` from his working copy; the agent is restricted to feature
branches + MR. The restriction is enforced by two layers in MVP:

1. **Soft guardrail** — `SOUL.md` "Promotion Discipline" + the SKILL workflow
   tell the agent to push only `ai/promote-*` branches and open MRs.
2. **GitLab branch protection on `main`** — server-side hard stop. Even if the
   agent attempts `git push origin main`, the remote rejects it.

A PreToolUse-level hard guardrail (regex-blocking `git push origin main` at the
Bash-tool boundary) was in scope for an earlier draft and is deferred to
`07-security-hardening.md`. The two layers above are sufficient for MVP.

**GitLab instance:** `gitlab_host` (default `gitlab.com`) is set per instance in
`config.toml [human_vault]`. For self-hosted corporate GitLab, **verify with
your employer** before pushing personal notes.

**Vault paths:**

- Hagai's working copy (Obsidian, source of truth): `~/notes/work/`
- Agent's read mirror (submodule): `~/second-brain/work/human-vault/`

The two clones share `origin`. Hagai writes to upstream from `~/notes/work/`;
the agent fast-forwards the submodule from `origin` (via the
`sync_human_vault()` helper documented below) and never pushes to `main`.

In MVP this `sync` is run on demand by Hagai (`sb-sync-human-vault` console
script — simple wrapper around the function). The recurring "fast-forward from
cron" piece moves to `04-heartbeat-and-reflection.md`.

**Key files:**

```
~/second-brain/work/human-vault/                  # git submodule
src/sb/scripts/memory_index.py                    # already dual-vault (Phase 3)
src/sb/scripts/sync_human_vault.py                # exposed as sb-sync-human-vault
claude/skills/promote-to-human-vault/
├── SKILL.md
└── scripts/
    └── promote.py
```

**Configuration:**

```toml
# ~/dotfiles/second-brain/config.toml
[human_vault]
gitlab_host = "gitlab.com"
repo = "donbayov/work-notes"
```

```toml
# ~/second-brain/<instance>/secrets.toml  (gitignored)
[gitlab]
token = "glpat-..." # PAT, scopes: api, read_repository, write_repository
```

The `[human_vault]` section's presence is the opt-in flag — bootstrap and the
indexer skip submodule logic when it's absent.

**One-time GitLab setup:**

Install `glab` CLI. The agent does **not** run `glab auth login` — `glab` is
authenticated per-subprocess via env vars exported from
`secrets.toml [gitlab]
token` and `config.toml [human_vault] gitlab_host` (see
`promote.py` below). This keeps a single source of truth for the token in
`secrets.toml` and avoids a second secrets store under `~/.config/glab-cli/`.

1. **Push `~/notes/work` to GitLab:**
   ```bash
   glab repo create donbayov/work-notes --private
   git -C ~/notes/work remote add origin git@gitlab.com:donbayov/work-notes.git
   git -C ~/notes/work push -u origin main
   ```

2. **Branch-protect `main`:**
   - Settings → Repository → Protected branches → `main`
   - Allowed to push: `Maintainers` (you).
   - Allowed to merge: `Maintainers`.

3. **Personal Access Token** dedicated to the agent — scopes: `api`,
   `read_repository`, `write_repository`. Store in
   `secrets.toml [gitlab]
   token`.

4. **Add the repo as a submodule:**
   ```bash
   cd ~/second-brain/work
   git submodule add git@gitlab.com:donbayov/work-notes.git human-vault
   git commit -m "Add human-vault submodule"

   git config -f .gitmodules submodule.human-vault.ignore all
   git config submodule.human-vault.ignore all
   ```

   The submodule pointer is **not** pinned in practice — the agent keeps the
   submodule's `main` branch fast-forwarded from `origin`. Avoid running
   `git submodule update` after the initial clone; let `sync_human_vault()`
   manage it.

5. **Set the agent's git identity:**
   ```bash
   git -C ~/second-brain/work config user.name  "bayov-agent"
   git -C ~/second-brain/work config user.email "bayov-agent@second-brain.local"
   git -C ~/second-brain/work/human-vault config user.name  "bayov-agent"
   git -C ~/second-brain/work/human-vault config user.email "bayov-agent@second-brain.local"
   ```

6. **Dedicated SSH key:**
   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/bayov_agent -C "bayov-agent"
   # ~/.ssh/config — point the submodule's host at the dedicated key:
   #   Host gitlab.com-bayov-agent
   #     HostName gitlab.com
   #     IdentityFile ~/.ssh/bayov_agent
   #     IdentitiesOnly yes
   git -C ~/second-brain/work/human-vault remote set-url origin \
     git@gitlab.com-bayov-agent:donbayov/work-notes.git
   ```
   Add `~/.ssh/bayov_agent.pub` to GitLab → User Settings → SSH Keys.

**`sync_human_vault()` — fast-forward + commit-event ingestion:**

```python
HUMAN_VAULT_PATH = Path.home() / "second-brain/work/human-vault"


def sync_human_vault() -> bool:
    """Fast-forward human-vault to origin/main. For each new commit, write
    one `human_vault_commit` event so future heartbeat reasoning can react
    to Hagai's note changes. Returns True if HEAD advanced.
    """
    head_before = subprocess.check_output(
        ["git", "-C", str(HUMAN_VAULT_PATH), "rev-parse", "HEAD"], text=True
    ).strip()
    for cmd in [
        ["git", "-C", str(HUMAN_VAULT_PATH), "fetch", "origin"],
        [
            "git",
            "-C",
            str(HUMAN_VAULT_PATH),
            "merge",
            "--ff-only",
            "origin/main",
        ],
    ]:
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            log_warning(
                f"Human vault sync failed: {' '.join(cmd)}\n{result.stderr}"
            )
            return False
    head_after = subprocess.check_output(
        ["git", "-C", str(HUMAN_VAULT_PATH), "rev-parse", "HEAD"], text=True
    ).strip()
    advanced = head_before != head_after

    if advanced:
        log = subprocess.check_output(
            [
                "git",
                "-C",
                str(HUMAN_VAULT_PATH),
                "log",
                "--reverse",
                "--format=%H%x00%aI%x00%s",
                f"{head_before}..{head_after}",
            ],
            text=True,
        ).strip()
        for line in log.splitlines():
            sha, iso_time, subject = line.split("\x00", 2)
            files = []
            raw_files = (
                subprocess.check_output(
                    [
                        "git",
                        "-C",
                        str(HUMAN_VAULT_PATH),
                        "show",
                        "--name-status",
                        "--format=",
                        sha,
                    ],
                    text=True,
                )
                .strip()
                .splitlines()
            )
            for entry in raw_files:
                parts = entry.split("\t")
                status, path = parts[0], parts[-1]
                files.append({"status": status, "path": path})
            write_event(
                Event(
                    source="human-vault",
                    type="human_vault_commit",
                    source_id=sha,
                    occurred_at=iso_time,
                    payload={"sha": sha, "subject": subject, "files": files},
                )
            )

    write_event(
        Event(
            source="heartbeat",  # kept for forward-compat; future heartbeat owns this
            type="human_vault_synced",
            occurred_at=now_utc(),
            payload={
                "head_before": head_before,
                "head_after": head_after,
                "advanced": advanced,
            },
        )
    )
    return advanced
```

**`promote.py` — reformulate-then-MR (the only writer to human vault):**

```python
HUMAN_VAULT_PATH = Path.home() / "second-brain/work/human-vault"
GITLAB_HOST = config["human_vault"]["gitlab_host"]
HUMAN_VAULT_REPO = config["human_vault"]["repo"]
GITLAB_TOKEN = secrets["gitlab"]["token"]

PromotionTrigger = Literal[
    "decision-crystallized",
    "risk-tracked",
    "incident-lesson-learned",
    "user-request",
]


def promote(
    ai_vault_source: str,
    target_file: str,
    reformulated_content: str,
    trigger: PromotionTrigger,
    placement_rationale: str,
    dedup_result: str,
):
    if len(reformulated_content.strip()) < 100:
        raise ValueError("reformulated content too short — refusing to open MR")

    digest = hashlib.sha256(
        f"{ai_vault_source}\0{reformulated_content}".encode()
    ).hexdigest()[:8]
    branch = f"ai/promote-{date.today()}-{trigger}-{digest}"

    subprocess.run(
        ["git", "-C", HUMAN_VAULT_PATH, "checkout", "-b", branch], check=True
    )
    (HUMAN_VAULT_PATH / target_file).parent.mkdir(parents=True, exist_ok=True)
    (HUMAN_VAULT_PATH / target_file).write_text(reformulated_content)
    subprocess.run(
        ["git", "-C", HUMAN_VAULT_PATH, "add", target_file], check=True
    )
    subprocess.run(
        [
            "git",
            "-C",
            HUMAN_VAULT_PATH,
            "commit",
            "-m",
            f"AI promotion: {trigger}",
        ],
        check=True,
    )
    subprocess.run(
        ["git", "-C", HUMAN_VAULT_PATH, "push", "origin", branch], check=True
    )

    body = (
        f"**Proposed by:** Second Brain agent (`bayov-agent`)\n"
        f"**Trigger:** {trigger}\n"
        f"**AI-vault source:** `{ai_vault_source}`\n"
        f"**Placement:** `{target_file}` — {placement_rationale}\n"
        f"**Dedup check:** {dedup_result}\n\n"
        f"Review the diff, merge if useful, close if not. "
        f"Rejections with a one-line note improve future proposals."
    )
    subprocess.run(
        [
            "glab",
            "mr",
            "create",
            "--source-branch",
            branch,
            "--target-branch",
            "main",
            "--title",
            f"AI promotion: {trigger}",
            "--description",
            body,
            "--remove-source-branch",
            "--yes",
        ],
        env={
            **os.environ,
            # `glab` reads GITLAB_URI for the host (not GITLAB_HOST) and
            # GITLAB_TOKEN for auth. For gitlab.com both can be set anyway;
            # for self-hosted, GITLAB_URI is required.
            "GITLAB_URI": f"https://{GITLAB_HOST}",
            "GITLAB_TOKEN": GITLAB_TOKEN,
        },
        check=True,
    )
    subprocess.run(
        ["git", "-C", HUMAN_VAULT_PATH, "checkout", "main"], check=True
    )
```

**`promote-to-human-vault/SKILL.md`:**

```yaml
---
name: promote-to-human-vault
description: Promote durable AI-vault content to Hagai's human vault via GitLab MR. Reformulates to match human-vault voice, searches for related notes to avoid duplication, and decides extend-vs-new placement. Use for crystallized decisions, tracked risks, incident lessons, or explicit "save this" requests.
---
```

Mandatory workflow (enforced by SOUL.md "Promotion Discipline"):

1. **Trigger check.** Confirm content matches one of the five triggers in
   SOUL.md. If not, do not promote.
2. **Search human vault.** `sb-search "<topic>" --source human-vault`.
3. **Read top-3 results + list `human-vault/` top-level directories.**
   Judgment-based dedup. If any of the top-3 covers the same fact → propose a
   merge edit. If related but distinct → extend the relevant section. Otherwise
   → new file.
4. **Reformulate.** Rewrite to match human-vault voice/conventions. Never paste
   AI-vault prose verbatim.
5. **Open MR.** Call `promote.py` with all six fields.
6. **Tell Hagai.** "MR opened: [link]"

**Dependencies:** Phase 3 (dual-vault indexer). **Estimated complexity:**
Low–Medium

---

## Phase 5 (slim): Skills (MVP starter pack)

**What to build:** Two skills — one teaches Claude how the dual-vault layout
works, the other is the `promote-to-human-vault` skill described in Phase 4.5.

```
claude/skills/
├── vault-structure/
│   ├── SKILL.md
│   └── references/
│       └── vault-map.md
└── promote-to-human-vault/        # owned by Phase 4.5
    ├── SKILL.md
    └── scripts/
        └── promote.py
```

**`vault-structure/SKILL.md`:**

```yaml
---
name: vault-structure
description: Reference for Hagai's dual-vault layout — AI vault (~/second-brain/work/ai-vault/) and human vault (~/notes/work/, mirrored at ~/second-brain/work/human-vault/). Covers folder taxonomy (briefings/, dossiers/, meetings/, drafts/, analysis/, logs/, questions/), Workflow A vs B for meeting notes, and what triggers promotion (decision-crystallized, risk-tracked, incident-lesson-learned, user-request) vs. stays in AI vault.
---
```

Body: full map of both vaults, naming conventions, promotion triggers + the
discipline (mirroring SOUL.md), Workflow A/B switch for meeting notes, the
contract that the AI is sole writer of `ai-vault/` and only writes to
`human-vault/` via the `promote-to-human-vault` skill.

Other skills (`surface-risks`, future engineering-leadership skills) are
deferred to `06-extended-skills.md`. They depend on integrations
(calendar/gmail/heartbeat) that the MVP doesn't ship.

**Dependencies:** Phase 1–3, 4.5. **Estimated complexity:** Low

---

## Phase 9 (slim): Scheduler + Operational Surface

### `sb-scheduler` daemon

A long-running, always-on daemon, one process per instance, dispatches the MVP's
recurring jobs. It replaces what would otherwise be `cron` / `systemd-timer`
entries — keeping the dispatch logic in our own code keeps it inside
`find_instance()` resolution and `secrets.toml` loading without reimplementing
those for cron entries.

**MVP jobs:**

| Job            | Default interval | Command    |
| -------------- | ---------------- | ---------- |
| `index_events` | 5 min            | `sb-index` |

`sync_human_vault` is **not** scheduled in MVP; Hagai runs `sb-sync-human-vault`
on demand. It moves to the scheduler in `04-heartbeat-and-reflection.md` along
with the heartbeat itself.

**Implementation:** simple loop over `[[scheduler.jobs]]` from `config.toml`,
`subprocess.run(...)` each command at its interval, log result to
`~/.local/log/second-brain/<instance>/scheduler.log`. No threading, no
concurrent jobs in MVP — jobs are quick and serial is fine.

```toml
# config.toml
[[scheduler.jobs]]
name = "index_events"
command = "sb-index"
interval_seconds = 300
```

**systemd user unit (templated by instance):**

```ini
# ~/.config/systemd/user/sb-scheduler@.service  — installed by bootstrap.py
[Unit]
Description=Second-brain scheduler (instance %i)
After=default.target

[Service]
Type=simple
Environment=SECOND_BRAIN_INSTANCE=%h/second-brain/%i
ExecStart=%h/.local/bin/uv run --project %h/dotfiles/second-brain sb-scheduler
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=default.target
```

Bootstrap installs the unit (idempotent), then runs:

```bash
systemctl --user daemon-reload
systemctl --user enable --now sb-scheduler@<instance>
```

Logs are reachable via `journalctl --user -u sb-scheduler@work`. The unit
restarts on crash; it starts on login (WSL2 with systemd enabled satisfies this
— `/etc/wsl.conf` must have `[boot] systemd=true`, which is the modern WSL2
default).

**Multi-instance.** The `@<instance>` template makes the unit reusable: when the
planned `personal` instance lands,
`systemctl --user enable --now
sb-scheduler@personal` is the only step.

### Operational surface

```bash
# One-time: sync the venv from pyproject.toml.
uv sync --project ~/dotfiles/second-brain

# Bootstrap an instance.
uv run --project ~/dotfiles/second-brain python ~/dotfiles/second-brain/bootstrap.py work

# DB initialization (also called automatically by bootstrap).
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-db --init
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-rag-db --init

# Index vaults (run after editing notes).
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-index

# Search.
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-search "..."

# Add a manual event.
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-event add note --title ... --body ...

# Sync the human-vault submodule (Phase 4.5).
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-sync-human-vault

# Scheduler (managed by systemd-user; ad-hoc commands).
systemctl --user status sb-scheduler@work
journalctl --user -u sb-scheduler@work -f

# Operational commands on the cloud DB.
sb-db --dump > ~/backups/cloud-$(date +%F).sql   # snapshot
sb-db --status                                   # event count, sync health
# `sb-db --rebuild` (wipe + replay projections) lands with the projection layer
# in `04-heartbeat-and-reflection.md`; the MVP has no projections to rebuild.

# Swap-away from Turso (the embedded replica is the escape hatch).
sb-db --dump > ~/backups/cloud-final.sql
cp ~/.local/share/second-brain/work/cloud-replica.db \
   ~/.local/share/second-brain/work/cloud.db
# Then in secrets.toml [db]: drop sync_url + auth_token, point replica_path
# at the copy above (or just rename the replica file in place).
```

**Logs:** soft-fail hook errors go to
`~/.local/log/second-brain/<instance>/hooks.log`. Scheduler stdout/stderr go to
journald (`journalctl --user -u sb-scheduler@<instance>`). No log rotation in
MVP; lograte config moves to the deployment PRD.

**Cost estimate (MVP-only):** small but non-zero. The Anthropic API is now on
the indexing path for non-English chunk translation (Phase 3). Per-chunk-once
caching keeps cost bounded — at MVP volume (personal notes, mostly English)
expect cents-to-low-dollars per month. Turso free tier covers the DB. Other
Claude API usage is inside interactive Claude Code sessions Hagai already pays
for.

**Dependencies:** All MVP phases. **Estimated complexity:** Low (mostly docs).

---

## MVP build order

```
Phase 1 → Phase 2 → Phase 3 → Phase 4 (manual) → Phase 4.5 → Phase 5 (slim)
                                                       ↓
                                                Phase 9 (slim)
```

**Recommended first milestone (1–2 days):** Phases 1–2. Every Claude
conversation immediately benefits from memory injection.

**Recommended second milestone (2–3 days):** Phases 1–3 + manual events. Vaults
are searchable; events table is exercised.

**Recommended third milestone (1 day):** Phase 4.5 + slim 9. Human-vault
submodule mounted, indexer covers both vaults, `promote-to-human-vault` skill
works end-to-end, scheduler keeps the index fresh.

After that, pick from the follow-up PRDs in this directory based on which
capability you most want next.
