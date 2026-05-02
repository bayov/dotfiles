---
name: decompose
description: Break a plan into tasks
disable-model-invocation: true
---

Break a plan into small vertical slices.

## What is a Vertical Slice?

A vertical slice is a small task that goes through all relevant layers of the
system end-to-end (e.g., schema, API, UI, tests) to produce a narrow but
complete piece of functionality.

Vertical slices ensure short feedback loops and reduce the risk of downstream
integration issues.

Avoid horizontal tasks like:

- build all database schemas
- build all API endpoints
- build all UI components
- write all tests

## Process

### 1. Decomposition

1. Start from user-visible behavior.
2. Cut scope to the smallest useful outcome.
3. Include every layer needed for that outcome.
4. Make tasks independent when possible.
5. Prefer thin slices over thick ones.
6. Prefer tasks that expose unknowns early.
7. Stop before splitting into disconnected layer work.

Present the proposed slice breakdown to the user for approval in the form of a
table.

| Task      | Blocked by               | Description      |
| --------- | ------------------------ | ---------------- |
| task_name | Names of dependent tasks | Task description |

> List tasks in dependency order (blockers first)

Ask the user:

- Does the granularity feel right? (too coarse / too fine)
- Are the dependency relationships correct?
- Should any slices be merged or split further?

Iterate until the user approves the breakdown.

### 2. Writing tasks

Write each of the user-approved tasks as a file
plan/{change_name}/tasks/{task_name}.md using the template below.

<task-template>
A concise description of this vertical slice. Describe the end-to-end behavior,
not layer-by-layer implementation.

## Blocked by

- Blocked by plan/{change_name}/tasks/{task_name}.md

> Omit the "Blocked by" section if there are no blockers

## Acceptance criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

</task-template>

## Examples

### Good (Vertical Slice)

```md
Implement the ability to archive and unarchive notes.

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
