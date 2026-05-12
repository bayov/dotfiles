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

1. Confirm with the user: problem framing, the user-visible change, and
   acceptance criteria. If any of these are unresolved or contested, send them
   back through `plan-interview` — do not invent answers in the plan.

2. Copy [template.md](./template.md) to `$PLAN_DIR/{slug}/plan.md` and fill it
   out. Follow the template's writing rules: bullets only outside the Problem
   paragraph, ≤15 words per bullet, omit optional sections rather than leaving
   empty headings. The **Notes** section is the only place for decisions / risks
   / testing approach / rollout — add a bullet there only if it would surprise a
   reviewer or change the AC.

3. Report any decision-relevant information from the inputs (interview Q&A,
   glossary) that was omitted to meet the word limit, so the user can decide
   whether to expand the doc or capture it elsewhere.

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

- **Keep it short.** The plan is for a change small enough to fit one document.
  If a section needs paragraphs of justification, the change has outgrown
  `write-plan` — route to `write-prd` + `write-design`.
- **AC is the spine.** Everything else exists to justify the AC. If a section
  doesn't change the AC, omit it.
- **Prefer deep modules** when the change introduces a new seam. Justify the
  boundary by what the interface hides. (See `write-design` for the full
  treatment — record only the result here, as a Notes bullet if non-obvious.)
- **Testing belongs in the AC** when it's the standard shape ("covered by
  integration test for X"). Use a Notes bullet only if the approach is
  non-obvious or risky.
