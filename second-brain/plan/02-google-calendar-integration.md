Google Calendar Integration
===========================

**Status:** Future (post-MVP). **Depends on:** `01-gmail-integration.md` (shared
`shared_auth.py`, `registry.py`, `query.py`, `errors.py`, `secrets.toml`,
`pyproject.toml` deps). Calendar reuses Gmail's OAuth token via `shared_auth`.

What to build
-------------

A Calendar collector that lists upcoming events and deadline-keyword matches,
exposed via `sb-query calendar`. **No heartbeat ingestion** here — that's
`04-heartbeat-and-reflection.md`.

Files
-----

```
src/sb/scripts/integrations/
└── calendar.py
```

Configuration
-------------

```toml
[integrations]
enabled = ["gmail", "calendar"]

[integrations.calendar]
calendar_ids = ["all"] # "all" → calendarList().list() minus declined
days_ahead = 7
deadline_keywords = ["deadline", "due", "review", "launch", "release"]
# Per-user vocabulary — Hebrew speakers may want "מועד אחרון", etc.
```

Auth
----

Shared OAuth2 token from `shared_auth.py`. Add scope:

- `https://www.googleapis.com/auth/calendar.readonly`

`calendar.py`
-------------

```python
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

**Israel timezone note:** Always store/compare in UTC. Display in
`Asia/Jerusalem` using `zoneinfo`.

`query.py` subcommands
----------------------

```bash
sb-query calendar upcoming        # Next [days_ahead] days, all enabled calendars
sb-query calendar deadlines       # Events matching deadline_keywords
```

Tests
-----

- `calendar.tests.py` — patch `googleapiclient.discovery.build`; assert `Event`
  shape, `"all"`-calendar expansion, deadline keyword filter (case-insensitive,
  multi-keyword).
- `tests/integration_live/calendar_live.py` — opt-in.

Acceptance — "done"
-------------------

- `[integrations.calendar]` in `config.toml`.
- `calendar` listed in `[integrations].enabled`.
- `sb-query calendar upcoming` and `sb-query calendar deadlines` succeed against
  a real account.
- Default test suite passes with SDK calls mocked.
- Calendar scope added to OAuth consent screen and re-consent walked through
  (the `token.json` from Gmail-only setup needs re-issuing once the scope list
  grows).

---

Hand-off to the heartbeat PRD
-----------------------------

```python
for ev in calendar.list_events_with_deadlines(...):
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
