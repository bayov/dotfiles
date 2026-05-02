---
name: quick-task
description: Align on a small-scope change and write a self-contained task file.
disable-model-invocation: true
---

# Quick Task Session

For a change small enough to fit one vertical slice. Run a short interview, then
write `plan/{slug}/task.md`.

Interview the user relentlessly about every aspect of the task until you reach a
shared understanding.

Ask the questions one at a time, waiting for feedback on each question before
continuing. For each question, provide your recommended answers.

If a question can be answered by exploring the codebase, explore the codebase
instead.

## Scope of the interview

- The user-visible change in one or two sentences.
- Acceptance criteria (the checklist `implement` will work against).
- Where in the code this lives — file paths or component name.
- Riskiest behavior and how it will be tested (level + roughly what the test
  asserts).
- Anything explicitly out of scope that a reader might expect.

## Ubiquitous Language

Read `GLOSSARY.md` (and relevant sub-glossaries) before drafting. Use those
terms verbatim in the task description, acceptance criteria, and any test names
you suggest.

If a new domain term surfaces during the interview:

- If it's a trivial rename or a term already covered → propose the glossary
  delta inline and include it in the task's docs acceptance criterion.
- If it's load-bearing (a new concept domain experts will use) → stop and route
  to `plan-interview`. Don't invent ubiquitous language at this altitude.

## Cross-reference with code

When the user states how something works, check whether the code agrees. If you
find a contradiction, surface it before writing the task.

## Output

1. Pick a short kebab-case `{slug}` that names the change.
2. Copy [template.md](./template.md) to `plan/{slug}/task.md` and fill out each
   section.
3. Show the file to the user for approval. Iterate until approved.
