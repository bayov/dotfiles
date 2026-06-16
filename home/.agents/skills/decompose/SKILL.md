---
name: decompose
description: Break a design into vertical-slice tasks
disable-model-invocation: true
model: opus
effort: high
---

Break an approved design into small vertical slices, each captured as a task
file under `$PLAN_DIR/{slug}/tasks/`.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset). **Glossary
> root:** `$GLOSSARY_DIR` env var (defaults to the project root if unset).

## Input

Read whichever of these exist under `$PLAN_DIR/{slug}/`:

- `prd.md` — user stories are the source of truth for what each slice advances.
- `design.md` — components, data model, data flow, migration plan are the
  engineering contract.
- `plan.md` — single combined doc covering both. Acceptance Criteria drive what
  each slice advances; Design / Migration are the engineering contract.

Also read `$GLOSSARY_DIR/GLOSSARY.md` (and relevant sub-glossaries under
`$GLOSSARY_DIR`) — use these terms verbatim in task names and descriptions.

Every entry in the **Glossary Changes** section of the input doc(s) must be
owned by the slice that introduces or changes the corresponding behavior.

If the design is missing or contested, stop and send the user back to the
relevant interview (`design-interview` or `plan-interview`) — do not invent
design answers in tasks.

## What is a Vertical Slice?

A vertical slice is a small task that goes through all relevant layers of the
system end-to-end (e.g., schema, API, UI, tests) to produce a narrow but
complete piece of user-visible behavior.

Vertical slices ensure short feedback loops and reduce the risk of downstream
integration issues.

A good slice:

- Advances at least one PRD user story.
- Touches every layer needed to ship that behavior — including docs.
- Is independently mergeable.
- Ships behind whichever flag/gate the design's Migration Plan specifies (if
  any).

Docs ship with the code that justifies them. A slice owns:

- Glossary entries for any domain term it introduces, renames, or removes (apply
  via `update-glossary`).
- Updates to `design.md` / `prd.md` if implementation surfaces drift from the
  contract.
- Any README, code example, or in-repo doc that references the behavior the
  slice changes.

Don't bundle all glossary or doc work into a separate "docs" slice — that
recreates the horizontal-slice problem.

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
| task-name | Names of dependent tasks | S1, S2       | Comp A, Comp B | Task description |

> List tasks in dependency order (blockers first).

### 2. Completeness check

Before asking for approval, verify:

- Every PRD user story is advanced by at least one slice (or is explicitly out
  of scope for this plan).
- Every design component that needs new behavior appears in at least one slice.
- Every entry in the prd.md / design.md **Glossary Changes** sections is owned
  by exactly one slice (or is explicitly deferred).
- The dependency graph is acyclic and every blocker exists in the table.

List any uncovered stories, components, or glossary terms as Open Questions for
the user rather than silently dropping them.

Ask the user:

- Does the granularity feel right? (too coarse / too fine)
- Are the dependency relationships correct?
- Should any slices be merged or split further?

Iterate until the user approves the breakdown.

### 3. Writing tasks

For each user-approved task, copy [template.md](./template.md) to
`$PLAN_DIR/{slug}/tasks/{NN}-{task_name}.md` and fill out each section. Keep the
`linear` frontmatter field untouched — `/linear-sync` fills it with the created
issue's link.

Prefix each filename with a zero-padded number (`01-`, `02-`, ...) reflecting
the natural execution order from the dependency table. Tasks that can run in
parallel still get sequential numbers — pick an order that respects blockers.

### 4. Report omissions

After writing all task files, report any decision-relevant information from the
design or PRD that was omitted from task files to meet the word limit, so the
user can decide whether to expand a task or capture it elsewhere.

## Examples

### Good (Vertical Slice)

```md
Implement the ability to archive and unarchive notes.

## References

- PRD: S3, S4 ($PLAN_DIR/archive-notes/prd.md)
- Design: NoteStore, NotesListView ($PLAN_DIR/archive-notes/design.md)

## Acceptance criteria

- [ ] An existing note can be archived.
- [ ] An existing archived note can be unarchived.
- [ ] An archived note no longer appears in the default notes list.
- [ ] An unarchived note appears in the default notes list again.
- [ ] `$GLOSSARY_DIR/GLOSSARY.md` defines **Archived note** (per design.md
      Glossary Changes).
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
