---
name: design-interview
description: Interview the user to align on a design before drafting the design doc.
disable-model-invocation: true
model: opus
effort: xhigh
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

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset). **Glossary
> root:** `$GLOSSARY_DIR` env var (defaults to the project root if unset).

- `$PLAN_DIR/{slug}/prd.md` — the PRD. Treat the **what / who / why** as
  settled; this session is about **how**.
- `$GLOSSARY_DIR/GLOSSARY.md` (and relevant sub-glossaries under
  `$GLOSSARY_DIR`) — use these terms verbatim.

If the PRD is missing, contested, or contradicts what the user is now saying,
stop and send the user back to `prd-interview` — don't relitigate product
decisions here.

## Stay at design altitude

This is an **engineering** alignment, not a product alignment. Steer every
question toward how the system is built and the trade-offs involved — not what
the user sees or why the feature exists.

Out of scope:

- Problem framing, target users, user stories, success criteria — those belong
  in the PRD.
- Per-task breakdown and acceptance criteria — those belong in `decompose`.

If the user starts relitigating product scope, gently steer back: "That sounds
like a PRD question — let's keep the PRD as the contract and stay on how we
build it."

## Ubiquitous Language

The project's ubiquitous language is defined in `GLOSSARY.md` files under
`$GLOSSARY_DIR`. The design may introduce **technical** terms (component names,
internal concepts) that don't belong in the domain glossary — keep those in the
design doc itself. Only propose glossary changes for terms meaningful to domain
experts.

## During the session

### Walk concrete scenarios

When component relationships or data flows are being discussed, walk a real
request through the system end-to-end. Probe failure modes: timeouts, partial
data, concurrent writes, retries, stale caches, permission boundaries. Push
until each failure mode has a defined behavior, not "it shouldn't happen."

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you
find a contradiction, surface it: "You said the worker holds the lock for the
whole batch, but `worker.go:120` releases it per-item — which is right?"

### Force trade-offs into the open

For every non-trivial design decision, ask what alternatives were considered and
why this one wins. If the user can't articulate the trade-off, that's a sign the
decision isn't actually made yet.

### Pressure-test boundaries

For each component the user proposes, ask: what does its interface hide that
callers don't need to know? If we deleted it, would complexity vanish, or
reappear across N callers? Is there a second implementation, or just a
hypothetical seam? Don't accept a decomposition just because each part looks
clean in isolation — check whether callers must thread the seams between them.

### Pin down the contract

For each interface or schema discussed, push for the actual shape: field names,
types, nullability, error cases. "It returns the user" is not a contract; the
field list is.

### Pin down testing

For each component or flow agreed, ask: what's the riskiest behavior, and what
test pins it down? At what level — unit, integration, end-to-end — and why? What
invariants must always hold (concurrency, retries, migration paths)? What can't
be tested automatically, and how will that be verified instead?

### Pin down rollout

For each schema, contract, or storage change, ask: what's the deploy ordering?
Is there a backwards-compatibility window? How do we roll back, and what state
can't be rolled back?

Does this need a gate at all? If yes: what behavior does it gate, what state
does it ship in, what flips it forward, what flips it back, and — if it's not
meant to be permanent — what triggers its removal and who owns that?

## Session readiness

Before ending, confirm each item below has an answer or is captured as an open
question:

- Context: background, current state, constraints, stakeholders.
- Engineering goals and non-goals.
- Component boundaries with interfaces, what they hide, and dependencies.
- Data model changes.
- Data flow end-to-end, including at least one failure path.
- Each non-trivial decision with alternatives considered.
- Testing strategy: risk map, levels, invariants, manual checks.
- Engineering risks / trade-offs with mitigations.
- Migration plan: deploy ordering, backwards compat, rollout controls, rollback.
- Observability: signals tied to failure modes.
- Open questions with owners.

If a load-bearing question is still open, leave it in Open Questions rather than
inventing an answer.

## Persistence

`$PLAN_DIR/{slug}/design_interview.md` is the session's source of truth —
questions, answers, proposed glossary changes — so the session can be paused,
resumed, and reviewed. Use the same `{slug}` as the PRD.

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
