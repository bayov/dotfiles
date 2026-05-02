---
name: prd
description: Produce a PRD
disable-model-invocation: true
---

Produce a PRD (Product Requirements Document): `plan/{slug}/prd.md`.

A PRD describes **what** is being built, **who** it's for, and **why** — at a
level both product managers and engineers can read. Avoid discussing
implementation details (modules, interfaces, schemas, API contracts, tech
choices).

## Input

Before writing, read whichever of these exist:

- `GLOSSARY.md` (and relevant sub-glossaries) — the project's existing
  ubiquitous language. Use these terms verbatim in the PRD.
- `plan/{slug}/align_session.md` — the alignment session's Q&A and proposed
  glossary changes. Treat answers as authoritative; do not relitigate them.

## Steps

1. Confirm with the user: problem framing, target users, in/out of scope, and
   how success will be measured. If any of these are unresolved or contested,
   send them back through `align` — do not invent answers in the PRD.

2. Copy [template.md](./template.md) to `plan/{slug}/prd.md` and fill out each
   section.
