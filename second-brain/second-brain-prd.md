Hagai's Second Brain — Product Requirements Document
====================================================

**Generated:** 2026-04-29\
**Last updated:** 2026-04-30\
**Role:** Staff Software Engineer / Software Architect\
**Timezone:** Asia/Jerusalem (UTC+3 / UTC+2 winter)

**Summary:** An AI Second Brain optimized for engineering leadership workflows —
surfacing risks, proposing actions for review, tracking deadlines from Google
Calendar, and keeping meeting notes searchable. Integrates Gmail, Google
Calendar, and Discord; runs on Linux with Obsidian as the vault viewer. Source
of truth lives in a private cloud SQL DB (Turso/libSQL) with a
continuously-synced local replica. The agent operates two Obsidian vaults: a
rich **AI vault** at `~/second-brain/work/ai-vault/` it owns outright
(briefings, dossiers, drafts, analysis, reasoning logs), and a read-only mirror
of Hagai's curated **human vault** at `~/second-brain/work/human-vault/` — a git
submodule of the AI's fork of `~/notes/work`. Hagai writes freely in
`~/notes/work/`; the AI proposes durable changes via GitLab MR with
reformulation + dedup checks. The human-vault repo lives on Hagai's employer's
GitLab instance (self-hosted; `GITLAB_HOST` configurable, defaults to
`gitlab.com`).

**Architecture principle:** A cloud SQL DB (Turso/libSQL) is the single source
of truth, with an append-only `events` table at its core. All other tables
(`actions`, `event_processing`, `notifications`) are derived projections
rebuildable by replaying the event log. The DB is accessed via libSQL embedded
replica — reads come from a local SQLite file (sub-ms), writes go to the cloud
primary and sync back; every machine running the agent gets a continuous cold
backup as a side effect. The substrate is swappable: plain SQL only, one `db.py`
access layer, `DATABASE_URL` in `.env`. The system stays reprocessable with
better agents as they are released.

---

Phase 1: Foundation (Memory Layer + Cloud DB)
---------------------------------------------

**What to build:** The `bootstrap.py` scaffolder, AI-vault skeleton, core memory
files (SOUL/USER/MEMORY), cloud DB (Turso/libSQL with embedded replica), and
configuration (`config.toml`, `.env`). Every subsequent phase depends on these.

### Project layout

The system splits **code** (lives in dotfiles, single source of truth across all
brain instances) from **content** (per-instance repos under `~/second-brain/`).
Today there is one instance — `work`; a `personal` instance is planned,
bootstrapped against the same dotfiles tree.

**Code repo — `~/dotfiles/second-brain/`** (a subdirectory of Hagai's existing
dotfiles repo):

```
~/dotfiles/second-brain/
├── second-brain-prd.md              # this spec
├── bootstrap.py                     # scaffolds a new ~/second-brain/<instance>/
├── pyproject.toml                   # declares the `sb` package + console scripts
│                                    #   (sb-index, sb-search, sb-heartbeat,
│                                    #    sb-reflect, sb-db, sb-rag-db, sb-chat,
│                                    #    sb-context-check, sb-guardrail, ...)
├── src/sb/                          # ordinary importable Python package
│   ├── instance.py                  #   find_instance() — marker-file ascent
│   ├── scripts/                     #   db.py, memory_index.py, heartbeat.py, ...
│   ├── chat/                        #   adapters + core (Phase 7)
│   └── hooks/                       #   guardrail_check.py (Phase 8)
├── claude/
│   ├── hooks/                       # → <instance>/.claude/hooks/   (Claude Code lifecycle hooks)
│   └── skills/                      # → <instance>/.claude/skills/  (populated in Phase 4.5+)
├── schema/
│   └── 0001_init.sql                # → <instance>/schema.sql
└── starter/                         # COPIED at bootstrap; diverges per instance
    ├── ai-vault/
    │   ├── SOUL.md, USER.md, MEMORY.md, human-vault-map.md
    │   └── .obsidian/
    ├── config.toml                  # → <instance>/config.toml
    ├── claude-settings.json         # → <instance>/.claude/settings.json
    ├── env.example                  # → <instance>/.env
    └── gitignore                    # → <instance>/.gitignore
```

**Content instance — `~/second-brain/work/`** (a standalone git repo, no remote
required):

```
~/second-brain/work/
├── ai-vault/                        # COPIED from starter/ai-vault/, then evolves
│   ├── SOUL.md, USER.md, MEMORY.md, human-vault-map.md
│   ├── .obsidian/
│   └── briefings/, dossiers/, meetings/, drafts/, analysis/, logs/, questions/
├── config.toml                      # COPIED from starter/config.toml, then evolves
│                                    # (also serves as the instance marker — see find_instance)
├── human-vault/                     # git submodule → AI's fork (Phase 4.5)
├── .claude/
│   ├── hooks           → ~/dotfiles/second-brain/claude/hooks/
│   ├── skills          → ~/dotfiles/second-brain/claude/skills/
│   └── settings.json                # COPIED (per-instance permissions)
├── schema.sql          → ~/dotfiles/second-brain/schema/0001_init.sql
├── second-brain-prd.md → ~/dotfiles/second-brain/second-brain-prd.md
├── .env                             # COPIED from starter/env.example, edited
└── .gitignore                       # COPIED
```

**No `.agents/` symlink.** Earlier drafts symlinked
`~/dotfiles/second-brain/agents/` into each instance as `.agents/`, so scripts
could derive their instance dir from an unresolved `__file__`. That trick has
been retired in favor of an ordinary Python package (`src/sb/`) plus runtime
instance discovery via [`find_instance()`](#find_instance--instance-discovery).
Code is now invoked by console-script name (`sb-index`, `sb-heartbeat`, …) from
the instance directory, with `uv run --project ~/dotfiles/second-brain`
supplying the venv. The only remaining symlinks are the Claude-Code-side
artifacts (`.claude/hooks`, `.claude/skills`) which must live at well-known
instance-relative paths because Claude Code itself discovers them by path.

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
      1. `SECOND_BRAIN_INSTANCE` env var, if set (used by the systemd chat unit
         and by ad-hoc invocations that don't have a useful cwd).
      2. Marker-file ascent from `start` (default: `Path.cwd()`) — walk up until
         a directory containing `config.toml` is found. Same idea as how `git`
         finds the repo root.

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

Cron jobs `cd ~/second-brain/<instance>` before invoking the script; Claude Code
hooks receive `cwd` in their JSON payload and pass it to `find_instance`; the
chat systemd unit sets `SECOND_BRAIN_INSTANCE` explicitly. There is no other
path-discovery mechanism in the codebase — anything that needs the instance dir
calls `find_instance()`.

> **⚠️ Open inconsistency — needs a dedicated /align pass on Phase 1.** Phase 4
> was aligned on machine-local config at
> `~/.local/share/second-brain/<instance>/secrets.toml` (TOML, loaded via
> `tomllib`), with `~/.config` reserved for VCS-trackable data. This Phase 1
> section still describes a per-instance `.env` at
> `~/second-brain/<instance>/.env`, populated by bootstrap prompts (lines
> 128–181) — the same role, a different file, in a different location, in a
> different format. Phase 4.5 (GitLab section, lines ~1651, 1699, 1817) shares
> the same convention.
>
> Resolution should be a single `/align` pass on Phase 1 + 4.5 to migrate the
> whole bootstrap/secrets surface to `secrets.toml` at the
> `~/.local/share/second-brain/<instance>/` path. Until then, treat Phase 4 as
> the canonical decision and these older references as stale.

**Code/content split — rationale:**

- **Single source of truth for code.** All Python code, Claude-Code hooks,
  skills, and the cloud-DB schema live in `~/dotfiles/second-brain/`. Edit once
  and `uv run --project ~/dotfiles/second-brain` picks the change up immediately
  for every instance — no per-instance code drift.
- **Audit trail per instance.** Each `~/second-brain/<instance>/` is its own git
  repo. Its history captures only AI-vault content writes — clean separation
  between _code changes_ (commits to dotfiles) and _agent activity_ (commits to
  the instance repo).
- **Starter files diverge.**
  SOUL/USER/MEMORY/`config.toml`/`.claude/settings.json` are **copied** at
  bootstrap, not symlinked: `config.toml` carries per-instance tuning, SOUL tone
  differs work-vs-personal, USER holds per-instance account IDs, MEMORY evolves
  continuously, settings carry per-instance permissions.
- **Instance is movable.** The remaining `.claude/hooks` and `.claude/skills`
  symlinks are relative, and the package is found via uv's `--project` flag
  rather than any in-instance path. Moving an instance dir requires no code
  edits. Inside the instance repo, `git status` shows only data changes — code
  never appears as drift.

**Bootstrap:**

```bash
uv run --project ~/dotfiles/second-brain python ~/dotfiles/second-brain/bootstrap.py work       # creates ~/second-brain/work/
uv run --project ~/dotfiles/second-brain python ~/dotfiles/second-brain/bootstrap.py personal   # later, same template
```

`bootstrap.py` does, idempotently:

1. `git init ~/second-brain/<instance>/` if absent.
2. Copy `starter/` into the instance (preserving subtree shape).
3. Create the relative symlinks (`.claude/hooks`, `.claude/skills`,
   `schema.sql`, `second-brain-prd.md`).
4. **Interactively prompt for required + optional fields**, then render `.env`,
   `USER.md`, and `config.toml` from their starter templates. See "Bootstrap
   prompts" below.
5. Run `sb-db --init` (cloud DB schema; primes the local SQLite file in fallback
   mode; idempotent) and `sb-rag-db --init` (local RAG DB schema; no-op stub
   until Phase 3). Bootstrap invokes both as Python subprocesses with
   `cwd=<instance>` and `SECOND_BRAIN_INSTANCE=<instance>` so `find_instance()`
   resolves correctly. Since bootstrap itself runs under
   `uv run --project ~/dotfiles/second-brain`, the venv is already resolved and
   the subprocesses inherit it via `sys.executable`.
6. `git -C ~/second-brain/<instance> commit -m "Initial scaffold"` so the audit
   trail starts from a known baseline.

#### Bootstrap prompts

The script reads each starter file, finds `{{placeholders}}`, prompts for each
one, and writes the rendered file to the instance. Behavior:

- **Required vs. optional.** Required prompts loop until non-empty. Optional
  prompts accept empty input and leave a `# TODO` marker in the rendered file.
- **Defaults.** Where a sensible default exists (timezone, working hours,
  paths), the prompt shows it in `[brackets]` and accepts Enter to keep it.
- **Re-runnable.** If the instance already exists, `bootstrap.py` parses the
  existing `.env` / `USER.md` / `config.toml`, pre-fills prompts from current
  values, and only re-prompts for unset / `# TODO` entries. Pass `--reconfigure`
  to force re-prompting for everything.
- **Phase-gated sections.** Phase 4.5 (GitLab) and Phase 7 (chat adapters)
  prompts can be skipped — the script asks
  `Configure GitLab integration now?
  [y/N]` and writes only the relevant block
  if yes. Skipping leaves a commented-out block in `.env` for later activation.
- **Secrets stay out of git.** `.env` is gitignored; the bootstrap commit
  includes `USER.md` and `config.toml` only. The script verifies `.gitignore`
  contains `.env` before committing.

| Group        | Prompt                                                                      | Required           | Target              |
| ------------ | --------------------------------------------------------------------------- | ------------------ | ------------------- |
| Profile      | Full name                                                                   | required           | `USER.md`           |
| Profile      | Role (default: `Staff Software Engineer / Software Architect`)              | required (default) | `USER.md`           |
| Profile      | Timezone (default: `Asia/Jerusalem`)                                        | required (default) | `USER.md`           |
| Profile      | Working hours (default: `09:00–18:00 local`)                                | required (default) | `USER.md`           |
| Google       | Gmail address                                                               | required           | `USER.md`, `.env`   |
| Google       | OAuth credentials path (default: `~/.config/second-brain/credentials.json`) | required (default) | `.env`              |
| DB           | `DATABASE_URL` mode: `turso` or `local-sqlite`                              | required           | `.env`              |
| DB (turso)   | Turso DB URL (e.g. `libsql://my-db.turso.io`)                               | required if turso  | `.env`              |
| DB (turso)   | Turso auth token                                                            | required if turso  | `.env`              |
| Discord      | Configure Discord now? `[y/N]`                                              | optional           | gates the next two  |
| Discord      | Bot token                                                                   | required if y      | `.env`              |
| Discord      | Your Discord user ID (snowflake)                                            | required if y      | `.env`, `USER.md`   |
| GitLab (4.5) | Configure GitLab human-vault now? `[y/N]`                                   | optional           | gates the next four |
| GitLab       | `GITLAB_HOST` (default: `gitlab.com`)                                       | required if y      | `.env`              |
| GitLab       | Personal/Project access token                                               | required if y      | `.env`              |
| GitLab       | Upstream repo (e.g. `hagai/work-notes`)                                     | required if y      | `.env`, `USER.md`   |
| GitLab       | Fork repo (e.g. `hagai-agent/work-notes`)                                   | required if y      | `.env`              |
| Chat (7)     | `CHAT_ADAPTERS` (default: `discord`)                                        | optional (default) | `.env`              |

The starter templates use `{{var_name}}` markers; bootstrap substitutes them
verbatim. Adding a new field = add a marker to the relevant starter file plus
one row to the prompt table inside `bootstrap.py`.

A `bootstrap.py --check <instance>` mode (run via
`uv run --project
~/dotfiles/second-brain ...`) validates that no `# TODO`
markers remain in `.env` / `USER.md` and exits non-zero otherwise — useful as a
pre-flight before enabling the systemd service or cron jobs.

`ai-vault/` is a regular folder versioned in the content repo — its git history
is the audit trail for AI-vault writes. `human-vault/` is a git submodule
pointing to the AI's fork of the GitLab repo backing `~/notes/work`. The AI has
write access to the fork (MRs only) and read access to upstream via
`git fetch upstream`. See Phase 4.5.

### AI vault structure

```
~/second-brain/work/ai-vault/
├── .obsidian/                # Obsidian vault marker + config
├── .gitignore
├── SOUL.md                   # Agent persona, hard rules, editorial freedom, promotion discipline
├── USER.md                   # Hagai's profile, account IDs, team map, drafting criteria
├── MEMORY.md                 # Agent-owned live index: projects, risks, decisions, team context
├── human-vault-map.md        # AI's running mental model of human-vault structure (populated in Phase 4.5)
├── briefings/
│   ├── meetings/             # one .md per upcoming meeting (pre-meeting context)
│   └── daily/                # rolling daily snapshots produced by memory_reflect
├── meetings/                 # AI-digested meeting transcripts (Workflow A)
├── dossiers/
│   ├── people/               # one .md per person — rolling synthesis from comms + events
│   └── projects/             # one .md per project — rolling state
├── drafts/                   # final-form candidates pending promotion (MR to human vault)
├── analysis/                 # speculative — risk hypotheses, themes, "what if"
├── logs/                     # reasoning traces (why proposal X was made)
└── questions/                # open clarifications queued for Hagai
```

The AI vault is an **Obsidian vault** — open it directly in Obsidian to inspect
what the agent is doing. Internal `[[wikilinks]]` between dossiers, meetings,
and analysis form a navigable graph. Hagai is a read-only observer who skims
occasionally; the AI is sole writer.

`human-vault-map.md` is created empty (with a placeholder header) in Phase 1 and
populated in Phase 4.5 once the human-vault submodule exists. Until then,
references to it from skills/scripts are no-ops.

### Editorial model

- **Human vault** (`~/notes/work/`, mirrored read-only at
  `~/second-brain/work/human-vault/`): canonical record of Hagai's knowledge —
  meeting notes he writes, decisions he endorses, project plans, architecture
  docs, hand-written notes on people. Curated by Hagai directly. AI proposes
  changes only via MR through the `promote-to-human-vault` skill (Phase 4.5).
- **AI vault** (`~/second-brain/work/ai-vault/`): the AI's working memory +
  scratchpad. Briefings, dossiers, drafts, analysis, reasoning logs, open
  questions, digested transcripts. AI writes freely.

The two vaults stay independent — content moves AI → human only via MR with
reformulation, never via direct copy.

### Meeting notes — both workflows supported

- **Workflow A (transcript-driven):** AI ingests a transcript and writes a
  digest at `ai-vault/meetings/YYYY-MM-DD-name.md`. If outcomes are durable
  (decisions made, commitments taken), AI promotes to human vault.
- **Workflow B (Hagai-driven):** Hagai writes the meeting note directly in
  `~/notes/work/`. AI reads it and may add an analysis note in
  `ai-vault/analysis/` linking back via wikilink. No promotion — Hagai's note is
  already canonical.

The AI defaults to Workflow A, but switches to Workflow B if it detects Hagai
already wrote a note for the same meeting in the human vault (via path/title
match or memory_search).

### Promotion (AI vault → human vault)

Durable AI-vault content is promoted to a MR against the human vault.
**Reformulation is mandatory** — never paste AI-vault prose into a human-vault
MR. Before opening a MR the agent MUST:

1. Search the human vault: `memory_search.py --source human-vault "<topic>"` —
   find related notes
2. Consult `human-vault-map.md` to decide placement (extend existing note vs.
   create new)
3. Reformulate to match human-vault voice, density, naming, and link conventions
4. If overlap >70% with an existing note, propose a merge edit — not a new file
5. Include in the MR description: trigger, AI-vault source path, placement
   rationale, dedup-check result

See the `promote-to-human-vault` skill in Phase 4.5.

### Audit trail

The `ai-vault/` git history is the audit trail for SOUL/USER/MEMORY edits and
AI-vault writes. Raw event data (email threads, calendar events, session
summaries) lives in the cloud DB — not in git. Daily reflection (Phase 6) reads
`events WHERE type = 'session_summary'` and writes daily briefings to
`ai-vault/briefings/daily/`.

### SOUL.md starter content

```markdown
Agent Soul
==========

Identity
--------

I am Hagai's engineering second brain. I think like a Staff Engineer: I surface
risks early, flag blockers across teams, and prefer directness over
pleasantries.

Hard Rules (Advisor Mode)
-------------------------

- NEVER send emails, post messages, or act on behalf of Hagai without explicit
  approval.
- NEVER access financial data or make purchases.
- NEVER post to social media.
- I propose actions. Present for review. Wait.

Vault Boundaries
----------------

- AI vault (`~/second-brain/work/ai-vault/`): my working memory. Briefings,
  dossiers, drafts, analysis, logs, open questions, digested transcripts. I am
  sole writer. Hagai is read-only.
- Human vault (`~/notes/work/`, mirrored at `~/second-brain/work/human-vault/`):
  Hagai's curated knowledge. Read-only for me. Changes only via the
  `promote-to-human-vault` skill (MR through the AI's fork).

Vault Editorial Freedom (AI vault)
----------------------------------

The AI vault is mine to write freely. Goal: be maximally useful to Hagai. I
create folders, files, wikilinks, and structure as needed — no per-file approval
required. Use the standard subfolders (briefings/, dossiers/, meetings/,
drafts/, analysis/, logs/, questions/) and add new ones when a clear category
emerges.

Promotion Triggers (AI vault → human vault)
-------------------------------------------

Promote when:

- A meeting decision crystallizes (action taken, commitment made)
- A risk graduates from "watching" to "tracked"
- A pattern recurs across ≥3 events and is worth Hagai's long-term reference
- Hagai explicitly asks "save this"

Never promote: speculative analysis, draft reasoning, transient briefings,
operational logs.

Promotion Discipline
--------------------

Before opening a MR to the human vault, I MUST:

1. Search the human vault for related notes
   (`memory_search.py --source human-vault`)
2. Read `human-vault-map.md` and decide placement (extend existing vs. create
   new)
3. Reformulate to match human-vault voice — never paste AI-vault prose
4. If overlap >70% with an existing note, propose a merge edit, not a new file
5. Include in the MR description: trigger, AI-vault source path, placement
   rationale

Meeting Notes Workflow
----------------------

- Default to **Workflow A**: ingest transcript → write digest at
  `ai-vault/meetings/YYYY-MM-DD-name.md` → consider promotion if outcomes are
  durable.
- Switch to **Workflow B** (Hagai writes the note himself) if a matching note
  already exists in human-vault for the same meeting. Detection rule:
  `memory_search.py --source human-vault "<meeting-title>"` returns a result
  whose path contains the meeting date (YYYY-MM-DD) AND a hybrid score ≥0.6. In
  that case: add an analysis note in `ai-vault/analysis/` with a `[[wikilink]]`
  back to Hagai's note; do not duplicate his content.

Communication Style
-------------------

- Concise. Technical substance only.
- Flag risks with severity: [P0 blocker / P1 risk / P2 watch].
- Surface cross-team dependencies proactively.

Actions
-------

- I propose actions; Hagai accepts, rejects, or modifies them.
- Rejections ALWAYS carry a one-line note — the "why" is the learning signal
  that improves future proposals.
- Acceptance notes are optional but encouraged when something was non-obvious.

MEMORY.md Rules
---------------

- I am the sole writer of MEMORY.md. Sessions are read-only consumers.
- Each section has a hard cap (see config.toml). When full, evict by
  priority-desc then recency-desc. Never drop a higher-priority entry to make
  room for a lower-priority one.
- Every entry is a one-liner only. No rationale, no detail — detail belongs in
  AI-vault dossiers/analysis, durable knowledge in human vault via MR.
```

### USER.md starter content

```markdown
{{full_name}}'s Profile
=======================

- **Role:** {{role}}
- **Timezone:** {{timezone}}
- **Working hours:** {{working_hours}}

Accounts
--------

- Gmail: {{gmail_address}}
- Google Calendar: primary
- Discord user ID: {{discord_user_id_or_TODO}}
- GitLab: {{gitlab_upstream_repo_or_TODO}} (host: {{gitlab_host_or_TODO}})

Team Map
--------

<!-- Populate manually or via skill -->
<!-- Format: Name | Role | Timezone | Notes -->

Action Criteria
---------------

Propose an email-reply action when:

- Sender is a team lead, manager, or cross-team partner
- Email has been unread > 4 hours during working hours
- Subject contains: deadline, blocker, decision, review, urgent

Skip:

- Automated notifications, newsletters, CCs where I'm not the primary recipient
```

### MEMORY.md starter content

```markdown
Active Memory
=============

Active Projects
---------------

<!-- Max 7 bullets. One-liners only. Evict: priority-desc then recency-desc. -->

Open Risks
----------

<!-- Max 10 bullets. Format: [P0/P1/P2] Risk — owner -->

Key Decisions
-------------

<!-- Max 10 bullets. Format: YYYY-MM-DD | Decision -->

Team Context
------------

<!-- Max 20 bullets. Quick facts: ownership, timezones, OKRs. -->
```

### config.toml

All tunable parameters in one file. Scripts read this; never hardcode these
values.

```toml
[context]
# Max bytes for each auto-injected file. Enforced by check_context_size.py.
max_bytes.SOUL = 4096
max_bytes.USER = 6144
max_bytes.MEMORY = 8192

[memory]
# Hard caps per MEMORY.md section. No TTL — caps are the only enforcement.
max_active_projects = 7
max_open_risks = 10
max_key_decisions = 10
max_team_context = 20

[heartbeat]
interval_minutes = 30

[hooks]
# Bound on session_summary.payload size written by pre-compact / session-end
# hooks. Role-aware truncation: full user prompts, head+tail truncation for
# long assistant text, tool-use args truncated, tool_results dropped.
transcript_tail_max_bytes = 50000
assistant_text_head_bytes = 2048
assistant_text_tail_bytes = 4096
tool_args_summary_bytes = 256
```

### Context size tests

Add to `.config/test.toml`:

```toml
[[test_suite]]
name = "context_size_check"
command = "uv run --project ~/dotfiles/second-brain sb-context-check"
```

`check_context_size.py` reads `config.toml [context] max_bytes.*`, checks
`len(content.encode("utf-8"))` for each injected file, exits non-zero with a
clear message if any exceed their limit. No Claude, no network. Adding a new
injected file = add one entry to `config.toml` and it is automatically enforced.

### Database

The system uses two SQLite-compatible databases:

- **Cloud DB (source of truth) — Turso/libSQL.** Holds the event log and all
  projections. Accessed via libSQL embedded replica: a local SQLite file at
  `~/.local/share/second-brain/<instance>/cloud-replica.db` mirrors the cloud
  primary. Reads hit the local file (sub-ms); writes go to the primary and sync
  back. Per-instance subdir (`work`, `personal`) so multiple brain instances
  don't collide on the same replica. Connection in the instance's `.env`:

  ```
  # Turso (production)
  DATABASE_URL=libsql://<your-db>.turso.io?authToken=<token>&sync_url=file:///home/bayov/.local/share/second-brain/work/cloud-replica.db

  # Or fallback (local-only SQLite, no cloud)
  DATABASE_URL=file:///home/bayov/.local/share/second-brain/work/cloud.db
  ```

  Same `db.py` access layer in both modes; the URL prefix decides. Plain SQL
  only — no Turso-specific extensions in the schema, so a swap to plain SQLite
  (or another SQLite-family backend) requires no code changes.

- **Local RAG DB — plain SQLite.** Holds the vector + keyword search index at
  `~/.local/share/second-brain/<instance>/rag.db`. Pure cache, regenerable
  anytime via `memory_index.py`. See Phase 3.

#### Cloud DB schema

```sql
-- Append-only event log — the source of truth
CREATE TABLE events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,  -- monotonic; ID *is* the order
    source          TEXT NOT NULL,         -- 'gmail', 'calendar', 'session_compact', 'user', 'heartbeat'
    type            TEXT NOT NULL,         -- 'email_thread', 'calendar_event', 'action_proposed', ...
    source_id       TEXT,                  -- external dedupe key (gmail thread_id, etc.) — nullable
    occurred_at     TIMESTAMP NOT NULL,
    ingested_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    payload         TEXT NOT NULL,         -- JSON-as-text
    metadata        TEXT,                  -- JSON-as-text
    schema_version  INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX events_source_dedupe
    ON events(source, source_id) WHERE source_id IS NOT NULL;
CREATE INDEX events_type_occurred ON events(type, occurred_at);

-- Agents that produce events / actions. Lazy-registered on first use.
CREATE TABLE agents (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT NOT NULL,           -- 'heartbeat', 'claude-code-cli', 'pi-cli'
    harness       TEXT NOT NULL,           -- 'anthropic-sdk-python', 'claude-code', 'pi', ...
    model         TEXT NOT NULL,           -- 'claude-sonnet-4-6', 'claude-opus-4-7', ...
    version       TEXT,
    prompt_hash   TEXT,                    -- hash of system prompt + skills + config bundle
    first_seen    TIMESTAMP NOT NULL,
    last_seen     TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX agents_identity ON agents(harness, model, version, prompt_hash);

-- Actions proposed by agents, plus their responses (merged: latest response wins; full history in events)
CREATE TABLE actions (
    id              TEXT PRIMARY KEY,                 -- UUID; referenced from outside (Discord, logs)
    type            TEXT NOT NULL,                    -- 'email_draft', 'risk_alert', 'note_pr_proposal', ...
    created_at      TIMESTAMP NOT NULL,
    agent_id        INTEGER NOT NULL REFERENCES agents(id),
    source_event_id INTEGER REFERENCES events(id),
    content         TEXT NOT NULL,                    -- JSON, type-dependent

    -- Response fields (NULL until the user responds)
    status          TEXT,                             -- 'accepted' | 'rejected' | 'modified'
    response_notes  TEXT,
    responded_at    TIMESTAMP
);

CREATE INDEX actions_pending  ON actions(created_at DESC) WHERE status IS NULL;
CREATE INDEX actions_by_agent ON actions(agent_id, created_at DESC);

-- Which agent processed which event. Reprocessing with a new agent = new row, not update.
CREATE TABLE event_processing (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id        INTEGER NOT NULL REFERENCES events(id),
    agent_id        INTEGER NOT NULL REFERENCES agents(id),
    started_at      TIMESTAMP NOT NULL,
    completed_at    TIMESTAMP,
    status          TEXT NOT NULL,                    -- 'pending' | 'success' | 'failed'
    output_refs     TEXT,                             -- JSON: vault files written, MR URLs
    error           TEXT
);

CREATE UNIQUE INDEX event_processing_uniq ON event_processing(event_id, agent_id);

-- Notifications sent (replaces md5-hash diffing for dedupe)
CREATE TABLE notifications (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    channel         TEXT NOT NULL,                    -- 'discord_dm', 'notify-send'
    sent_at         TIMESTAMP NOT NULL,
    related_id      TEXT,                             -- actions.id, or NULL for general advisories
    related_type    TEXT,                             -- 'action' | 'advisory'
    message         TEXT NOT NULL,
    source_event_id INTEGER REFERENCES events(id)
);

CREATE INDEX notifications_related ON notifications(related_id, sent_at DESC);

-- Cursors for projection consumers (replaces file/seek_offset cursor)
CREATE TABLE projection_cursors (
    name           TEXT PRIMARY KEY,                  -- 'main_projector', etc.
    last_event_id  INTEGER NOT NULL DEFAULT 0,
    updated_at     TIMESTAMP NOT NULL
);

-- Convenience: completed actions = proposal + response in one row
CREATE VIEW completed_actions AS
    SELECT * FROM actions WHERE status IS NOT NULL;
```

#### Event types

Every fact in the system is an event:

| Event type           | Source                        | When written                                                                 |
| -------------------- | ----------------------------- | ---------------------------------------------------------------------------- |
| `email_thread`       | gmail                         | Heartbeat ingestion                                                          |
| `calendar_event`     | calendar                      | Heartbeat ingestion                                                          |
| `session_summary`    | session_compact / session_end | Hook on compact/stop                                                         |
| `action_proposed`    | heartbeat / agent             | Agent generates a proposal                                                   |
| `action_responded`   | user                          | Hagai accepts / rejects / modifies                                           |
| `event_processed`    | agent                         | After processing an input event                                              |
| `notification_sent`  | heartbeat / bot               | When a notification goes out                                                 |
| `human_vault_synced` | heartbeat                     | After `sync_human_vault()` runs (success or no-op) — feeds `db.py --status`  |
| `human_vault_commit` | human-vault                   | One per new commit observed by `sync_human_vault()` — drives change-reaction |

#### Writing events (sync projection in one transaction)

All writers go through a single helper that atomically inserts the event and
updates the relevant projection:

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
        if row is None:
            return None  # duplicate (source, source_id) — idempotent skip
        event_id = row[0]
        project_event(conn, event._replace(id=event_id))
        return event_id
```

`project_event(conn, event)` dispatches by `event.type`:

- `action_proposed` → `INSERT OR IGNORE INTO actions (...)`
- `action_responded` →
  `UPDATE actions SET status, response_notes, responded_at WHERE id = ?`
- `event_processed` → `INSERT INTO event_processing (...)`
- `notification_sent` → `INSERT INTO notifications (...)`
- `email_thread`, `calendar_event`, `session_summary` — no projection (raw
  inputs that agents read later)

#### Rebuilding projections (replay)

The same `project_event()` is reused for full rebuilds — e.g., after a schema
change, or to reprocess everything with a smarter agent. One implementation,
used at write-time and replay-time, no drift:

```python
def rebuild_projections(conn):
    with conn:
        for table in ["actions", "event_processing", "notifications"]:
            conn.execute(f"DELETE FROM {table}")
        conn.execute("UPDATE projection_cursors SET last_event_id = 0")
        for event in conn.execute("SELECT * FROM events ORDER BY id"):
            project_event(conn, event)
```

**Reprocessing with a new agent:** the new agent re-reads input events
(`email_thread`, etc.) and emits its own `action_proposed` events with new IDs.
Old `action_responded` events stay untouched as immutable historical learning
data — the new agent reads them as "here's how previous proposals landed"
context, but never tries to regenerate them (responses came from a human and
aren't reproducible).

#### Pending actions query

```sql
SELECT * FROM actions WHERE status IS NULL ORDER BY created_at DESC;
```

#### Agent identity (lazy registration)

At agent startup, the harness computes its
`(harness, model, version, prompt_hash)` tuple, runs
`INSERT OR IGNORE INTO agents`, and resolves the resulting `id` for the session.
New prompt = new `prompt_hash` = new row. This gives you the granularity to ask
later: "did SOUL.md v2 produce a higher action-acceptance rate than v1?"

**Dependencies:** None — this is the foundation.\
**Estimated complexity:** Low

---

Phase 2: Hooks (Context Persistence)
------------------------------------

**What to build:** Three Python hook scripts that inject memory into every
Claude conversation and write session events to the event log on compaction or
session end.

**Key files to create:**

```
~/dotfiles/second-brain/claude/hooks/
├── _setup.py                    # shared helper: paths, stdin parse, soft-fail wrapper
├── session-start-context.py
├── pre-compact-flush.py
└── session-end-flush.py

~/dotfiles/second-brain/starter/
└── claude-settings.json         # COPIED to <instance>/.claude/settings.json
```

**Hook lifecycle mapping.** `SessionStart` for inject; `PreCompact` and
`SessionEnd` for the two flush moments. **Not** `Stop` — `Stop` fires after
every assistant turn, which would write a `session_summary` event per response.
`SubagentStop` is also skipped: subagent transcripts are local to one task and
not session-level memory worth flushing.

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

`uv run --project ~/dotfiles/second-brain` resolves the project pyproject.toml
and managed venv (`~/dotfiles/second-brain/.venv`) without requiring activation.
The cwd at hook fire time is the instance dir (Claude Code's launch dir), so
relative paths inside the hook resolve against the instance.

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

`_setup.py` parses this once at import time and exposes `SESSION_ID`,
`TRANSCRIPT` (Path or None), `CWD`, `HOOK_EVENT` as module attributes. Hooks
import what they need.

**Path resolution in `_setup.py`.** The instance dir comes from
`find_instance()` — using the `cwd` field of the Claude-Code hook payload as the
starting point — and the package is imported normally (no `sys.path`
manipulation), since hooks run under `uv run --project
~/dotfiles/second-brain`
and `sb` is a regular installable package.

```python
# ~/dotfiles/second-brain/claude/hooks/_setup.py
import json, pathlib, sys, tomllib

from sb.instance import find_instance
from sb.scripts.db import write_event, Event  # re-exported for hooks

# stdin parse (gated for unit tests where stdin isn't a Claude Code payload)
_PAYLOAD = json.load(sys.stdin) if not sys.stdin.isatty() else {}
SESSION_ID = _PAYLOAD.get("session_id", "unknown")
TRANSCRIPT = (
    pathlib.Path(_PAYLOAD["transcript_path"])
    if "transcript_path" in _PAYLOAD
    else None
)
CWD = _PAYLOAD.get("cwd", str(pathlib.Path.cwd()))
HOOK_EVENT = _PAYLOAD.get("hook_event_name", "unknown")

# Claude Code launches in the instance dir, so `cwd` from the payload sits
# inside the instance — find_instance walks up from there to the marker.
INSTANCE_DIR = find_instance(pathlib.Path(CWD))

with (INSTANCE_DIR / "config.toml").open("rb") as f:
    config = tomllib.load(f)
```

`_setup.py` also wraps each hook's `main()` in soft-fail boilerplate (catch all
exceptions, write traceback to `~/.local/log/second-brain/<instance>/hooks.log`,
print a one-liner to stderr, exit 0 — never block Claude).

### `session-start-context.py` — inject memory

Reads `config.toml [context] max_bytes.*` and injects each listed file into the
system prompt, wrapped in XML-like tags. The inject list is config-driven (not
hardcoded) so the same `max_bytes` table that drives `check_context_size.py`
also drives what gets injected — single source of truth, no drift.

For Phase 2, the entries are `SOUL`, `USER`, `MEMORY`. Phase 4.5 adds `MAP`
(`human-vault-map.md`) by adding one config line; the hook picks it up
automatically.

Output format: a JSON object printed to stdout (Claude Code merges
`additionalContext` into the system prompt):

```json
{
    "hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": "<soul>\n{SOUL.md content}\n</soul>\n\n<user>\n{USER.md content}\n</user>\n\n<memory>\n{MEMORY.md content}\n</memory>"
    }
}
```

Tag name = filename stem, lowercase, kebab-case (so `human-vault-map.md` →
`<human-vault-map>`). Files are concatenated in `config.toml` declaration order.

`SessionStart` fires on `startup`, `resume`, `clear`, and `compact`. Behavior is
identical for all four — the hook re-reads the files from disk every time, so
resumed sessions automatically pick up MEMORY.md updates that landed during the
gap (e.g., from heartbeat or reflection). The hook does not branch on `source`.

### `pre-compact-flush.py` — flush before compaction

Reads `TRANSCRIPT` (the JSONL transcript from stdin's `transcript_path`),
distills it via the role-aware helper below, and writes one `session_summary`
event:

```python
write_event(
    Event(
        source="session_compact",
        type="session_summary",
        source_id=None,  # session events have no external dedupe key
        occurred_at=now_utc(),
        payload=distill_transcript(TRANSCRIPT, config["hooks"]),
        metadata={
            "session_id": SESSION_ID,
            "harness": "claude-code",
            "model": _PAYLOAD.get(
                "model"
            ),  # populated when Claude Code sends it
            "cwd": CWD,
        },
    )
)
```

The write goes to the local libSQL replica file, which syncs to the cloud
primary in the background — durable locally even if the network is down.
Heartbeat (Phase 6) reads these events later and updates MEMORY.md when
warranted.

### `session-end-flush.py` — flush at session end

Identical to `pre-compact-flush.py` except `source="session_end"`.

### Transcript distillation (`distill_transcript`)

Lives in `_setup.py` (or a helper module imported by it). Bounded role-aware
truncation — keeps signal, drops noise:

| Entry type         | Treatment                                                                                                                                                         |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| User text          | Full content. No truncation. (Short and signal-dense.)                                                                                                            |
| Assistant text     | If > head+tail bytes: keep first `assistant_text_head_bytes` (2048) + last `assistant_text_tail_bytes` (4096) joined by `[... N chars truncated ...]`. Else full. |
| Assistant tool_use | `{tool_name, args_summary}` where args is JSON-stringified and truncated to `tool_args_summary_bytes` (256).                                                      |
| User tool_result   | **Dropped.** The next assistant turn already distills what mattered; the raw JSONL on disk preserves it for forensics.                                            |

Walk the JSONL from the end backwards, accumulating entries until the serialized
payload approaches `transcript_tail_max_bytes` (50000), then stop. This means
tool-heavy sessions naturally get fewer turns; conversation-heavy sessions get
more — total payload size stays bounded regardless.

Payload shape:

```json
{
  "messages": [{"role": "user|assistant", "content": "...", "timestamp": "..."}, ...],
  "stats": {"message_count": 47, "first_message_at": "...", "last_message_at": "..."}
}
```

### Tests (`~/dotfiles/second-brain/tests/hooks/`)

```
conftest.py             # fixtures: temp DB (DATABASE_URL=file:///tmp/...),
                        # fake instance dir with SOUL/USER/MEMORY,
                        # sample transcript JSONL with mixed turns
test_distill.py         # role-aware truncation: long tool result truncated,
                        # payload ≤ 50KB, user prompts never truncated,
                        # head+tail shape correct
test_session_start.py   # stdin payload → captured stdout has hookSpecificOutput.additionalContext
                        # with <soul>/<user>/<memory> tags + full file contents
test_pre_compact.py     # stdin → exactly one row in events table,
                        # source=session_compact, source_id=NULL,
                        # payload + metadata shape correct
test_session_end.py     # same as pre_compact with source=session_end
```

Wired into `.config/test.toml`:

```toml
[[test_suite]]
name = "second_brain_hooks"
command = "uv run --project ~/dotfiles/second-brain pytest second-brain/tests/hooks/"
```

`pytest` lives in `pyproject.toml` `[dependency-groups] dev`.

**Dependencies:** Phase 1 (vault + cloud DB must exist; `db.py` exposes
`write_event` / `Event`)\
**Estimated complexity:** Medium

---

Phase 3: Memory Search (Hybrid RAG)
-----------------------------------

**What to build:** A local search pipeline that queries every vault present in
the instance by meaning (semantic) and keyword, merged for best results. Runs in
a **separate, local-only SQLite file**
(`~/.local/share/second-brain/<instance>/rag.db`) — no separate server, and
intentionally decoupled from the cloud DB so the heaviest data (embeddings)
doesn't bloat sync traffic. The RAG DB is a pure cache: rebuildable anytime by
re-running `memory_index.py`.

**Scope of this phase:** ship the indexer + search library + CLI. **No edits to
the Phase 2 hooks.** Hook integration is deferred — `session-start` has no query
and the right caller is a skill (Phase 5) or chat adapter (Phase 7) with
explicit intent. `memory_search.py` is importable Python _and_ a CLI; both
surfaces are first-class.

**Language scope:** English-only. The chosen embedding model
(`BAAI/bge-small-en-v1.5`) is English-only by design. Non-English content
(Hebrew names in calendar invites, mixed-language emails) will be normalized to
English via a translation step at _ingestion time_ in later phases — search
itself stays monolingual. FTS5's `unicode61` tokenizer still indexes non-Latin
script correctly, so any Hebrew that does sneak through is at least
keyword-findable.

**Key files to create:**

```
src/sb/scripts/
├── rag_db.py          # Local RAG DB connection + schema (chunks, vec_chunks, fts_chunks, indexed_files)
├── embeddings.py      # FastEmbed wrapper
├── memory_index.py    # Chunking + indexing pipeline (incremental, writes to rag.db)
└── memory_search.py   # Hybrid search (0.7 vector + 0.3 keyword) — reads from rag.db
```

**Library setup:** add `fastembed` (brings ONNX runtime) and `sqlite-vec`
(vector similarity for SQLite) to `~/dotfiles/second-brain/pyproject.toml`, then
`uv sync --project ~/dotfiles/second-brain`. FTS5 is built into Python's stdlib
`sqlite3` — no install needed.

### `rag_db.py` — schema

The RAG DB is a pure cache, so schema bumps **drop and rebuild** rather than
migrate. `PRAGMA user_version` is checked on every connection; mismatch with the
code constant triggers `DROP` of all tables, recreate, then a full re-index on
the next pass. No backwards-compat shims, ever.

```sql
-- One row per chunk. Source of truth; vec_chunks/fts_chunks key off id.
CREATE TABLE chunks (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    source_path  TEXT NOT NULL,        -- absolute path on disk
    source_tag   TEXT NOT NULL,        -- 'ai-vault' | 'human-vault'
    chunk_index  INTEGER NOT NULL,     -- 0-based order within the file
    char_start   INTEGER NOT NULL,     -- offset in original file (for citation)
    char_end     INTEGER NOT NULL,
    text         TEXT NOT NULL,        -- post-cleaning chunk body
    mtime_ns     INTEGER NOT NULL,     -- ns precision; fast-skip key
    file_hash    TEXT NOT NULL,        -- sha256 of file bytes; truth (mtime can lie after `git checkout`)
    embed_model  TEXT NOT NULL,        -- 'bge-small-en-v1.5' — detect model swaps for re-embed
    indexed_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX chunks_path_idx ON chunks(source_path, chunk_index);
CREATE INDEX chunks_tag_idx ON chunks(source_tag);

-- sqlite-vec virtual table — rowid = chunks.id (manual link, not FK)
CREATE VIRTUAL TABLE vec_chunks USING vec0(
    embedding float[384]
);

-- FTS5 contentless: writers supply (rowid, text); query with MATCH.
-- Saves ~30% storage vs. duplicated content; we always insert chunk + vec + fts
-- together in a single transaction so external-content triggers buy nothing.
CREATE VIRTUAL TABLE fts_chunks USING fts5(text, content='', tokenize='unicode61');

-- Bookkeeping for fast "skip unchanged" lookups (single PK hit instead of
-- scanning chunks for each file in every indexer pass).
CREATE TABLE indexed_files (
    source_path  TEXT PRIMARY KEY,
    source_tag   TEXT NOT NULL,
    mtime_ns     INTEGER NOT NULL,
    file_hash    TEXT NOT NULL,
    chunk_count  INTEGER NOT NULL,
    indexed_at   TIMESTAMP NOT NULL
);
```

`rag_db.py --init` is idempotent (`CREATE TABLE IF NOT EXISTS`) and is invoked
from `bootstrap.py`. Phase 1 ships it as a no-op stub; Phase 3 fills it in
without changing the bootstrap call site.

### `embeddings.py` — model setup

```python
from fastembed import TextEmbedding

# Downloads ~80MB ONNX model on first run, cached in ~/.cache/fastembed/
model = TextEmbedding(model_name="BAAI/bge-small-en-v1.5")  # 384 dims


def embed_batch(texts: list[str]) -> list[list[float]]:
    return [v.tolist() for v in model.embed(texts, batch_size=64)]


def embed_query(query: str) -> list[float]:
    return next(model.query_embed(query)).tolist()
```

### `memory_index.py` — vault discovery + incremental pipeline

Vaults are **discovered at runtime**, not configured. The indexer locates its
instance via `find_instance()` (cwd ascent, or `SECOND_BRAIN_INSTANCE`), then
scans for known vault subdirectories. Missing directories (human-vault before
Phase 4.5 lands the submodule) are silently skipped with one INFO log line — not
an error. Adding a new vault is a code change, not a config edit, because each
`source_tag` carries semantic implications (search filters, write authority,
promotion paths).

```python
from pathlib import Path

from sb.instance import find_instance

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

**File-scope rules** (applied during the walk):

- Extension allowlist: `.md` only.
- Path exclusions: `.obsidian/`, `.git/`, `.gitmodules/`, `.trash/`, `.claude/`
  (the hooks/skills symlinks point at dotfiles — we never want code or
  configuration embedded as memory).
- **Don't follow symlinks** during the walk.
- Skip files with <50 chars of post-frontmatter body (avoids embedding stubs).
- Always include `SOUL.md`, `USER.md`, `MEMORY.md`, `human-vault-map.md` at
  vault root.
- `logs/` and `briefings/daily/` are included — easier to filter at search time
  than to rebuild the index when you decide you wanted them.

**Skip logic (per file):**

1. Look up `(source_path)` in `indexed_files`.
2. If `mtime_ns` matches → skip (fast path, no I/O).
3. Else hash the file; if `file_hash` matches → update `mtime_ns` only (mtime
   lied; common after `git checkout` or `cp -p`).
4. Else re-chunk and re-embed.

**Frontmatter handling:** YAML frontmatter (`---\n...\n---` at file head) is
stripped before chunking and _not_ embedded. It's structured metadata; including
it pollutes embeddings (a `tags: [foo]` line otherwise matches every tagged
file). Future work may store the parsed dict in a `frontmatter_json` column.

**Chunking algorithm** (~400 tokens with 50-token overlap, paragraph-aware):

1. Tokenize the file with the **BGE tokenizer** (exposed via FastEmbed). Char
   heuristics drift on Hebrew/code/URLs and silently overflow the model's
   512-token limit.
2. Parse the file into "blocks": paragraphs (run of non-empty lines separated by
   ≥1 blank line), fenced code blocks, markdown tables, list items. Headings
   stay attached to the following block.
3. Greedy-pack blocks into chunks until adding the next block would exceed 400
   tokens.
4. **Atomic units that must not be split mid-block:** code blocks, tables, list
   items. The only place we split mid-block is when a _single_ block exceeds 400
   tokens — then a sliding token window cuts it.
5. **Overlap = last 50 tokens of previous chunk prepended to next chunk.** The
   overlap is duplicated text; `char_start`/`char_end` track only the
   non-overlapping core for citation accuracy.
6. **Heading context preservation:** when a chunk starts mid-section, prepend
   the nearest enclosing H1/H2/H3 trail (e.g. `> Project X > Risks >`) as a
   single line at chunk top. Big relevance win for queries like "risks for
   project X" against chunks that say "blocked on infra" without otherwise
   naming the project.
7. Wikilinks `[[Target]]` / `[[Target|alias]]` are kept verbatim — they're the
   strongest "this note connects to" signal embeddings can see.
8. Strip before embedding: HTML comments `<!-- ... -->`, image refs
   `![alt](url)` (keep `alt`), reference-style links `[text][ref]` → `text`.
   Bold/italic markers stay (not worth the cost).
9. Document-level cap: warn (don't fail) when a single file produces >100 chunks
   — usually indicates an accidentally-committed log dump.

**Per-file isolation:** wrap each file in `try/except`; on failure, log
`warn`-level with the path and continue. Indexer ends with summary counters
(`indexed=X skipped=Y errors=Z`) and exits non-zero only if `errors>0`. Cron
mails on non-empty stderr; systemd marks the unit failed. Monitoring stays
trivial.

**Reconcile deletes:** after the upsert pass, `reconcile_deletes()` lists every
`source_path` in `chunks` for the vault, diffs against the on-disk file set, and
`DELETE`s chunks (and matching `vec_chunks` / `fts_chunks` / `indexed_files`
rows) for files that no longer exist. Without this, renamed/deleted notes leave
orphan chunks that surface as ghost results in hybrid search.

**Concurrency safety:** the indexer takes an exclusive `flock` on `rag.db.lock`
at startup (`fcntl.flock(LOCK_EX | LOCK_NB)`). On contention it exits 0 with
`info: indexer already running, skipping` — the next cron tick picks up. Search
has no lock; SQLite handles read concurrency natively, and indexer writes are
short per-file transactions so reads stay responsive.

**CLI flags:**

```bash
python memory_index.py                          # All discovered vaults, full incremental pass
python memory_index.py --source human-vault     # One vault only (used by heartbeat after submodule sync)
python memory_index.py --verbose                # DEBUG-level logs
```

### `memory_search.py` — hybrid search

Importable Python (`from memory_search import search`) and a CLI. The Python
function is the primary contract; the CLI is a thin wrapper. Skills that call
search repeatedly within one invocation should `import` to share the loaded
embedding model (process cold-start dominates at ~2–4s for model load; warm
in-process queries are <100ms).

```python
@dataclass
class SearchHit:
    source_path: str
    source_tag: str  # 'ai-vault' | 'human-vault'
    chunk_index: int
    score: float  # fused, [0, 1]
    vector_score: float  # for debugging weight tuning
    bm25_score: float
    snippet: str  # ~200 chars centered on best match
    text: str  # full chunk for callers that want it


def search(
    query: str,
    source: str | None = None,
    limit: int = 5,
) -> list[SearchHit]: ...
```

**Hybrid scoring algorithm (union-then-rescore):**

1. Take top-50 from each side independently (vector via sqlite-vec, BM25 via
   FTS5 `MATCH`).
2. Union the candidate sets. For any chunk missing from one side's top-50, fetch
   its actual score on that side rather than substituting a default — keeps
   fusion honest when a chunk dominates one side and is rank ~200 on the other.
3. **Vector normalization:** sqlite-vec returns L2 distance; convert to
   similarity via `1 / (1 + distance)`. Already in `[0, 1]`. **No per-query
   renormalization** — absolute similarity is meaningful.
4. **BM25 normalization:** FTS5's `bm25()` is unbounded and negative (lower =
   better). Min-max within the candidate set, then invert so higher is better.
   The asymmetry — vector absolute, BM25 relative — is intentional: BM25 has no
   zero point; cosine does.
5. Fuse: `score = vector_weight * vec + (1 - vector_weight) * bm25`. Weights
   come from `config.toml [search]` (default `vector_weight = 0.7`).
6. Sort descending; tie-break on lower `chunk_index`, then more recent
   `mtime_ns`.
7. **Adjacent-chunk dedup:** when two results have the same `source_path` and
   `|chunk_index_a − chunk_index_b| ≤ 1`, drop the lower-scored one. The
   50-token overlap otherwise surfaces both halves of a single passage.
8. Return top-`limit`.

**Snippet generation:** for BM25-dominant hits, use FTS5's `snippet()` to
highlight the matched term. For vector-dominant hits with no FTS hit in the
candidate set, return the chunk's first 200 chars. Cheap heuristic; good enough.

**CLI:**

```bash
python memory_search.py "cross-team blockers" --limit 5
python memory_search.py "..." --source ai-vault     # filter by vault
python memory_search.py "..." --json                # emits list[SearchHit] as JSON for skill/chat consumers
```

Default output is human-readable (path, score, snippet); `--json` is the
contract for programmatic callers. Empty query or no candidates returns `[]` and
exits 0 — not an error.

### Operational notes

- **Cron line, per instance.** Bootstrap _prints_ the line and exits — it does
  not modify the user's crontab. Phase 9 will add a systemd user-timer
  alternative; for Phase 3, cron is canonical.

  ```cron
  */15 * * * * cd ~/second-brain/<instance> && uv run --project ~/dotfiles/second-brain sb-index
  ```

- **First-run cost:** ~80MB ONNX download to `~/.cache/fastembed/` on the first
  `embed_batch()` call, plus a full corpus embed. For a fresh `ai-vault` (only
  SOUL/USER/MEMORY/map) that's <30 chunks — trivial. The 15-min cadence absorbs
  it. `bootstrap.py`'s final printout mentions the one-time download so the
  operator isn't surprised.

- **Logging.** Stdlib `logging` to stderr; INFO by default, DEBUG via
  `--verbose`. Dynamic data goes through `extra={...}`, not into the message
  string — mirrors the structured-logging rule from the project guide so a
  future log shipper can parse cleanly.

- **`config.toml [search]`:**

  ```toml
  [search]
  vector_weight = 0.7 # bm25_weight = 1 - vector_weight
  candidate_k = 50 # top-K per side before fusion
  ```

**Dependencies:** Phase 1 (vault), Phase 2 (hooks coexist; no edits required in
Phase 3 — integration deferred to Phase 5+ skills and Phase 7 chat adapters)\
**Estimated complexity:** Medium

---

Phase 4: Integrations (Gmail → Google Calendar → Discord)
---------------------------------------------------------

**What to build:** Python modules for each platform exposed via a unified
`query.py` CLI. Claude never sees API tokens — Python handles auth and passes
only typed data. Heartbeat ingestion is **not** part of Phase 4 (Phase 6) and
the real `sanitize_external_text` is **not** part of Phase 4 (Phase 8 — Phase 4
ships a no-op stub).

**File structure:**

```
src/sb/scripts/integrations/
├── registry.py              # Reads [integrations] enabled from config.toml
├── query.py                 # Unified CLI: query.py gmail list [--format=text|json]
├── shared_auth.py           # Google OAuth2 (Gmail + Calendar) + retry helper
├── secrets.py               # Loads ~/.local/share/second-brain/<instance>/secrets.toml
├── errors.py                # IntegrationAuthError, IntegrationTransientError, MessageTooLong
├── sanitize.py              # Phase 4 no-op stub; Phase 8 fills in
├── gmail.py
├── calendar.py
├── discord_client.py        # REST-only via httpx (no discord.py in Phase 4)
└── integration_template.py
```

**Dependencies added to `~/dotfiles/second-brain/pyproject.toml`:**

```
google-auth
google-auth-oauthlib
google-api-python-client
httpx
tenacity
# tomllib is stdlib (Python 3.11+) — no dependency needed.
```

`discord.py` is **not** added in Phase 4; it enters in Phase 7 when an inbound
gateway connection is needed.

---

### Configuration

**Repo-level (checked in):** `~/dotfiles/second-brain/config.toml` gains:

```toml
[integrations]
# User opts in explicitly. Missing creds for an enabled integration → fail-fast.
enabled = ["gmail", "calendar", "discord"]

[integrations.gmail]
# Default filter combines Gmail's own importance signal + the user's stars,
# narrowed to the Primary inbox to drop Promotions/Social/Updates/Forums.
filter = "(is:important OR is:starred) category:primary is:unread"
max_results = 20

[integrations.calendar]
# "all" expands to calendarList().list() minus declined calendars.
calendar_ids = ["all"]
days_ahead = 7
# Per-user vocabulary — Hebrew speakers may want "מועד אחרון", etc.
deadline_keywords = ["deadline", "due", "review", "launch", "release"]
```

**Machine-local (never in VCS):**
`~/.local/share/second-brain/<instance>/secrets.toml`. TOML, not env. Loaded via
stdlib `tomllib` at entrypoints (`query.py`, `shared_auth.py` standalone, the
Phase 6 heartbeat). Library modules read from a process-level `Secrets`
dataclass — no side-effecting imports. The repo ships `secrets.example.toml`.

```toml
[google]
credentials_path = "~/.local/share/second-brain/<instance>/credentials.json"
token_path = "~/.local/share/second-brain/<instance>/token.json"

[discord]
bot_token = "..."
user_id = "..."
```

Phase 9's systemd unit invokes the Python entrypoint directly; the entrypoint
reads `secrets.toml` itself. No `EnvironmentFile=`, no env-format duplication.

---

### Error model — `errors.py`

```python
class IntegrationError(Exception): ...


class IntegrationAuthError(IntegrationError):
    """Permanent auth failure: RefreshError, 401, 403. Surface to user."""


class IntegrationTransientError(IntegrationError):
    """Retryable failure that exhausted retries: 429, 5xx, network."""


class MessageTooLong(IntegrationError):
    def __init__(self, length: int, limit: int):
        self.length, self.limit = length, limit
        super().__init__(f"message length {length} exceeds {limit}-char cap")
```

`shared_auth.py` exposes a `_retry` helper (used by every integration call):
`tenacity` with 3 tries, exponential backoff (1s/2s/4s), retrying only on
`HttpError` 429/5xx and `httpx.TransportError`. On final failure raises
`IntegrationTransientError`. Auth-class failures
(`google.auth.exceptions.
RefreshError`, `HttpError` 401/403, Discord 401) are
mapped to `IntegrationAuthError` — distinct so the heartbeat (Phase 6) can route
them differently: auth failures → Discord notification to the user; transient
failures → silent skip + `warn!` log.

Errors are isolated **per integration**: one broken integration does not crash
the heartbeat — it returns an empty list for that cycle.

---

### Integration 1: Gmail

**Auth setup:**

1. Google Cloud Console → Create project → Enable Gmail API.
2. OAuth consent screen → Add your Gmail address as test user.
3. **Move the consent screen to "In production"** as soon as the app verifies
   itself — in "Testing" mode Google expires refresh tokens after 7 days, which
   will silently break the heartbeat every week. (If you keep it in Testing,
   accept the weekly re-auth.)
4. Create OAuth 2.0 credentials → Desktop app → set the authorized redirect URI
   to **exactly** `http://localhost:8765/` → download `credentials.json` to
   `~/.local/share/second-brain/<instance>/credentials.json`.
5. On first run:
   `uv run --project ~/dotfiles/second-brain python shared_auth.py` opens a
   browser via `InstalledAppFlow.run_local_server(port=8765)` → authenticate →
   `token.json` is saved next to `credentials.json`.

   **WSL2 note:** Win11 + WSLg forwards `localhost` automatically. On older
   Windows builds, run once:
   `netsh interface portproxy add v4tov4 listenport=8765 listenaddress=127.0.0.1 connectport=8765 connectaddress=$(wsl hostname -I)`.
   Mac/Linux: works as-is.

**Required scopes:**

- `https://www.googleapis.com/auth/gmail.readonly`
- `https://www.googleapis.com/auth/gmail.compose` (only if the agent should
  create Gmail drafts)

**`gmail.py` — unit is the thread, not the message:**

```python
from dataclasses import dataclass
from datetime import datetime
from googleapiclient.discovery import build

from shared_auth import get_google_credentials, retry
from sanitize import sanitize_external_text


@dataclass
class EmailSummary:
    """One thread, identified by (thread_id, latest_message_id) for idempotency."""

    thread_id: str
    latest_message_id: str
    source_id: str  # f"{thread_id}:{latest_message_id}" — stable + version-bumps when thread grows
    subject: str
    sender: str
    snippet: str
    received_at: datetime


@dataclass
class ThreadContext:
    thread_id: str
    messages: list["EmailSummary"]
    full_bodies: list[str]  # already sanitized


def list_important_unread(
    filter_query: str, max_results: int = 20
) -> list[EmailSummary]:
    """Returns one EmailSummary per matching thread.

    `filter_query` comes from config.toml [integrations.gmail].filter. Default:
    "(is:important OR is:starred) category:primary is:unread".
    """
    service = build("gmail", "v1", credentials=get_google_credentials())
    threads = retry(
        lambda: (
            service.users()
            .threads()
            .list(
                userId="me",
                q=filter_query,
                maxResults=max_results,
            )
            .execute()
        )
    ).get("threads", [])
    # ... fetch latest message per thread, build EmailSummary list
    ...


def get_thread(thread_id: str) -> ThreadContext:
    """Full thread for action-generation context."""
    ...
```

**Why per-thread, versioned `source_id`:** a thread is the right cognitive unit
("one back-and-forth to act on"). Combining `thread_id` with `latest_message_id`
keeps the event log idempotent on repeat ingestion _and_ re-surfaces the thread
when it grows. Cheap — `threads().list()` returns the current `historyId` and
last-message id without an extra round-trip.

**Rate limits:** Gmail API: 250 quota units/second, 1 billion/day. At 20
threads/heartbeat every 30 min: ~200 units/hour — well within limits.

**`query.py` subcommands:**

```bash
python query.py gmail list                         # Unread important threads
python query.py gmail thread <thread_id>           # Full thread for context
python query.py gmail list --format=json           # For programmatic use / tests
```

---

### Integration 2: Google Calendar

**Auth:** Shared OAuth2 token from `shared_auth.py`. Add scope:

- `https://www.googleapis.com/auth/calendar.readonly`

**`calendar.py` — key functions:**

```python
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from googleapiclient.discovery import build

from shared_auth import get_google_credentials, retry


@dataclass
class Event:
    id: str
    calendar_id: str
    summary: str
    start: datetime  # UTC
    end: datetime  # UTC
    location: str | None
    attendees: list[str]
    url: str


def list_upcoming_events(
    calendar_ids: list[str], days_ahead: int, max_results: int = 20
) -> list[Event]:
    """If calendar_ids == ["all"], expands to calendarList().list() minus declined."""
    ...


def list_events_with_deadlines(
    calendar_ids: list[str], deadline_keywords: list[str]
) -> list[Event]:
    """Events whose summary contains any keyword (case-insensitive)."""
    ...
```

**Israel timezone note:** Always store and compare times in UTC. Display in
`Asia/Jerusalem` using `zoneinfo` (Python 3.9+).

**`query.py` subcommands:**

```bash
python query.py calendar upcoming        # Next [days_ahead] days, all enabled calendars
python query.py calendar deadlines       # Events matching deadline_keywords
```

---

### Integration 3: Discord (outbound only — Phase 7 owns inbound)

**Auth setup:**

1. Discord Developer Portal → New Application → Bot → Enable bot.
2. Permissions: Send Messages (DMs only). Phase 4 is outbound-only — **no
   "Message Content Intent" needed**, no privileged intents at all.
3. Copy Bot Token → `secrets.toml [discord] bot_token`.
4. Find your Discord user ID: Settings → Advanced → Developer Mode → right-click
   your name → Copy ID → `secrets.toml [discord] user_id`.

**No `discord.py` dependency.** Outbound DMs are two REST calls:

```python
# discord_client.py — REST-only via httpx
import httpx

from errors import IntegrationAuthError, MessageTooLong
from secrets import load_secrets
from shared_auth import retry


DM_LIMIT = 2000  # Discord hard cap


def notify(message: str) -> None:
    """Send `message` as a DM. Single-message primitive — no chunking.

    Raises:
        MessageTooLong: if len(message) > 2000. Caller (agent or orchestrator)
            decides how to split.
        IntegrationAuthError: 401 from Discord (bad/revoked bot token).
        IntegrationTransientError: retries exhausted on 429/5xx.
    """
    if len(message) > DM_LIMIT:
        raise MessageTooLong(len(message), DM_LIMIT)

    s = load_secrets()
    headers = {"Authorization": f"Bot {s.discord.bot_token}"}

    # 1. Open (or fetch existing) DM channel
    channel = retry(
        lambda: httpx.post(
            "https://discord.com/api/v10/users/@me/channels",
            headers=headers,
            json={"recipient_id": s.discord.user_id},
        )
    )
    channel_id = channel.json()["id"]

    # 2. Send the message
    retry(
        lambda: httpx.post(
            f"https://discord.com/api/v10/channels/{channel_id}/messages",
            headers=headers,
            json={"content": message},
        )
    )
```

**Why REST-only:** outbound DMs do not need the gateway / WebSocket. The
`discord.py` library forces an `asyncio.Client` lifecycle plus intents config
that are pure overhead for a fire-and-forget DM. Phase 7 introduces the gateway
client when inbound chat lands.

**`query.py` subcommand:**

```bash
python query.py discord notify "Action ready: reply to Alice re: Q3 launch"
# Exit code 2 + stderr "Discord DMs cap at 2000 chars; got <n>. Split and retry."
# on MessageTooLong, so a calling agent has what it needs to recover.
```

---

### `query.py` — output format

Two formatters, one shared per output shape:

- **`--format=text`** (default): one block per item with stable field labels
  (`id:`, `from:`, `subject:`, `snippet:`). Skimmable for humans and parseable
  by Claude when invoked from a skill via Bash.
- **`--format=json`**: emits `[asdict(item), ...]`. For tests and any
  programmatic caller.

Tests assert against `--json`; manual debugging uses `text`.

---

### Adding a new integration — `integration_template.py`

Copy this file when wiring up a new platform (Linear, Jira, Slack, Notion,
etc.). It encodes the contract every integration must satisfy: auth stays in
Python, Claude only ever sees typed data, external text is sanitized at the
boundary, and errors are mapped to the shared taxonomy.

```python
# integration_template.py — copy and rename when adding a new integration.
"""
Pattern every integration follows:
  1. Auth via shared module or secrets.toml. Claude never sees credentials.
  2. Return typed values (dataclasses defined inline) — not raw SDK responses.
  3. Sanitize any free-text field reaching Claude (Phase 4 stub; Phase 8 real).
  4. One bounded list_*() call per heartbeat — keeps API + token budgets predictable.
  5. Map errors to errors.IntegrationAuthError / IntegrationTransientError.
  6. Register in registry.py and add a query.py subcommand.
"""

from dataclasses import dataclass
from datetime import datetime

from errors import IntegrationAuthError
from sanitize import (
    sanitize_external_text,
)  # Phase 4: no-op stub. Phase 8: real.
from secrets import load_secrets
from shared_auth import retry


@dataclass
class FooItem:
    id: str
    title: str
    body: str  # already sanitized at construction time
    occurred_at: datetime
    url: str


def _client():
    # Build the authenticated client here. Read credentials from load_secrets();
    # never accept them as arguments and never log them.
    s = load_secrets()
    return build_foo_client(s.foo.token)


def list_recent(max_results: int = 20) -> list[FooItem]:
    """Bounded listing for the heartbeat. Read-only."""
    raw = retry(lambda: _client().items.list(limit=max_results))
    return [
        FooItem(
            id=r["id"],
            title=sanitize_external_text(r["title"], source="foo"),
            body=sanitize_external_text(r["body"], source="foo"),
            occurred_at=datetime.fromisoformat(r["occurred_at"]),
            url=r["url"],
        )
        for r in raw
    ]


def get_detail(item_id: str) -> FooItem:
    """Full single-item fetch for action-generation context."""
    ...
```

**Register in `registry.py`** — the registry reads
`config.toml
[integrations].enabled` and resolves each name to a module path.
Missing credentials for an enabled integration → fail-fast with a clear error.

```python
# registry.py
import tomllib
from importlib import import_module
from pathlib import Path

_MODULES = {
    "gmail": "integrations.gmail",
    "calendar": "integrations.calendar",
    "discord": "integrations.discord_client",
    # "foo":    "integrations.foo",
}


def enabled_modules() -> dict[str, object]:
    cfg = tomllib.loads(
        Path("~/dotfiles/second-brain/config.toml").expanduser().read_text()
    )
    return {
        name: import_module(_MODULES[name])
        for name in cfg["integrations"]["enabled"]
    }
```

**Add a `query.py` subcommand:**

```bash
python query.py foo list           # → calls foo.list_recent()
python query.py foo detail <id>    # → calls foo.get_detail(id)
```

**Heartbeat ingestion (Phase 6 pattern — not built in Phase 4):**

```python
for item in foo.list_recent():
    write_event(
        Event(
            source="foo",
            type="foo_item",
            source_id=item.id,  # idempotent — re-ingesting is a no-op
            occurred_at=item.occurred_at,
            payload=asdict(item),
        )
    )
```

If a new integration needs to break this contract (e.g., it cannot return typed
data, or auth must flow through Claude), the contract is wrong — fix the
template, not the integration.

---

### Tests

CLAUDE.md mandates "no network" in the default suite. Phase 4 tests mock at the
SDK boundary:

- `gmail.tests.py` — patch `googleapiclient.discovery.build`; assert
  `EmailSummary` shape, `source_id` versioning, error mapping.
- `calendar.tests.py` — patch `build`; assert `Event` shape, "all"-calendar
  expansion, deadline keyword filter.
- `discord_client.tests.py` — patch `httpx.post`; assert two-call flow,
  `MessageTooLong` raised at 2001 chars, `IntegrationAuthError` on 401.
- `query.tests.py` — assert text and JSON formatters on golden inputs.
- `shared_auth.tests.py` — assert `retry` retries on 429/5xx and propagates on
  other errors; `IntegrationAuthError` mapping for `RefreshError` / 401.

Live tests are **not** in the default suite. A separate
`tests/integration_live/` directory holds opt-in tests that hit real APIs when
credentials are present (skip otherwise) — run manually, never in CI.

---

### Phase 4 acceptance — what's "done"

- `~/dotfiles/second-brain/src/sb/scripts/integrations/` has every file in the
  structure above, including `sanitize.py` (no-op stub) and `errors.py`.
- `secrets.example.toml` checked in at the repo root.
- `config.toml` gains `[integrations]`, `[integrations.gmail]`, and
  `[integrations.calendar]` sections with the documented defaults.
- `pyproject.toml` adds `google-auth`, `google-auth-oauthlib`,
  `google-api-python-client`, `httpx`, `tenacity`. **Not** `discord.py`.
- Bootstrap docs cover: Google Cloud Console steps, OAuth "In production"
  warning (7-day refresh expiry footgun), Discord bot setup with no privileged
  intents, fixed-port `8765` redirect URI, WSL2 portproxy fallback.
- Default test suite (`uv run dev/test.py`) passes with all integration tests
  SDK-mocked; no network access.
- Manual smoke (one-time, off CI): `python query.py gmail list`,
  `... calendar upcoming`, `... discord notify "hello"` succeed against real
  accounts.

**Phase 4 explicitly does not include:** heartbeat ingestion (Phase 6), the real
`sanitize_external_text` (Phase 8), or Discord inbound / gateway client (Phase
7).

---

**Dependencies:** Phase 1 (config.toml, secrets layout under
`~/.local/share/second-brain/<instance>/`).\
**Estimated complexity:** Medium per integration (Gmail + Calendar share OAuth
setup; Discord is a few REST calls).

---

Phase 4.5: Human Vault Integration (Submodule + Promotion via MR)
-----------------------------------------------------------------

**What to build:** Mount the GitLab repo backing `~/notes/work/` as a git
submodule inside the project (`~/second-brain/work/human-vault/`) so the agent
has read access, and a `promote-to-human-vault` skill that proposes durable
knowledge from the AI vault to the human vault via MR — with mandatory
reformulation and dedup checks. The two vaults stay fully independent: content
moves AI → human only via MR, never via direct copy.

**GitLab instance:** Hagai's employer hosts GitLab on `${GITLAB_HOST}` (e.g.,
`gitlab.company.com`). All `glab` commands and SSH URLs below use that host.
Substitute `gitlab.com` if running on GitLab SaaS. **Verify with your employer**
before pushing personal notes to a corporate GitLab — corporate GitLabs are
often subject to admin visibility, retention policies, and DLP scanning. If
that's a concern, host the human-vault on GitLab SaaS in a private personal
namespace and keep the corporate instance for work code only.

**Fork-vs-single-repo trade-off:** The default flow uses an agent fork as the
security boundary (agent token can only write to the fork; merges into upstream
require Hagai's review). If your corporate GitLab forbids forks outside a group
or doesn't allow a separate "agent" account, fall back to **single-repo mode**:
one repo with branch protection on `main`; the agent pushes feature branches and
opens MRs against the same repo. The MR review still gates promotion. Pick one
and stay consistent — the security model differs.

**Vault paths:**

- Hagai's working copy (Obsidian, source of truth): `~/notes/work/` (a git repo
  with a GitLab remote)
- AI's read mirror (submodule of fork): `~/second-brain/work/human-vault/`

**Key files to create / modify:**

```
~/second-brain/work/human-vault/                  # git submodule → AI's fork
src/sb/scripts/memory_index.py                    # Already dual-vault (Phase 3)
claude/skills/promote-to-human-vault/             # symlinked into <instance>/.claude/skills/
├── SKILL.md
└── scripts/
    ├── promote.py                                # Search + reformulate + branch + MR
    └── update_vault_map.py                       # Refresh ai-vault/human-vault-map.md
```

**One-time GitLab setup:**

Install the `glab` CLI (`https://gitlab.com/gitlab-org/cli`). For self-hosted
instances, run `glab auth login --hostname gitlab.company.com` once.

1. **Push `~/notes/work` to GitLab** — Hagai initializes the canonical repo
   (private, in his personal namespace):
   ```bash
   glab repo create hagai/work-notes --private
   git -C ~/notes/work remote add origin git@gitlab.company.com:hagai/work-notes.git
   git -C ~/notes/work push -u origin main
   ```

2. **Fork it** under the agent's identity (assumes a separate `hagai-agent`
   GitLab user, or a sub-namespace Hagai owns):
   ```bash
   glab repo fork hagai/work-notes --clone=false
   # The fork is the hard security boundary — agent token has write access to fork only.
   # If your employer's GitLab forbids forking outside a group, use single-repo mode (see above).
   ```

3. **Personal Access Token (project-scoped where possible)** — fork only:
   - GitLab token type: **Project Access Token** on the fork
     (`hagai-agent/work-notes`) is preferred over a user PAT — it's scoped to
     one project, can't accidentally grant write to other repos.
   - If your GitLab tier doesn't expose Project Access Tokens (Free SaaS limits
     this), fall back to a Personal Access Token but expect to audit it more
     carefully.
   - Scopes: `api`, `read_repository`, `write_repository` (the `api` scope is
     required for `glab mr create`).
   - Add to `.env`:
     ```
     GITLAB_HOST=gitlab.company.com
     GITLAB_TOKEN=glpat-...
     HUMAN_VAULT_UPSTREAM=hagai/work-notes
     HUMAN_VAULT_FORK=hagai-agent/work-notes
     ```

4. **Add the fork as a submodule** under the project repo:
   ```bash
   cd ~/second-brain/work
   git submodule add git@gitlab.company.com:hagai-agent/work-notes.git human-vault
   git -C human-vault remote add upstream git@gitlab.company.com:hagai/work-notes.git
   git commit -m "Add human-vault submodule"

   # The heartbeat advances human-vault past the parent's pinned commit.
   # Tell git to ignore that drift so it doesn't pollute `git status` in the project repo.
   git config -f .gitmodules submodule.human-vault.ignore all
   git config submodule.human-vault.ignore all
   ```

   The submodule pointer is **not** pinned in practice — the heartbeat keeps the
   submodule's branch on `main` floating with upstream. Avoid running
   `git submodule update` after the initial clone; let `sync_human_vault()`
   manage it.

5. **Set the AI's git identity** for `ai-vault/` and `human-vault/` commits so
   the audit trail doesn't pick up Hagai's identity:
   ```bash
   git -C ~/second-brain/work config user.name  "second-brain-agent"
   git -C ~/second-brain/work config user.email "agent@second-brain.local"
   git -C ~/second-brain/work/human-vault config user.name  "second-brain-agent"
   git -C ~/second-brain/work/human-vault config user.email "agent@second-brain.local"
   ```

6. **SSH key for the agent** (used by `git push origin <branch>` in
   `promote.py`). Generate a dedicated key and register its public half on the
   agent's GitLab account (preferred) or as a deploy key on the fork:
   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/second_brain_agent -C "second-brain-agent"
   # Add to ~/.ssh/config so the submodule's git@gitlab.company.com URL picks it up:
   #   Host gitlab.company.com
   #     HostName gitlab.company.com
   #     IdentityFile ~/.ssh/second_brain_agent
   #     IdentitiesOnly yes
   ```
   Then paste `~/.ssh/second_brain_agent.pub` into GitLab → User Settings → SSH
   Keys (or Project → Settings → Repository → Deploy Keys with "Grant write
   permissions"). The token in `.env` is for `glab mr create` only; git
   transport uses SSH.

**Keep the submodule current — add to heartbeat pre-gather:**

```python
HUMAN_VAULT_PATH = Path.home() / "second-brain/work/human-vault"


def sync_human_vault() -> bool:
    """Fast-forward human-vault to upstream/main. For each new commit, write
    one `human_vault_commit` event so the heartbeat reasoning phase can react
    to Hagai's note changes in the same cycle. Returns True if HEAD advanced."""
    head_before = subprocess.check_output(
        ["git", "-C", str(HUMAN_VAULT_PATH), "rev-parse", "HEAD"], text=True
    ).strip()
    for cmd in [
        ["git", "-C", str(HUMAN_VAULT_PATH), "fetch", "upstream"],
        [
            "git",
            "-C",
            str(HUMAN_VAULT_PATH),
            "merge",
            "--ff-only",
            "upstream/main",
        ],
        ["git", "-C", str(HUMAN_VAULT_PATH), "push", "origin", "main"],
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
        # One event per commit, oldest-first so event IDs preserve commit order.
        # Payload is metadata only — full file content lives on disk in
        # human-vault/ and is read on demand via `git show <sha>:<path>` (or
        # the current working-tree file) during reasoning. The vault is the
        # source of truth; we don't duplicate diffs into the DB.
        # %x00 = NUL separator, survives any character in the subject line.
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
                # `A path`, `M path`, `D path`, or `R100 old new` for renames.
                parts = entry.split("\t")
                status = parts[0]
                path = parts[-1]
                files.append({"status": status, "path": path})
            write_event(
                Event(
                    source="human-vault",
                    type="human_vault_commit",
                    source_id=sha,  # UNIQUE(source, source_id) → idempotent re-sync
                    occurred_at=iso_time,
                    payload={"sha": sha, "subject": subject, "files": files},
                )
            )

    write_event(
        Event(
            source="heartbeat",
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

The `human_vault_synced` event is the system-health signal Phase 9's
`db.py --status` reads to surface "last successful submodule sync" without
grepping logs. The `human_vault_commit` events are the _content_ signal —
consumed by the heartbeat reasoning phase to react to Hagai's note changes (see
Phase 6).

**`promote.py` — reformulate-then-MR (the only writer to human vault):**

```python
HUMAN_VAULT_PATH = Path.home() / "second-brain/work/human-vault"
UPSTREAM_REPO = os.environ["HUMAN_VAULT_UPSTREAM"]  # e.g. "hagai/work-notes"
FORK_REPO = os.environ["HUMAN_VAULT_FORK"]  # e.g. "hagai-agent/work-notes"
GITLAB_TOKEN = os.environ["GITLAB_TOKEN"]
GITLAB_HOST = os.environ.get("GITLAB_HOST", "gitlab.com")


def promote(
    ai_vault_source: str,  # path inside ai-vault/ that triggered promotion
    target_file: str,  # path inside human-vault/ (existing or new)
    reformulated_content: str,  # already reformulated to human-vault voice
    trigger: str,  # decision crystallized | risk tracked | pattern recurred | user-request
    placement_rationale: str,  # why this file/section
    dedup_result: str,  # "no overlap" | "merged with <path>" | "extended <section>"
):
    slug = trigger.lower().replace(" ", "-")[:40]
    branch = f"ai/promote-{date.today()}-{slug}"

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
    # Push to the fork (origin). Submodule remote is SSH (see Phase 4.5 setup),
    # so this uses the agent's SSH key — no token in env. If you switch the
    # remote to HTTPS, embed the token via a credential helper instead of argv.
    subprocess.run(
        ["git", "-C", HUMAN_VAULT_PATH, "push", "origin", branch],
        check=True,
    )
    body = (
        f"**Proposed by:** Second Brain agent\n"
        f"**Trigger:** {trigger}\n"
        f"**AI-vault source:** `{ai_vault_source}`\n"
        f"**Placement:** `{target_file}` — {placement_rationale}\n"
        f"**Dedup check:** {dedup_result}\n\n"
        f"Review the diff, merge if useful, close if not. "
        f"Rejections with a one-line note improve future proposals."
    )
    # Cross-fork MR: --source-project is the fork, --target-project is upstream.
    # `glab` reads GITLAB_HOST and GITLAB_TOKEN from the environment.
    subprocess.run(
        [
            "glab",
            "mr",
            "create",
            "--source-project",
            FORK_REPO,
            "--target-project",
            UPSTREAM_REPO,
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
            "GITLAB_HOST": GITLAB_HOST,
            "GITLAB_TOKEN": GITLAB_TOKEN,
        },
        check=True,
    )

    subprocess.run(
        ["git", "-C", HUMAN_VAULT_PATH, "checkout", "main"], check=True
    )
```

In single-repo mode (no fork), drop `--source-project` / `--target-project` and
`glab` will open the MR within the current repo against `main`.

**`promote-to-human-vault/SKILL.md`:**

```yaml
---
name: promote-to-human-vault
description: Promote durable AI-vault content to Hagai's human vault via GitLab MR. Reformulates to match human-vault voice, searches for related notes to avoid duplication, and decides extend-vs-new placement. Use for crystallized decisions, tracked risks, recurring patterns, or explicit "save this" requests.
---
```

Mandatory workflow (enforced by SOUL.md "Promotion Discipline"):

1. **Trigger check.** Confirm the AI-vault content matches a Promotion Trigger
   from SOUL.md. If not, do not promote.
2. **Search human vault.** `memory_search.py --source human-vault "<topic>"` —
   gather related notes.
3. **Read `human-vault-map.md`.** Decide placement: extend an existing note vs.
   create a new one.
4. **Dedup check.** If semantic overlap with an existing note >70%, propose a
   merge edit (modify the existing file), not a new file. If overlap is partial,
   extend the relevant section.
5. **Reformulate.** Rewrite to match human-vault voice, density, naming
   conventions, and link style. Never paste AI-vault prose verbatim.
6. **Open MR.** Call `promote.py` with `ai_vault_source`, `target_file`,
   `reformulated_content`, `trigger`, `placement_rationale`, `dedup_result`.
7. **Update `human-vault-map.md`.** If the human vault structure shifted, run
   `update_vault_map.py` to refresh the AI's mental model.
8. **Tell Hagai.** "MR opened: [link]"

**`update_vault_map.py`** scans `human-vault/` and rewrites
`ai-vault/human-vault-map.md` with: top-level folders, file counts, naming
conventions, recurring section headings, observed link patterns. Runs after
every successful MR merge (or weekly via cron) so the AI's structural model
never drifts.

**Guardrail addition for Phase 8:**

```python
(r"git push.*--force",)
(r"git remote set-url.*human-vault",)
```

**Dependencies:** Phase 3 (dual-vault indexer), Phase 4 (OAuth pattern — same
env-loading discipline applies to `GITLAB_TOKEN`)\
**Estimated complexity:** Low–Medium

---

Phase 5: Skills (Starter Pack)
------------------------------

**What to build:** Two skills — one for vault structure (teach Claude how both
vaults are organized) and one for the most common engineering leadership task:
surfacing project risks.

**Key files to create:**

```
claude/skills/                # symlinked into <instance>/.claude/skills/
├── vault-structure/
│   ├── SKILL.md
│   └── references/
│       └── vault-map.md    # File tree + what goes where for both vaults
└── surface-risks/
    ├── SKILL.md
    └── scripts/
        └── gather-risk-context.py  # Pulls from calendar + MEMORY.md
```

**`vault-structure/SKILL.md`:**

```yaml
---
name: vault-structure
description: Reference for Hagai's dual-vault layout — AI vault (~/second-brain/work/ai-vault/) and human vault (~/notes/work/, mirrored at ~/second-brain/work/human-vault/). Covers folder taxonomy (briefings/, dossiers/, meetings/, drafts/, analysis/, logs/, questions/), Workflow A vs B for meeting notes, and what triggers promotion vs. stays in AI vault.
---
```

Body: Full map of both vaults, naming conventions, promotion triggers +
discipline (mirroring SOUL.md), Workflow A/B switch logic for meeting notes, and
the contract that the AI is sole writer of `ai-vault/` and only writes to
`human-vault/` via the `promote-to-human-vault` skill.

**`surface-risks/SKILL.md`:**

```yaml
---
name: surface-risks
description: Analyze active projects and calendar to surface engineering risks, blockers, and cross-team dependencies. Outputs a prioritized [P0/P1/P2] risk list.
---
```

Trigger: `/surface-risks` or "what are the risks", "what could go wrong",
"surface risks"

Script `gather-risk-context.py`:

1. Read `MEMORY.md` (active projects + open risks)
2. Call `query.py calendar upcoming` — deadlines in next 14 days
3. Pass assembled context to Claude: "List risks by severity [P0/P1/P2] with
   owner and mitigation. Be direct and terse."

**Dependencies:** Phase 1–3\
**Estimated complexity:** Low–Medium

---

Phase 6: Proactive Systems (Heartbeat + Reflection)
---------------------------------------------------

**What to build:** A scheduled heartbeat that gathers context, reasons with
Claude, writes events (raw inputs and proposed actions), and delivers advisories
via Discord DM. The heartbeat is the sole writer of MEMORY.md.

**Key files to create:**

```
src/sb/scripts/
├── heartbeat.py
└── memory_reflect.py
```

**Heartbeat schedule (Linux cron):**

```cron
# Active hours: 08:00–19:00 Israel time (UTC+3 summer, Sun–Thu)
*/30 5-16 * * 0-4  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-heartbeat
```

**`heartbeat.py` — data gathering phase:**

```python
def gather_context() -> dict:
    # sync_human_vault() does two things: (a) fast-forwards the local
    # submodule, (b) writes one `human_vault_commit` event per new commit.
    # Those events become available to the same heartbeat's reasoning phase —
    # no second cycle needed.
    if sync_human_vault():  # Returns True if HEAD advanced
        # Reindex human-vault delta before reasoning, otherwise the chat / RAG
        # reads a corpus older than the events the heartbeat is about to ingest.
        # Cheap: mtime-incremental + delete reconciliation only on changed files.
        subprocess.run(
            [
                sys.executable,  # heartbeat itself runs under uv-managed venv
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
        "active_risks": parse_memory_risks(),  # From MEMORY.md
        "active_projects": parse_memory_projects(),  # From MEMORY.md
        "human_vault_commits": fetch_unprocessed_human_vault_commits(),
    }


def fetch_unprocessed_human_vault_commits() -> list[dict]:
    """Commits not yet processed by *this* agent identity. Catches both
    fresh commits from this run and stragglers from a previous run that
    crashed before writing `event_processed`."""
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
        # Read the actual content from disk — vault is the source of truth.
        # `git show <sha>:<path>` gives the exact post-commit version even if
        # later commits in this batch modified the file again.
        contents = {}
        for f in payload["files"]:
            if f["status"].startswith("D"):
                continue  # deletion — nothing to read
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

**Ingest phase — write raw events via `write_event()`:**

```python
def ingest_events(context: dict):
    for email in context["unread_emails"]:
        write_event(
            Event(
                source="gmail",
                type="email_thread",
                source_id=email[
                    "thread_id"
                ],  # idempotent — re-ingesting same thread is a no-op
                occurred_at=email["received_at"],
                payload=email,
            )
        )
    for event in context["upcoming_deadlines"]:
        write_event(
            Event(
                source="calendar",
                type="calendar_event",
                source_id=event["id"],
                occurred_at=event["updated"],
                payload=event,
            )
        )
    # write_event already projects atomically — no separate replay phase needed
```

**Reasoning phase (Claude SDK):**

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

**Human-vault commit processing (same heartbeat cycle):**

After reasoning, the heartbeat closes the loop on each commit it consumed. This
is what `event_processing` exists for — the same pattern used for
`email_thread`. Without it, the next heartbeat would re-surface the same commits
forever.

```python
def mark_human_vault_commits_processed(commits: list[dict], output_refs: dict):
    """Record (event_id, agent_id) → status. Re-runs are no-ops (UNIQUE constraint)."""
    for commit in commits:
        conn.execute(
            "INSERT OR IGNORE INTO event_processing "
            "(event_id, agent_id, started_at, completed_at, status, output_refs) "
            "VALUES (?, ?, ?, ?, 'success', ?)",
            (
                commit["event_id"],
                current_agent_id,
                commit["occurred_at"],
                now_utc(),
                json.dumps(output_refs.get(commit["sha"], {})),
            ),
        )
```

`output_refs` captures what the agent did in response — e.g.
`{"<sha>": {"updated_dossier": "dossiers/people/alice.md", "linked_analysis": [...]}}`.
That history is queryable later: "for the last 50 human-vault commits, did the
agent meaningfully react, or did it no-op?" — a direct quality signal on whether
SOUL.md's reaction discipline is paying off.

**Reprocessing with a smarter agent.** Because `event_processing` is keyed by
`(event_id, agent_id)`, a new agent identity (different `prompt_hash`) sees
every commit as unprocessed and gets to re-react with its better judgment. Old
agent's outputs stay as immutable history.

**Action generation:** When the heartbeat flags emails needing a reply:

1. Fetch full thread via `gmail.get_thread(thread_id)`
2. Search human vault via `memory_search.py --source human-vault` for similar
   past context
3. Query
   `completed_actions WHERE type = 'email_draft' ORDER BY responded_at DESC LIMIT 20`
   — the last N proposals plus user responses become learning context for the
   new proposal
4. Generate action content via Claude
5. `write_event(Event(type="action_proposed", payload={action_id, type, content, agent_id}, source_event_id=email_event_id))`
   — atomic insert + project into `actions`
6. Send Discord DM, then `write_event(Event(type="notification_sent", ...))`

**AI vault writes (heartbeat is also the primary writer of
dossiers/briefings/analysis):**

Beyond `MEMORY.md`, the heartbeat freely writes to
`~/second-brain/work/ai-vault/`:

- `dossiers/people/<name>.md` — append/update rolling synthesis when a person
  appears in new comms
- `dossiers/projects/<slug>.md` — update project state when status changes
- `briefings/meetings/<YYYY-MM-DD>-<slug>.md` — generate pre-meeting briefings
  for tomorrow's calendar
- `analysis/<topic>.md` — capture speculative themes worth re-reading
- `logs/<YYYY-MM-DD>-<run>.md` — append the run's reasoning trace
- `questions/open.md` — append clarifications to surface to Hagai later

Use `[[wikilinks]]` between dossiers/meetings/analysis for navigability in
Obsidian.

**MEMORY.md update (heartbeat only — sole writer):** After processing events,
the heartbeat updates MEMORY.md:

- Add new risks, projects, decisions surfaced in this run
- Apply eviction policy if any section exceeds its cap: priority-desc, then
  recency-desc
- Enforce `max_bytes.MEMORY` from config.toml

**Notification dedupe** — the `notifications` projection table replaces md5-hash
diffing. Each action triggers at most one Discord DM:

```python
def notify_about_action(action_id: str, message: str):
    already_sent = conn.execute(
        "SELECT 1 FROM notifications WHERE related_id = ? AND related_type = 'action' LIMIT 1",
        (action_id,),
    ).fetchone()
    if already_sent:
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


def notify_linux(title: str, body: str):
    subprocess.run(["notify-send", title, body])
```

**`memory_reflect.py` — daily at 08:00 Israel time:**

```cron
0 5 * * *  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-reflect
```

Process:

1. Query the cloud DB:
   `SELECT * FROM events WHERE type = 'session_summary' AND occurred_at >= date('now', '-1 day')`
2. Ask Claude: "Extract decisions, risks, and key facts worth remembering
   long-term."
3. Update MEMORY.md under appropriate sections (respecting caps + eviction
   policy)
4. Write a daily snapshot to `ai-vault/briefings/daily/<YYYY-MM-DD>.md`
   (yesterday's themes, today's open items, [[wikilinks]] to relevant dossiers)
5. Refresh dossiers in `ai-vault/dossiers/` for any people/projects that
   surfaced in the day's session summaries
6. For any durable knowledge worth preserving: emit a `promote-to-human-vault`
   action (MR to `~/notes/work` upstream) via `write_event(action_proposed)` —
   see Phase 4.5 for the discipline (search, dedup, reformulate)

**Advisor mode behaviors:**

- Heartbeat proposes actions for review — never acts on them
- All Discord notifications are advisory: "Here's what needs your attention"
- No autonomous sending, posting, or purchasing

**Estimated cost:**
~$0.05/heartbeat run × 22 runs/day × 22 workdays/month ≈ **$24/month**

**Dependencies:** Phase 1–4\
**Estimated complexity:** High

---

Phase 7: Chat Interface (Conversational Access via Adapters)
------------------------------------------------------------

**What to build:** A persistent chat surface — Discord today,
Slack/Telegram/iMessage/SMS later — built on a platform-agnostic core and a thin
per-platform adapter. Conversation history is event-sourced; adding a new
platform is a single file plus an env-var entry.

### Architecture

Three layers, top-to-bottom:

1. **Adapters** (`adapters/*.py`) — platform-specific I/O only. Receive incoming
   messages, do auth, emit a normalized `ChatMessage`, send replies via a
   platform-native callback.
2. **Core** (`core.py`) — platform-agnostic. Takes a `ChatMessage`, loads
   history, calls Claude, persists turns as `chat_message` events, returns reply
   text.
3. **Runner** (`run.py`) — reads enabled adapters from env, runs them
   concurrently against the same core.

Each conversation has a stable, platform-namespaced `session_id` (e.g.
`discord:741...`, `slack:C0X...`). The `chat_messages` projection naturally
partitions per platform without per-platform schema.

### Key files

```
src/sb/chat/
├── run.py                    # Entry point — fans out to enabled adapters (exposed as `sb-chat`)
├── core.py                   # Platform-agnostic Claude loop
├── session_store.py          # DB-backed history (chat_messages projection)
├── adapter.py                # ChatAdapter protocol + ChatMessage dataclass
└── adapters/
    ├── __init__.py           # ADAPTERS = {"discord": DiscordAdapter, ...}
    ├── discord_adapter.py
    ├── slack_adapter.py      # (future)
    └── telegram_adapter.py   # (future)
```

### `adapter.py` — the contract

```python
from dataclasses import dataclass
from typing import Awaitable, Callable, Protocol


@dataclass
class ChatMessage:
    platform: str  # 'discord', 'slack', ...
    session_id: str  # f"{platform}:{channel_or_thread_id}"
    sender_id: str  # platform-native user id (already auth-checked by adapter)
    text: str


MessageHandler = Callable[[ChatMessage], Awaitable[str]]


class ChatAdapter(Protocol):
    name: str

    async def run(self, handle: MessageHandler) -> None:
        """Start the platform event loop. For each authorized incoming message,
        call `handle(msg)` and send the returned string back to the sender."""
```

Adapter responsibilities (the contract, not optional):

- **Auth** — drop messages from anyone who isn't Hagai before invoking `handle`.
- **DMs only** — ignore channel/group messages unless explicitly opted in.
- **Session ID namespacing** — always prefix with `{platform}:` so projections
  stay collision-free.
- **Idempotent reply** — on transient send failures, do not double-invoke
  `handle`.

### `core.py` — single source of Claude logic

```python
from anthropic import Anthropic
from pathlib import Path
from session_store import get_history
from db import write_event, Event

anthropic = Anthropic()


def load_context() -> str:
    vault = Path.home() / "second-brain/work/ai-vault"
    return "\n\n".join(
        (vault / f).read_text() for f in ("SOUL.md", "USER.md", "MEMORY.md")
    )


async def handle(msg: ChatMessage) -> str:
    history = get_history(msg.session_id)  # [{role, content}, ...]
    history.append({"role": "user", "content": msg.text})

    write_event(
        Event(
            source=msg.platform,
            type="chat_message",
            payload={"role": "user", "content": msg.text},
            metadata={"session_id": msg.session_id, "sender_id": msg.sender_id},
        )
    )

    response = anthropic.messages.create(
        model="claude-sonnet-4-6",
        max_tokens=1024,
        system=load_context(),
        messages=history,
    )
    reply = response.content[0].text

    write_event(
        Event(
            source=msg.platform,
            type="chat_message",
            payload={"role": "assistant", "content": reply},
            metadata={"session_id": msg.session_id},
        )
    )
    return reply
```

### `adapters/discord_adapter.py` — reference implementation

```python
import discord, os
from adapter import ChatMessage, MessageHandler


class DiscordAdapter:
    name = "discord"

    async def run(self, handle: MessageHandler) -> None:
        intents = discord.Intents.default()
        intents.dm_messages = True
        intents.message_content = True
        client = discord.Client(intents=intents)
        owner_id = os.environ["DISCORD_USER_ID"]

        @client.event
        async def on_message(m):
            if m.author.bot or not isinstance(m.channel, discord.DMChannel):
                return
            if str(m.author.id) != owner_id:
                return
            msg = ChatMessage(
                platform="discord",
                session_id=f"discord:{m.channel.id}",
                sender_id=str(m.author.id),
                text=m.content,
            )
            reply = await handle(msg)
            await m.reply(reply)

        await client.start(os.environ["DISCORD_BOT_TOKEN"])
```

### `run.py` — fan out

```python
import asyncio, os
from core import handle
from adapters import ADAPTERS


async def main():
    enabled = os.environ.get("CHAT_ADAPTERS", "discord").split(",")
    await asyncio.gather(*[ADAPTERS[name]().run(handle) for name in enabled])


if __name__ == "__main__":
    asyncio.run(main())
```

### `chat_messages` projection (added to Phase 1 schema)

```sql
CREATE TABLE chat_messages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id    INTEGER NOT NULL REFERENCES events(id),
    session_id  TEXT NOT NULL,            -- 'discord:741...', 'slack:C0X...'
    platform    TEXT NOT NULL,            -- 'discord', 'slack', ...
    role        TEXT NOT NULL,            -- 'user' | 'assistant'
    content     TEXT NOT NULL,
    created_at  TIMESTAMP NOT NULL
);

CREATE INDEX chat_messages_session ON chat_messages(session_id, id);
```

`project_event()` gains one branch: `chat_message` → insert into
`chat_messages`. `get_history(session_id)` is
`SELECT role, content FROM chat_messages WHERE session_id = ? ORDER BY id`.

### Adding a new platform (the test of the abstraction)

1. Create `adapters/slack_adapter.py` implementing `ChatAdapter` (auth via Slack
   signing secret, session_id `slack:{channel_id}`).
2. Register in `adapters/__init__.py`: `ADAPTERS["slack"] = SlackAdapter`.
3. Add creds to `.env`, set `CHAT_ADAPTERS=discord,slack`.
4. Restart the systemd unit. No core changes, no schema changes.

If a new platform requires a change to `core.py` or the schema, the adapter
contract is wrong — fix the contract, not the platform.

### Run as a background service

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
systemctl --user enable --now second-brain-chat
```

**Dependencies:** Phase 1–3, Phase 4 (at least one platform integration's auth
wired up)\
**Estimated complexity:** High (first adapter); Low per additional adapter

---

Phase 8: Security Hardening
---------------------------

**What to build:** Three-layer defense: sanitize all external content,
guardrails matching Hagai's security boundaries, and API key isolation from
Claude.

**Key files to create:**

```
src/sb/scripts/
├── sanitize.py
└── guardrails.py
```

**`sanitize.py` — 3-layer defense:**

````python
import re


def sanitize_external_text(text: str, source: str = "unknown") -> str:
    """Apply before ANY external content (email body, calendar description) reaches Claude."""
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"```[\s\S]*?```", "[code block]", text)

    injection_patterns = [
        r"ignore (previous|all) instructions?",
        r"you are now",
        r"system prompt",
        r"act as",
        r"jailbreak",
    ]
    for pattern in injection_patterns:
        text = re.sub(pattern, "[filtered]", text, flags=re.IGNORECASE)

    return f'<external_content source="{source}">\n{text}\n</external_content>'
````

**`guardrails.py` — security boundaries as code:**

```python
BLOCKED_PATTERNS = [
    r"send.*email",
    r"gmail.*send",
    r"service\.users\(\)\.messages\(\)\.send",
    r"discord.*send(?!.*notify)",
    r"post.*social",
    r"tweet|instagram|linkedin.*post",
    r"purchase|payment|stripe|bank|financial.*access",
    r"transfer.*funds",
    r"git push.*--force",
    r"git remote set-url.*human-vault",
]


def check_command(command: str) -> tuple[bool, str]:
    for pattern in BLOCKED_PATTERNS:
        if re.search(pattern, command, re.IGNORECASE):
            return False, f"Blocked by security guardrail: matches '{pattern}'"
    return True, "ok"
```

**API Key isolation:**

- Claude never sees `.env` values
- Python scripts handle all auth, pass only data to Claude
- Use `python-dotenv` to load `.env` in scripts

**`.gitignore` entries:**

```
.env
~/.config/second-brain/credentials.json
~/.config/second-brain/token.json
```

**PreToolUse hook for guardrails:**

```json
{
    "hooks": {
        "PreToolUse": [
            {
                "type": "command",
                "command": "uv run --project $HOME/dotfiles/second-brain sb-guardrail",
                "matcher": { "tool_name": "Bash" }
            }
        ]
    }
}
```

**Dependencies:** All previous phases\
**Estimated complexity:** Medium

---

Phase 9: Deployment (Linux Local)
---------------------------------

**What to build:** Wire all scheduled scripts to Linux cron, set up the Discord
bot as a systemd user service, and verify end-to-end flow.

**Cron jobs summary (`crontab -e`):** Each line `cd`s into the instance dir
first; `find_instance()` then resolves the instance from cwd. Logs are
per-instance.

All scheduled scripts run via console scripts under
`uv run --project ~/dotfiles/second-brain ...`. uv resolves the project venv
without requiring activation (which cron does not inherit), and the console
scripts (`sb-index`, `sb-heartbeat`, `sb-reflect`) are declared in the package
`pyproject.toml`.

```cron
# Memory indexer — every 15 minutes
*/15 * * * *  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-index >> ~/.local/log/second-brain/work/index.log 2>&1

# Heartbeat — every 30 min, Israeli working hours (Sun–Thu, 08:00–19:00 local = 05:00–16:00 UTC summer)
*/30 5-16 * * 0-4  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-heartbeat >> ~/.local/log/second-brain/work/heartbeat.log 2>&1

# Daily reflection — 08:00 Israel time (05:00 UTC summer)
0 5 * * *  cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-reflect >> ~/.local/log/second-brain/work/reflect.log 2>&1
```

**Note on Israel work week:** Sunday–Thursday. Cron day-of-week `0-4` (Sun=0,
Thu=4).

**DB initialization:**

Bootstrap runs both automatically (Phase 1 step 5). The commands here are for
manual re-init or recovery:

```bash
# Cloud DB schema — creates tables in Turso (or local SQLite, depending on DATABASE_URL).
# Idempotent (CREATE TABLE IF NOT EXISTS); also primes the embedded replica file.
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-db --init

# Local RAG DB schema (separate file, local-only)
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain sb-rag-db --init
```

**Operational commands:**

```bash
# All commands assume cwd=~/second-brain/work so find_instance() resolves correctly.

# Snapshot the cloud DB to a portable file (.sql text or .db file)
uv run --project ~/dotfiles/second-brain sb-db --dump > ~/backups/cloud-$(date +%F).sql

# Wipe and rebuild projections from events (after schema change or new agent)
uv run --project ~/dotfiles/second-brain sb-db --rebuild

# Show current state: event count, projection cursor positions, agents registered,
# plus a sync-health panel (see below).
uv run --project ~/dotfiles/second-brain sb-db --status
```

**`--status` sync-health panel.** When something feels stale ("did the agent see
my note from this morning?"), one CLI answers it. Output sections:

- **Events**: `MAX(ingested_at)` per `source` — last time
  gmail/calendar/heartbeat wrote anything.
- **Human vault**: most recent `human_vault_synced` event — `head_after` SHA,
  `advanced` flag, age. If older than ~1 heartbeat cycle, the submodule fetch is
  failing silently.
- **RAG index**: `MAX(indexed_at)` per `source` in `chunks`, plus `COUNT(*)`
  chunks per source. A frozen `indexed_at` means the indexer cron stopped
  firing.
- **Replica**: libSQL embedded-replica last-sync timestamp (`conn.sync()`
  records it on success). If the network is down the local replica still serves
  reads, but stale-replica age is what tells you.
- **Projections**: `projection_cursors.last_event_id` vs `MAX(events.id)` —
  drift means the projector died mid-replay.
- **Session hooks**: most recent `session_summary` event timestamp (split by
  `source = 'session_compact' | 'session_end'`). Soft-fail in the hooks means a
  broken DB write surfaces only as silently-missing events; this panel is the
  tripwire. If no `session_summary` for several days despite active Claude
  sessions, check `~/.local/log/second-brain/<instance>/hooks.log`.

Implementation is plain SQL queries plus one `os.stat` on the replica file; no
new state to maintain.

**Swap-away from Turso (the embedded replica is your built-in escape hatch):**

```bash
# 1. Capture a final snapshot
uv run --project ~/dotfiles/second-brain sb-db --dump > ~/backups/cloud-final.sql

# 2. Promote the embedded replica to be the source of truth
cp ~/.local/share/second-brain/work/cloud-replica.db \
   ~/.local/share/second-brain/work/cloud.db

# 3. Update DATABASE_URL in .env from libsql://... to file:///...
# Same db.py works in both modes — no code changes.
```

**systemd user service for chat bot (multi-platform via adapters):**

```bash
mkdir -p ~/.config/systemd/user
systemctl --user daemon-reload
systemctl --user enable --now second-brain-chat
systemctl --user status second-brain-chat
```

**Python environment setup:**

The project lives at `~/dotfiles/second-brain/` with its own `pyproject.toml`
and uv-managed venv at `~/dotfiles/second-brain/.venv`. All runtime invocations
(hooks, cron, systemd) use `uv run --project
~/dotfiles/second-brain ...` so no
activation is needed.

```bash
# One-time: sync the venv from pyproject.toml. uv creates ~/dotfiles/second-brain/.venv.
uv sync --project ~/dotfiles/second-brain
```

Add `.venv/` to `~/dotfiles/second-brain/.gitignore`. Adding a new dep is a
one-line edit to `pyproject.toml` followed by `uv sync`.

**OAuth first-run flow:**

```bash
cd ~/second-brain/work && uv run --project ~/dotfiles/second-brain python -m sb.scripts.integrations.shared_auth
# Token saved to GOOGLE_TOKEN_PATH — auto-refreshes after that
```

**Log rotation:**

```
~/.local/log/second-brain/*/*.log {
    daily
    rotate 7
    compress
    missingok
}
```

**Cost estimate (local-only):**

| Component                                         | Monthly Cost      |
| ------------------------------------------------- | ----------------- |
| Claude API (Sonnet 4.6, ~22 heartbeats/day)       | ~$24              |
| Claude API (chat interface, varies)               | ~$5–15            |
| Turso (free tier sufficient at single-user scale) | $0                |
| Obsidian                                          | Free              |
| Infrastructure                                    | $0 (local)        |
| **Total**                                         | **~$29–39/month** |

**Dependencies:** All phases\
**Estimated complexity:** Low (mostly config)

---

Recommended Build Order
-----------------------

```
Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 4.5 → Phase 5
                                               ↓
                                         Phase 6 (requires 1–4.5)
                                               ↓
                                         Phase 7 (requires 4, optional)
                                               ↓
                                         Phase 8 (wraps all)
                                               ↓
                                         Phase 9 (deploys all)
```

**Parallelizable after Phase 4:**

- Phase 4.5 (Human Vault Integration) can be built in parallel with Phase 5
  (Skills)
- Phase 7 (Discord bot) can be built in parallel with Phase 8 (Security)

**Recommended first milestone (1–2 days):** Complete Phases 1–2. Every Claude
conversation immediately benefits from memory injection.

**Recommended second milestone (3–5 days):** Add Phase 3 + Gmail integration.
Both vaults become searchable; heartbeat starts proposing actions.

**Recommended third milestone (1 day, high leverage):** Phase 4.5. Once the
human-vault submodule is mounted and the indexer scans both vaults, every
conversation benefits from your curated human vault as additional context — and
the agent gains the `promote-to-human-vault` MR path for durable knowledge.

---

_This PRD was last updated 2026-04-30 with three changes: (1) **code/content
split** — code now lives in `~/dotfiles/second-brain/` as an ordinary
installable Python package (`src/sb/`, console scripts `sb-index`,
`sb-heartbeat`, …), content lives in per-instance repos at
`~/second-brain/<instance>/` (today: `work`; planned: `personal`); per-instance
state under `~/.local/share/second-brain/<instance>/` to avoid replica/RAG
collisions; bootstrap is
`uv run --project ~/dotfiles/second-brain python ~/dotfiles/second-brain/bootstrap.py <instance>`.
(2) **Instance discovery** moved from the unresolved-`__file__` /
`.agents/`-symlink trick to a single `find_instance()` helper that ascends from
cwd looking for `config.toml` (or honors `SECOND_BRAIN_INSTANCE`); the only
remaining symlinks are `.claude/hooks` and `.claude/skills`, which Claude Code
itself discovers by path. (3) **GitLab migration** — human-vault remote moved
from GitHub to GitLab (Hagai's employer hosts GitLab; `GITLAB_HOST`
configurable, `glab` CLI replaces `gh`, `GITLAB_TOKEN` replaces
`GITHUB_HUMAN_VAULT_TOKEN`, MR replaces PR throughout). Earlier dual-vault
alignment review preserved. Key architectural decisions: event sourcing with a
cloud SQL DB (Turso/libSQL) as single source of truth, accessed via embedded
replica for local-disk read latency; **rich AI Obsidian vault** at
`~/second-brain/work/ai-vault/` (briefings, dossiers, drafts, analysis, logs,
questions — AI is sole writer with editorial freedom) sitting alongside the
**read-only mirror of the human vault** at `~/second-brain/work/human-vault/`
(git submodule of the AI's fork of `~/notes/work`); promotion AI → human only
via MR with mandatory search + reformulation + dedup discipline; separate local
RAG DB regenerable from vault files; actions and responses merged in one
projection table with full history preserved in events; agents (not just models)
tracked as first-class identity for reprocessability; substrate swappable to
plain SQLite via `DATABASE_URL` change with no code rewrite._
