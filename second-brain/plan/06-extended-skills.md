Extended Skills (Beyond MVP)
============================

**Status:** Future (post-MVP). **Depends on:** MVP (Phases 1–3 + 4.5 + slim 5),
`02-google-calendar-integration.md` (`surface-risks` queries calendar
deadlines), `04-heartbeat-and-reflection.md` (MEMORY.md is reliably populated
only once heartbeat runs daily).

What to build
-------------

Additional Claude skills that activate once integrations are wired. The MVP
ships `vault-structure` and `promote-to-human-vault`; this PRD adds the next
batch.

`surface-risks`
---------------

The most common engineering-leadership task: surface project risks, blockers,
and cross-team dependencies as a prioritized [P0/P1/P2] list.

```
claude/skills/surface-risks/
├── SKILL.md
└── scripts/
    └── gather-risk-context.py
```

`SKILL.md`:

```yaml
---
name: surface-risks
description: Analyze active projects and calendar to surface engineering risks, blockers, and cross-team dependencies. Outputs a prioritized [P0/P1/P2] risk list.
---
```

Trigger: `/surface-risks` or "what are the risks", "what could go wrong",
"surface risks".

`gather-risk-context.py`:

1. Read `MEMORY.md` (active projects + open risks).
2. Call `sb-query calendar upcoming` — deadlines in next 14 days.
3. Pass assembled context to Claude: "List risks by severity [P0/P1/P2] with
   owner and mitigation. Be direct and terse."

Future skills (sketch)
----------------------

When the heartbeat / reflection PRDs land and dossiers/briefings are populated,
the following skills become natural follow-ups. Each is its own small drop-in
once written; none changes the architecture.

- `summarize-meeting` — given a transcript path, produce the AI-vault digest at
  `meetings/YYYY-MM-DD-name.md` and check whether to switch to Workflow B or
  trigger promotion.
- `draft-email-reply` — given a thread id, fetch via `gmail.get_thread()`,
  search the human vault, and draft a reply for review.
- `pre-meeting-brief` — given a calendar event id, gather attendees' dossiers +
  recent comms and produce `briefings/meetings/<id>.md`.
- `weekly-review` — render last week's `session_summary` events plus promoted
  MRs as a one-page review.

Each should follow the contract: a Python helper script gathers structured
context, Claude does the language work, output goes to a known AI-vault path.

Acceptance — "done" (per skill)
-------------------------------

- `SKILL.md` description triggers correctly on natural-language phrasing.
- Helper script is deterministic; Claude does only the synthesis.
- Output lands in the correct AI-vault subfolder with `[[wikilinks]]` to related
  dossiers.

**Dependencies:** MVP + Calendar/Gmail integrations + heartbeat (so MEMORY.md
and dossiers are non-empty). **Estimated complexity:** Low per skill.
