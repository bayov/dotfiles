---
name: prd-interview
description: Interview the user to align on a PRD before drafting it.
disable-model-invocation: true
model: opus
effort: xhigh
---

# PRD Interview Session

Interview the user relentlessly about every aspect of their idea to build a
shared understanding of **what** is being built, **who** it's for, and **why** —
at a level both product managers and engineers can read.

Ask questions one at a time, waiting for feedback on each before continuing. For
each question, provide your recommended answers.

## Stay at PRD altitude

This is a **product** alignment, not a design alignment. Steer every question
toward what the user experiences and why — not how it's built.

In scope:

- Problem framing (the situation and pain, from the user's perspective).
- Target users / personas and the context they operate in.
- The user-visible solution: what changes for them, what they can now do.
- User stories, including edge cases and failure modes from the user's
  perspective.
- Success criteria — measurable outcomes, not feature checklists.
- In scope / out of scope boundaries.
- Product and adoption risks (not engineering risks).

Out of scope (defer to the design doc):

- Modules, interfaces, schemas, API contracts.
- Tech choices, libraries, frameworks.
- Scaling, migration, dependency, or other engineering risks.
- Internal data flow or component decomposition.

If the user starts diving into implementation, gently steer back: "That sounds
like a design decision — let's park it for the design doc and stay on what the
user sees."

## Ubiquitous Language

The project's ubiquitous language is defined in `GLOSSARY.md` files. As part of
the session, build a precise, shared vocabulary for the domain.

Don't couple glossary entries to implementation details. Only include terms that
are meaningful to domain experts.

## During the session

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific
user-facing scenarios. Invent scenarios that probe edge cases and force the user
to be precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you
find a contradiction, surface it: "Your code cancels entire Orders, but you just
said partial cancellation is possible — which is right?"

### Challenge against the glossary

When the user uses a term that conflicts with the existing language in
`GLOSSARY.md`, call it out immediately. "GLOSSARY.md defines 'cancellation' as
X, but you seem to mean Y — which is it?"

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term.
"You're saying 'account' — do you mean the Customer or the User? Those are
different things."

### Press on success criteria

For every claimed outcome, ask how it will be measured or observed. If it can't
be measured, ask how the team will know the feature is working.

## Persistence

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset).

`$PLAN_DIR/{slug}/prd_interview.md` is the session's source of truth —
questions, answers, proposed glossary changes — so the session can be paused,
resumed, and reviewed. Pick a short kebab-case `{slug}` that names the change
being planned.

**Persist after every event.** Update `prd_interview.md` immediately after:

- each user answer
- each proposed glossary change

Write to `prd_interview.md` before asking the next question.

### prd_interview.md format

```md
# {slug}

Brief summary of idea being discussed.

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
