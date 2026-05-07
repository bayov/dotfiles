---
name: prd
description: Produce a PRD
disable-model-invocation: true
model: opus
effort: medium
---

Produce a PRD (Product Requirements Document): `$PLAN_DIR/{slug}/prd.md`.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset).

A PRD describes **what** is being built, **who** it's for, and **why** — at a
level both product managers and engineers can read. Avoid discussing
implementation details (modules, interfaces, schemas, API contracts, tech
choices).

## Input

Before writing, read whichever of these exist:

- `GLOSSARY.md` (and relevant sub-glossaries) — the project's existing
  ubiquitous language. Use these terms verbatim in the PRD.
- `$PLAN_DIR/{slug}/prd_interview.md` — the alignment session's Q&A and proposed
  glossary changes. Treat answers as authoritative; do not relitigate them.

## Steps

1. Confirm with the user: problem framing, target users, in/out of scope, and
   how success will be measured. If any of these are unresolved or contested,
   send them back through `prd-interview` — do not invent answers in the PRD.

2. Copy [template.md](./template.md) to `$PLAN_DIR/{slug}/prd.md` and fill out
   each section.
