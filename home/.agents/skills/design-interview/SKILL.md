---
name: design-interview
description: Interview the user to align on a design before drafting the design doc.
disable-model-invocation: true
---

# Design Interview Session

Interview the user relentlessly about every aspect of the design to build a
shared understanding of **how** the change will be built. Walk down each branch
of the design tree, resolving dependencies between decisions one-by-one.

Ask questions one at a time, waiting for feedback on each before continuing. For
each question, provide your recommended answers.

If a question can be answered by exploring the codebase, explore the codebase
instead.

## Inputs

Before starting, read:

- `plan/{slug}/prd.md` — the PRD. Treat the **what / who / why** as settled;
  this session is about **how**.
- `GLOSSARY.md` (and relevant sub-glossaries) — use these terms verbatim.

If the PRD is missing, contested, or contradicts what the user is now saying,
stop and send the user back to `prd-interview` — don't relitigate product
decisions here.

## Stay at design altitude

This is an **engineering** alignment, not a product alignment. Steer every
question toward how the system is built and the trade-offs involved — not what
the user sees or why the feature exists.

In scope:

- Component / module boundaries and responsibilities.
- Interfaces, schemas, API contracts, data models.
- Data flow, state ownership, control flow across components.
- Tech choices: libraries, frameworks, storage, protocols.
- Engineering risks: scaling, performance, migration, dependencies, security,
  observability, failure modes.
- Backwards compatibility, rollout, and data migration strategy.
- Testing strategy at the system level.

Out of scope (defer elsewhere):

- Problem framing, target users, user stories, success criteria — those belong
  in the PRD.
- Per-task breakdown and acceptance criteria — those belong in `decompose`.

If the user starts relitigating product scope, gently steer back: "That sounds
like a PRD question — let's keep the PRD as the contract and stay on how we
build it."

## Ubiquitous Language

The project's ubiquitous language is defined in `GLOSSARY.md` files. The design
may introduce **technical** terms (component names, internal concepts) that
don't belong in the domain glossary — keep those in the design doc itself. Only
propose glossary changes for terms meaningful to domain experts.

## During the session

### Discuss concrete scenarios

When component relationships or data flows are being discussed, stress-test them
with specific scenarios. Walk a real request through the system. Probe edge
cases: concurrent writes, partial failures, retries, stale caches, permission
boundaries.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you
find a contradiction, surface it: "You said the worker holds the lock for the
whole batch, but `worker.go:120` releases it per-item — which is right?"

### Force trade-offs into the open

For every non-trivial design decision, ask what alternatives were considered and
why this one wins. If the user can't articulate the trade-off, that's a sign the
decision isn't actually made yet.

### Surface failure modes

For each component or interaction, ask: what happens when this fails, times out,
returns partial data, or is called concurrently? Push until each failure mode
has a defined behavior, not "it shouldn't happen."

### Pin down the contract

For each interface or schema discussed, push for the actual shape: field names,
types, nullability, error cases. "It returns the user" is not a contract; the
field list is.

## Persistence

`plan/{slug}/design_interview.md` is the session's source of truth — questions,
answers, proposed glossary changes — so the session can be paused, resumed, and
reviewed. Use the same `{slug}` as the PRD.

**Persist after every event.** Update `design_interview.md` immediately after:

- each user answer
- each proposed glossary change

Write to `design_interview.md` before asking the next question.

### design_interview.md format

```md
# {slug}

Brief summary of the design being discussed.

## Interview session

### Q1: Title of question

Additional question details (if needed).

**Answer:**

Answer details, including any follow-up questions that arose.

## Proposed glossary changes

### New term: **Term**

A concise definition of the new term.

### Updated term: **Term**

The proposed change to the existing definition.

### Removed term: **Term**

Reason for removing the term.
```

Only include the sections that apply.
