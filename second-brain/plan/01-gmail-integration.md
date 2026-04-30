Gmail Integration
=================

**Status:** Future (post-MVP). **Depends on:** MVP (Phase 1 cloud DB, manual
event entry path), `secrets.toml` discipline established by Phase 4.5. **Why
this is its own PRD:** the MVP exercises the events table via `sb-event add`.
Gmail is the first automated collector and brings the shared integration
scaffolding (`registry.py`, `query.py`, `shared_auth.py`) along with it — the
first integration PRD owns that scaffolding because it's the first one that has
a real consumer.

---

What to build
-------------

A Gmail collector that lists important unread threads, surfaces them as
`email_thread` events in the cloud DB, and exposes a `query.py gmail` subcommand
for ad-hoc inspection. **No heartbeat ingestion** in this PRD — that's wired in
`04-heartbeat-and-reflection.md`. **No `sanitize_external_text` real
implementation** — Phase 4 ships a no-op stub; the real one lands in
`07-security-hardening.md`.

Files
-----

```
src/sb/scripts/integrations/
├── __init__.py
├── registry.py              # reads [integrations] enabled from config.toml
├── query.py                 # unified CLI: query.py gmail list [--format=text|json]
├── shared_auth.py           # Google OAuth2 + retry helper
├── secrets.py               # loads ~/.local/share/second-brain/<instance>/secrets.toml
├── errors.py
├── sanitize.py              # no-op stub
├── gmail.py
└── integration_template.py
```

`pyproject.toml` adds: `google-auth`, `google-auth-oauthlib`,
`google-api-python-client`, `httpx`, `tenacity`.

Configuration
-------------

```toml
# config.toml
[integrations]
enabled = ["gmail"]

[integrations.gmail]
filter = "(is:important OR is:starred) category:primary is:unread"
max_results = 20
```

```toml
# ~/.local/share/second-brain/<instance>/secrets.toml
[google]
credentials_path = "~/.local/share/second-brain/<instance>/credentials.json"
token_path = "~/.local/share/second-brain/<instance>/token.json"
```

Auth setup
----------

1. Google Cloud Console → Create project → Enable Gmail API.
2. OAuth consent screen → Add Gmail address as test user.
3. **Move consent screen to "In production"** — in "Testing" mode Google expires
   refresh tokens after 7 days, which silently breaks recurring ingestion every
   week.
4. Create OAuth 2.0 credentials → Desktop app → set redirect URI to **exactly**
   `http://localhost:8765/` → download `credentials.json` to
   `~/.local/share/second-brain/<instance>/credentials.json`.
5. First run:
   `uv run --project ~/dotfiles/second-brain python -m sb.scripts.integrations.shared_auth`
   opens a browser via `InstalledAppFlow.run_local_server(port=8765)` →
   authenticate → `token.json` saved next to `credentials.json`.

   **WSL2 note:** Win11 + WSLg forwards `localhost` automatically. On older
   Windows builds run once:
   `netsh interface portproxy add v4tov4 listenport=8765 listenaddress=127.0.0.1 connectport=8765 connectaddress=$(wsl hostname -I)`.

**Required scopes:**

- `https://www.googleapis.com/auth/gmail.readonly`
- `https://www.googleapis.com/auth/gmail.compose` (only if the agent should
  create Gmail drafts)

Error model — `errors.py`
-------------------------

```python
class IntegrationError(Exception): ...


class IntegrationAuthError(
    IntegrationError
): ...  # permanent: RefreshError, 401, 403


class IntegrationTransientError(
    IntegrationError
): ...  # retried out: 429, 5xx, network


class MessageTooLong(IntegrationError):
    def __init__(self, length: int, limit: int):
        self.length, self.limit = length, limit
        super().__init__(f"message length {length} exceeds {limit}-char cap")
```

`shared_auth.py` exposes a `_retry` helper: `tenacity` with 3 tries, exponential
backoff (1s/2s/4s), retrying only on `HttpError` 429/5xx and
`httpx.TransportError`. Auth-class failures
(`google.auth.exceptions.
RefreshError`, 401/403) map to `IntegrationAuthError`.

Errors are isolated **per integration**: one broken integration does not crash a
caller — it returns an empty list.

`gmail.py` — unit is the thread, not the message
------------------------------------------------

```python
@dataclass
class EmailSummary:
    """One thread, identified by (thread_id, latest_message_id) for idempotency."""

    thread_id: str
    latest_message_id: str
    source_id: str  # f"{thread_id}:{latest_message_id}" — version-bumps when thread grows
    subject: str
    sender: str
    snippet: str
    received_at: datetime


@dataclass
class ThreadContext:
    thread_id: str
    messages: list[EmailSummary]
    full_bodies: list[str]  # already sanitized


def list_important_unread(
    filter_query: str, max_results: int = 20
) -> list[EmailSummary]:
    """Returns one EmailSummary per matching thread."""
    ...


def get_thread(thread_id: str) -> ThreadContext:
    """Full thread for action-generation context."""
    ...
```

**Why per-thread, versioned `source_id`:** combining `thread_id` with
`latest_message_id` keeps the event log idempotent on repeat ingestion _and_
re-surfaces the thread when it grows.

**Rate limits:** Gmail API: 250 quota units/sec, 1B/day. At 20 threads/heartbeat
every 30 min: ~200 units/hour — well within limits.

`query.py` subcommands
----------------------

```bash
sb-query gmail list                         # Unread important threads (text)
sb-query gmail thread <thread_id>           # Full thread for context
sb-query gmail list --format=json           # Programmatic / tests
```

Output formatters: text (one block per item, stable field labels) and JSON
(`[asdict(item), ...]`). Tests assert against JSON.

Integration template (`integration_template.py`)
------------------------------------------------

Copy this file when wiring up a new platform. The contract every integration
must satisfy: auth stays in Python, Claude only ever sees typed data, external
text is sanitized at the boundary, and errors are mapped to the shared taxonomy.

```python
"""
Pattern every integration follows:
  1. Auth via shared module or secrets.toml. Claude never sees credentials.
  2. Return typed values (dataclasses) — not raw SDK responses.
  3. Sanitize any free-text field reaching Claude (real impl in security PRD).
  4. One bounded list_*() call per heartbeat.
  5. Map errors to errors.IntegrationAuthError / IntegrationTransientError.
  6. Register in registry.py and add a query.py subcommand.
"""
```

If a new integration needs to break this contract, the contract is wrong — fix
the template, not the integration.

Registry
--------

```python
# registry.py — reads [integrations].enabled and resolves each name.
_MODULES = {
    "gmail": "sb.scripts.integrations.gmail",
    "calendar": "sb.scripts.integrations.calendar",  # future PRD
    "discord": "sb.scripts.integrations.discord_client",  # future PRD
}
```

Missing creds for an enabled integration → fail-fast with a clear error.

Tests
-----

CLAUDE.md mandates "no network" in the default suite. Tests mock at the SDK
boundary:

- `gmail.tests.py` — patch `googleapiclient.discovery.build`; assert
  `EmailSummary` shape, `source_id` versioning, error mapping.
- `query.tests.py` — text and JSON formatters on golden inputs.
- `shared_auth.tests.py` — `retry` retries on 429/5xx, propagates on others;
  `IntegrationAuthError` mapping.

A separate `tests/integration_live/` holds opt-in tests that hit real Gmail when
credentials are present. Run manually, never CI.

Acceptance — "done"
-------------------

- `secrets.example.toml` checked in.
- `config.toml` gains `[integrations]` and `[integrations.gmail]`.
- `pyproject.toml` adds the listed deps.
- `sb-query gmail list` succeeds against a real account.
- Default test suite passes with all SDK calls mocked.

**Explicit non-scope:** heartbeat ingestion (next PRD that wants it owns it),
sanitize_external_text real impl (security PRD).

---

Hand-off to the heartbeat PRD
-----------------------------

The heartbeat PRD picks Gmail up by writing one event per thread:

```python
for email in gmail.list_important_unread():
    write_event(
        Event(
            source="gmail",
            type="email_thread",
            source_id=email.source_id,
            occurred_at=email.received_at,
            payload=asdict(email),
        )
    )
```

`event_processing` then keys reaction state on `(event_id, agent_id)` so
re-running with a smarter agent reprocesses every thread once.
