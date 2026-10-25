# Review comment voice

The global AGENTS.md rules ("Write in my voice", no em dash, no semicolon) still
apply. These rules are for PR comments in particular.

## Rules

- Lead with the point. A clear fix starts with the imperative ("Use `isLoading`
  here instead"). When you're unsure what the author meant, ask a direct
  question ("Is this supposed to be renamed?").
- Give the reason in one short clause, often in parentheses. No paragraph of
  evidence.
- Show what the user would see. Quote the wrong text and the right text. Render
  UI as a markdown quote block.
- Don't explain what the author can check: generated type names, backend status
  codes, other threads, bot comments, line numbers elsewhere.
- Leave room for a reason you can't see ("unless we have real different
  actions?").
- Casual is fine: "A bit awkward no?", "probably".
- Use "we" for the team and the product ("Do we use Mark or Flag?").
- Mention sibling code in a few words ("EDM does the same.").
- No "Nit:" prefix, no severity tags, no "Can we ...?" softeners, no list of
  every place it happens.
- One to three sentences. A quote block plus a line or two at most.

## Before and after

Draft:

> This renames the EDM table to `FreetextDetectorRefConfig`, which is the name
> of a generated API type (used in `utils.ts` and `CustomDetectorDrawer.tsx`).
> Looks like an accidental rename. Can we revert it here and in
> `EdmDetectorsTab.tsx`?

Mine:

> Is this supposed to be renamed? Does `EdmDetectorsTable` no longer exist?

Draft:

> `isFetchingUsage` is also true during each 10s poll. If the menu opens during
> a poll, Delete stays greyed out until it's reopened (the issue from the
> earlier bot thread). Using `isLoading` here is a one-word fix for that. The
> backend already returns 409 when a detector is in use, so we lose no safety.
> EDM has the same line if you want to fix both.

Mine:

> Use `isLoading` here instead (`isFetchingUsage` is true briefly after each
> poll, meaning Delete is grayed out during polls). EDM does the same.

Draft:

> Nit: with one profile this reads "used by 1 Engine Profiles".
> `{count, plural, one {# Engine Profile} other {# Engine Profiles}}` fixes it.
> EDM has the same string.

Mine:

> Fix text for 1 profile, which will end up as "used by 1 Engine Profiles"
> instead of "used by 1 Engine Profile" (without plural "s").

Draft:

> The explanation now repeats the title, so the Summary step shows "Flagged as
> Violation" next to "Flagged as violation". Same on line 90. Bring back a real
> explanation, or drop it.

Mine (raw markdown, so the two quoted lines stay separate):

```text
Doesn't this show up in the UI as:

> **Flagged as Violation**
> Flagged as violation

A bit awkward no? Can remove the explanation field probably.
```

Draft:

> The copy now uses both "Mark" and "Flag" for the same action. This tooltip
> says "Flag conditions", the two below say "Mark N more". Elsewhere: "Mark
> conditions", "Flagged as Safe", "Unflagged conditions", "Keep flagging".
>
> "Flag" also means what the detector catches ("what this detector should
> flag"), so "Flagged as Safe" reads oddly. Can we use "Mark" everywhere? If the
> copy spec mixes them, worth checking with UX.

Mine (moved to a line that still says "Mark"):

> Do we use Mark or Flag? Above you changed some "Mark" to be "Flag", etc. Maybe
> using single consistent term would be better, unless we have real different
> actions (Flag & Mark)?
