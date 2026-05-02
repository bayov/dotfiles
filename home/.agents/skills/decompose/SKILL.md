---
name: decompose
description: Break a design into vertical-slice tasks
disable-model-invocation: true
---

Break an approved design into small vertical slices, each captured as a task
file under `plan/{slug}/tasks/`.

## Input

Before decomposing, read whichever of these exist:

- `plan/{slug}/prd.md` — the product contract. User stories here are the source
  of truth for what each slice must advance.
- `plan/{slug}/design.md` — the engineering contract. Components, data model,
  data flow, and migration plan are authoritative; do not relitigate them.
- `GLOSSARY.md` (and relevant sub-glossaries) — use these terms verbatim in task
  names and descriptions.

If the design is missing or contested, stop and send the user back to
`design-interview` — do not invent design answers in tasks.

## What is a Vertical Slice?

A vertical slice is a small task that goes through all relevant layers of the
system end-to-end (e.g., schema, API, UI, tests) to produce a narrow but
complete piece of user-visible behavior.

Vertical slices ensure short feedback loops and reduce the risk of downstream
integration issues.

A good slice:

- Advances at least one PRD user story.
- Touches every layer needed to ship that behavior.
- Is independently mergeable.
- Ships behind whichever flag/gate the design's Migration Plan specifies (if
  any).

Avoid horizontal tasks like:

- build all database schemas
- build all API endpoints
- build all UI components
- write all tests

## Process

### 1. Decomposition

1. Start from user-visible behavior (PRD user stories).
2. Cut scope to the smallest useful outcome.
3. Include every layer needed for that outcome.
4. Make tasks independent when possible.
5. Prefer thin slices over thick ones.
6. Prefer tasks that expose unknowns early.
7. Stop before splitting into disconnected layer work.

Present the proposed slice breakdown to the user for approval in the form of a
table.

| Task      | Blocked by               | User stories | Components     | Description      |
| --------- | ------------------------ | ------------ | -------------- | ---------------- |
| task_name | Names of dependent tasks | S1, S2       | Comp A, Comp B | Task description |

> List tasks in dependency order (blockers first).

### 2. Completeness check

Before asking for approval, verify:

- Every PRD user story is advanced by at least one slice (or is explicitly out
  of scope for this plan).
- Every design component that needs new behavior appears in at least one slice.
- The dependency graph is acyclic and every blocker exists in the table.

List any uncovered stories or components as Open Questions for the user rather
than silently dropping them.

Ask the user:

- Does the granularity feel right? (too coarse / too fine)
- Are the dependency relationships correct?
- Should any slices be merged or split further?

Iterate until the user approves the breakdown.

### 3. Writing tasks

For each user-approved task, copy [template.md](./template.md) to
`plan/{slug}/tasks/{task_name}.md` and fill out each section.

## Examples

### Good (Vertical Slice)

```md
Implement the ability to archive and unarchive notes.

## References

- PRD: S3, S4 (plan/archive-notes/prd.md)
- Design: NoteStore, NotesListView (plan/archive-notes/design.md)

## Acceptance criteria

- [ ] An existing note can be archived.
- [ ] An existing archived note can be unarchived.
- [ ] An archived note no longer appears in the default notes list.
- [ ] An unarchived note appears in the default notes list again.
```

### Bad (Horizontal Slice)

```md
Implement the database schema for notes, including support for create,
archive/unarchive, and delete.

## Acceptance criteria

- [ ] Tables or columns exist for notes.
- [ ] Schema supports state needed by create, archive/unarchive, and delete.
- [ ] Migrations run successfully.
```
