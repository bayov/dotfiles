---
name: plan-interview
description: Interview the user to align on a medium-scope change before drafting the plan. Combines product (what/why) and design (how) in one session.
disable-model-invocation: true
model: opus
effort: high
---

# Plan Interview Session

Interview the user relentlessly to build a shared understanding of **what** is
being built, **why**, and **how**. Walk down each branch of the design tree,
resolving dependencies between decisions one-by-one.

Ask questions one at a time, waiting for feedback on each before continuing. For
each question, provide your recommended answers.

If a question can be answered by exploring the codebase, explore the codebase
instead.

## Scope of this session

Covered briefly, not exhaustively:

- Problem and target user (one or two sentences each).
- The user-visible change and the acceptance criteria that prove it.
- Component boundaries and key interfaces touched.
- Data model / schema changes.
- Testing strategy: riskiest behaviors and the level they're tested at.
- Rollout: deploy ordering, gates, rollback, backwards compatibility.

Out of scope at this altitude:

- Pressure-testing every component boundary in isolation.
- Exhaustive failure-mode enumeration — cover the riskiest one or two.
- Separate persona / user-story enumeration — one paragraph of context is
  enough.

## Ubiquitous Language

The project's ubiquitous language is defined in `GLOSSARY.md` files. If a new
domain term surfaces, propose it as a glossary change. If the user uses a term
that conflicts with existing language, call it out.

Don't couple glossary entries to implementation details. Only include terms
meaningful to domain experts.

## During the session

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you
find a contradiction, surface it.

### Pin down the contract

For each interface or schema touched, push for the actual shape: field names,
types, error cases. "It returns the user" is not a contract; the field list is.

### Pin down testing

For the slice's riskiest behavior, ask: what test pins it down, at what level,
and why? What can't be tested automatically, and how is it verified instead?

### Pin down rollout

For each schema or contract change, ask: deploy ordering, backwards
compatibility window, rollback path. If a gate is needed: what it gates, what
flips it forward, what flips it back, who removes it.

## Session readiness

Before ending, confirm each item below has an answer or is captured as an open
question:

- Problem and target user (brief).
- The user-visible change.
- Acceptance criteria (the checklist `implement` will work against).
- Components and interfaces touched, with key contract shapes.
- Data model changes.
- Testing strategy: riskiest behavior + test level.
- Rollout: deploy ordering, gates, rollback.
- Open questions with owners.

If a load-bearing question is still open, leave it in Open Questions rather than
inventing an answer.

## Persistence

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset).

`$PLAN_DIR/{slug}/plan_interview.md` is the session's source of truth —
questions, answers, proposed glossary changes — so the session can be paused,
resumed, and reviewed. Pick a short kebab-case `{slug}` that names the change.

**Persist after every event.** Update `plan_interview.md` immediately after:

- each user answer
- each proposed glossary change

Write to `plan_interview.md` before asking the next question.

### plan_interview.md format

```md
# {slug}

Brief summary of the change being discussed.

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
