---
name: align
description: Build a shared understanding and a ubiquitous language.
disable-model-invocation: true
---

# Interview Session

Interview the user relentlessly about every aspect of their plan until you reach
a shared understanding. Walk down each branch of the design tree, resolving
dependencies between decisions one-by-one. For each question, provide your
recommended answer.

Ask the questions one at a time, waiting for feedback on each question before
continuing.

If a question can be answered by exploring the codebase, explore the codebase
instead.

## Ubiquitous Language

The project's ubiquitous language is defined in `GLOSSARY.md` files. As part of
the session, you'll also build a precise, shared vocabulary for the domain.

Don't couple glossary entries to implementation details. Only include terms that
are meaningful to domain experts.

## During the session

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific
scenarios. Invent scenarios that probe edge cases and force the user to be
precise about the boundaries between concepts.

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

## Persistence

`plan/{slug}/align_session.md` is the session's source of truth — questions,
answers, proposed glossary changes — so the session can be paused, resumed, and
reviewed. Pick a short kebab-case `{slug}` that names the change being planned
(e.g. `plan/archive-notes/align_session.md`).

**Persist after every event.** Update `align_session.md` immediately after:

- each user answer
- each proposed glossary change

Write to `align_session.md` before asking the next question.

### align_session.md format

```md
# {slug}

Brief summary of plan being discussed.

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
