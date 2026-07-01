---
name: handoff
description: Handoff session to a fresh agent
argument-hint: "What will the next session be used for?"
disable-model-invocation: true
model: sonnet
effort: high
---

<task>
Write a handoff document summarizing the current conversation so a fresh agent
can continue the work. Save it to
`/tmp/handoff/YYYY-MM-DD.HH-MM.descriptive-session-name.md`.
</task>

<guidelines>
Suggest the skills to be used, if any, by the next session.

Do not duplicate content already captured in other artifacts (PRDs, plans, ADRs,
issues, commits, diffs). Reference them by path or URL instead.

If the user passed arguments, treat them as a description of what the next
session will focus on and tailor the doc accordingly.

Make sure the overarching goal of the summarized session is clear so the next
agent can pick up the thread.
</guidelines>
