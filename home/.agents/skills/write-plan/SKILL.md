---
name: write-plan
description: Produce a combined plan doc (what / why / how / acceptance criteria) for changes small enough to fit one document.
disable-model-invocation: true
model: opus
effort: medium
---

Produce a plan doc: `$PLAN_DIR/{slug}/plan.md`.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset). **Glossary
> root:** `$GLOSSARY_DIR` env var (defaults to the project root if unset).

A plan doc bundles **what** is being built, **why**, and **how** — plus the
acceptance criteria `implement` will work against, in a single document.

If product scope is contested, or the design has many independent decisions, or
the change splits into more than ~3 independently-shippable slices, stop and
route the user to `prd-interview` → `write-prd` → `design-interview` →
`write-design` → `decompose` → `implement` instead — separate product and design
contracts pay off at that scale.

## Input

Before writing, read whichever of these exist:

- `$GLOSSARY_DIR/GLOSSARY.md` (and relevant sub-glossaries under
  `$GLOSSARY_DIR`) — use these terms verbatim.
- `$PLAN_DIR/{slug}/plan_interview.md` — the alignment session's Q&A and
  proposed glossary changes. Treat answers as authoritative; do not relitigate
  them.

## Steps

1. Confirm with the user: problem framing, the user-visible change, acceptance
   criteria, component boundaries, key interfaces, testing strategy, and
   rollout. If any of these are unresolved or contested, send them back through
   `plan-interview` — do not invent answers in the plan.

2. Copy [template.md](./template.md) to `$PLAN_DIR/{slug}/plan.md` and fill out
   each section.

## After the plan is approved

Run `decompose` against `plan.md` — usually one task file under
`$PLAN_DIR/{slug}/tasks/`, occasionally 2–3 if the change benefits from
independently-mergeable pieces. Then run `implement` per task, passing the task
file path as the argument. The plan doc serves as the design reference each task
points back to.

If decomposition would produce more than ~3 slices, that's a signal the change
has outgrown a single combined doc — route to `write-prd` + `write-design`
instead.

## Principles

- **Prefer deep modules.** Push complexity behind narrow interfaces. Justify
  each component by what its interface hides, not by what it contains. Don't
  introduce a seam unless something actually varies across it. (See the
  `write-design` skill for the full treatment.)
- **Testing is part of the design.** The plan must say how the change will be
  tested: riskiest behavior, test level, invariants, anything verified manually
  instead.
