Second Brain — Plan Index
=========================

This directory replaces the monolithic `second-brain-prd.md` with a vertical MVP
slice plus focused follow-up PRDs. Each follow-up is a self-contained feature
that can be built independently once the MVP is in place.

Build the MVP first. Then pick follow-ups based on which capability you most
want next.

MVP (build first — end-to-end working system)
---------------------------------------------

- [00-mvp.md](00-mvp.md) — Memory layer, cloud DB, hooks, hybrid RAG, manual
  event entry, human-vault GitLab integration with MR-based promotion, two
  skills, one guardrail, manual deployment. The MVP is a complete vertical slice
  — collect events, persist session memory, search both vaults, propose changes
  to the human vault via MR.

Follow-ups (each a vertical feature; pick what's most useful next)
------------------------------------------------------------------

- [01-gmail-integration.md](01-gmail-integration.md) — First automated
  collector. Brings the shared integration scaffolding (`shared_auth`,
  `registry`, `query.py`).
- [02-google-calendar-integration.md](02-google-calendar-integration.md) —
  Reuses Gmail's OAuth.
- [03-discord-outbound.md](03-discord-outbound.md) — REST-only DM notifier.
  Outbound only; chat lives separately.
- [04-heartbeat-and-reflection.md](04-heartbeat-and-reflection.md) — The
  proactive loop. Activates the deferred projection branches in
  `project_event()` and is the first place the integrations get used together.
- [05-chat-interface.md](05-chat-interface.md) — Discord inbound + the adapter
  abstraction for future Slack/Telegram/iMessage/SMS.
- [06-extended-skills.md](06-extended-skills.md) — `surface-risks` and beyond.
  Most need heartbeat-populated MEMORY.md / dossiers to be useful.
- [07-security-hardening.md](07-security-hardening.md) — Real
  `sanitize_external_text` + the full guardrail pattern list. The MVP ships only
  the one guardrail Phase 4.5 requires.
- [08-deployment-systemd.md](08-deployment-systemd.md) — cron timers + systemd
  user services. The MVP runs everything on demand.
- [09-multi-instance.md](09-multi-instance.md) — Bootstrap a `personal` instance
  alongside `work`.

Suggested order
---------------

```
00-mvp
   ↓
01-gmail → 02-calendar → 03-discord-outbound → 04-heartbeat → 06-extended-skills
                                                    ↓
                                              05-chat-interface
                                                    ↓
                                              07-security-hardening
                                                    ↓
                                              08-deployment-systemd
                                                    ↓
                                              09-multi-instance (any time after MVP)
```

Architectural commitments (do not break)
----------------------------------------

These are load-bearing across all PRDs:

- **Cloud SQL DB (Turso/libSQL) is the single source of truth.** Plain SQL only.
  The embedded replica is the local-read path and the swap-away escape hatch.
- **Append-only `events` table is the spine.** Every other table is a projection
  rebuildable by replaying the event log via `project_event()`.
- **Code in dotfiles, content per instance.** `~/dotfiles/second-brain/` ships
  the package; `~/second-brain/<instance>/` is content; per-instance state under
  `~/.local/share/second-brain/<instance>/`.
- **Instance discovery via `find_instance()`** — marker-file ascent (or
  `SECOND_BRAIN_INSTANCE` env var). Nothing else.
- **Two-vault editorial model.** AI writes freely to `ai-vault/`; human vault
  changes only via MR with mandatory search + reformulation + dedup.
- **Advisor mode.** The agent proposes; Hagai disposes. No autonomous sending,
  posting, or purchasing.
- **Agent identity is first-class.** `(harness, model, version,
  prompt_hash)`
  tuple. New prompt = new agent row. Reprocessing with a smarter agent re-keys
  `event_processing(event_id, agent_id)` so old outputs stay as immutable
  history.
