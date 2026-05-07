---
name: linear-sync-plan
description: Sync plan artifacts to Linear.
argument-hint: [slug]
disable-model-invocation: true
model: sonnet
effort: medium
---

Sync `$PLAN_DIR/{slug}/` markdown artifacts into Linear so they persist beyond
the local working directory.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset).

The repo's `$PLAN_DIR/` is intentionally uncommitted scratch. Linear is the
system of record. This skill is the one-way bridge `$PLAN_DIR/{slug}/ → Linear`.

## Invocation

- With argument: `/linear-sync-plan archive-notes` operates on
  `$PLAN_DIR/archive-notes/`.
- Without argument: must be run from inside a `$PLAN_DIR/{slug}/` directory;
  infer slug from the directory name.

If the slug doesn't resolve to an existing `$PLAN_DIR/{slug}/` directory, stop
and ask the user.

## Linear shape

For a slug `archive-notes`:

```
[archive-notes] Archive notes              ← parent issue (team FW)
├── Documents (attached to parent)
│     ├── PRD                              ← prd.md
│     ├── Design                           ← design.md
│     └── Plan                             ← plan.md (only if no prd/design)
└── Sub-issues
      ├── [archive-notes] Archive a note   ← tasks/archive_a_note.md
      └── [archive-notes] Unarchive a note ← tasks/unarchive_a_note.md
```

Defaults baked in:

- Team: `FW` (AI Security for Apps)
- Assignee on every issue: `hagai.akibayov@catonetworks.com`
- No Linear Project, no Linear labels, no cycle assignment
- Parent issue state is never set explicitly — it rolls up from sub-issues

## What to push, what to skip

| File                           | Linear destination                 |
| ------------------------------ | ---------------------------------- |
| `prd.md`                       | Document "PRD" on parent issue     |
| `design.md`                    | Document "Design" on parent issue  |
| `plan.md`                      | Document "Plan" on parent issue    |
| `tasks/<name>.md`              | Sub-issue under parent             |
| `*_interview.md`               | **skip** (local-only scratch)      |
| `GLOSSARY.md` / sub-glossaries | **skip** (repo-wide, not per-slug) |

`prd.md`+`design.md` and `plan.md` are mutually exclusive in the workflow — if
both somehow exist, push all three and warn the user.

## Idempotency: `$PLAN_DIR/{slug}/.linear.json`

The mapping file is the source of truth for "which Linear records belong to this
slug." It is gitignored along with the rest of `$PLAN_DIR/`. Schema:

```json
{
    "team": "FW",
    "parent_issue": "FW-100",
    "documents": {
        "prd": "<doc-uuid>",
        "design": "<doc-uuid>",
        "plan": "<doc-uuid>"
    },
    "tasks": { "archive_a_note.md": "FW-101", "unarchive_a_note.md": "FW-102" },
    "hashes": {
        "prd.md": "<sha256>",
        "design.md": "<sha256>",
        "plan.md": "<sha256>",
        "tasks/archive_a_note.md": "<sha256>",
        "tasks/unarchive_a_note.md": "<sha256>"
    }
}
```

If the file is missing, this is a first-time push: create everything, write the
file at the end.

## Steps

### 1. Load state

Read `$PLAN_DIR/{slug}/.linear.json` if present. Compute `sha256` for every file
in the "What to push" table that exists locally.

### 2. Resolve the parent issue

- **First push** (no `.linear.json`): the parent doesn't exist yet — create it
  in step 4 after gathering content.
- **Subsequent push**: call the Linear MCP to fetch the parent issue. If it
  returns 404, the user deleted it manually. Stop and prompt:
  - **Recreate**: clear all child IDs from `.linear.json` and treat as a
    first-time push (will recreate every Document and sub-issue too).
  - **Abort**: exit without changes. Don't pick on the user's behalf.

### 3. Compute the diff

For each tracked file, compare the current sha256 to the stored hash. Bucket
each file into one of:

- **new** — exists locally, not in `.linear.json`
- **changed** — hash differs
- **unchanged** — hash matches (skip, no Linear writes)
- **gone** — listed in `.linear.json` but no longer present locally

For each `changed` and `new` file, present a short summary (file path, old→new
hash, line-count delta if cheap) and ask the user to confirm the push. A blanket
"push everything changed? [Y/n]" is fine when the set is small (≤3 files);
itemize when larger.

For `gone` files (only relevant for tasks — docs are mandatory):

- The Linear sub-issue is **left in place** (it may have comments, PR links,
  etc. that must not be discarded).
- Warn the user and prompt to remove the entry from `.linear.json`'s `tasks` and
  `hashes` so it stops being tracked. Default to keeping the entry until the
  user confirms removal.

### 4. Write to Linear

Use the `mcp__claude_ai_Linear__*` tools. Do writes in this order so child
records always have a parent:

1. **Parent issue** (`save_issue`):
   - Title: `[<slug>] <H1 from prd.md or plan.md>`
   - Description: a brief index — one line per Document title and one line per
     sub-issue title with its Linear identifier. Regenerate this index every
     push so it stays accurate.
   - Team: `FW`. Assignee: `hagai.akibayov@catonetworks.com`.
   - Don't set state.

2. **Documents** (`save_document`): for each pushed doc, create or update with
   `issue: <parent-issue-id>`, `title` from the table above, `content` =
   verbatim file body. Store the returned UUID in `.linear.json`.

3. **Sub-issues** (`save_issue`): for each `tasks/*.md`:
   - Title: `[<slug>] <H1 from task file>`
   - Parent: parent issue ID
   - Team: `FW`. Assignee: `hagai.akibayov@catonetworks.com`.
   - Description: the task file body, with **path-style references rewritten to
     Linear Document URLs** (see "Reference rewriting" below).
   - State: derived from acceptance-criteria checkboxes (see "State derivation"
     below).
   - Markdown checkboxes (`- [ ]` / `- [x]`) stay verbatim — Linear renders them
     as native interactive checkboxes.

4. **Blocked-by relations**: for each task file with a `Blocked by:` line,
   resolve each named blocker to its Linear sub-issue ID via `.linear.json` and
   create the corresponding `blocked-by` issue relation. If a blocker name
   doesn't resolve (typo, slice not yet pushed), warn and skip — don't
   fabricate.

### 5. Persist state

Update `.linear.json` with new IDs and the fresh hashes for every file actually
pushed. Don't update hashes for files that weren't pushed (e.g., user declined
the prompt) — the next run should still see them as changed.

## Reference rewriting

Task files commonly contain references like:

```
## References
- Design: NoteStore, NotesListView ($PLAN_DIR/archive-notes/design.md)
- PRD: S3, S4 ($PLAN_DIR/archive-notes/prd.md)
```

Inside the Linear sub-issue description (only — leave the local file untouched),
rewrite each `($PLAN_DIR/{slug}/<name>.md)` to a markdown link pointing at the
corresponding Linear Document URL from `.linear.json`. The Linear Document URL
format is `https://linear.app/hakasecurity/document/<slug-or-id>` — prefer
fetching the URL from the `save_document` response when available.

If a reference points at a file the skill doesn't push (`*_interview.md`,
`GLOSSARY.md`), strip the parenthetical entirely rather than leaving a dead
relative path.

## State derivation

Look for the `## Acceptance criteria` section in each task file. Count top-level
checkboxes:

- 0 of N checked → **Backlog**
- 1..(N-1) of N checked → **In Progress**
- N of N checked → **Done**

Only checkboxes inside `## Acceptance criteria` count. Checkboxes elsewhere in
the file (e.g., `## Out of scope`) are informational and ignored.

If the file has no `## Acceptance criteria` section or zero checkboxes, default
to **Backlog** and warn the user.

To set the right state via `save_issue`, first call `list_issue_statuses` for
team `FW` to resolve the state names to IDs. Cache the IDs for the duration of
the run; they're stable enough that re-fetching every push is fine.

## Sync direction

This is one-way: file → Linear. If the user toggled a checkbox in the Linear UI
or edited a Document in Linear, the next push **silently overwrites** those
edits with the file content. This is by design — two-way merge would be costly
to get right and the workflow assumes the local files are authoritative.

If you notice (e.g., from a `get_document` call) that Linear content has
diverged from the local hash's expected state in a way that would lose user
work, surface it before overwriting. But don't routinely diff — trust the hash.

## Don'ts

- Don't archive issues automatically when their state goes Done. Archival is
  destructive enough that the user should do it manually.
- Don't auto-assign to a cycle. The skill is a persistence sink, not a planner.
- Don't apply Linear labels. The parent/sub-issue tree already conveys grouping.
- Don't push interview files or the glossary, even if asked inline — those
  decisions were made deliberately. If the user wants them, route them through a
  follow-up edit to this skill.
- Don't read Linear-side checkbox/description edits back into local files.
