---
name: linear-sync-plan
description: Sync all plan artifacts under PLAN_DIR to Linear in one pass.
disable-model-invocation: true
model: sonnet
effort: medium
---

Sync every `$PLAN_DIR/{slug}/` markdown artifact into Linear so they persist
beyond the local working directory.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset).

Linear is the system of record. This skill is the one-way bridge
`$PLAN_DIR/* → Linear`.

## Invocation

No arguments. Run from anywhere; the skill enumerates every direct child
directory of `$PLAN_DIR/` and treats each as a slug.

Skip entries that are not directories or whose name starts with `.` or `_`.

## Linear shape

For a slug `archive-notes`:

```
[archive-notes] Archive notes              ← parent issue
├── Documents (attached to parent)
│     ├── PRD                              ← prd.md
│     ├── Design                           ← design.md
│     └── Plan                             ← plan.md (only if no prd/design)
└── Sub-issues
      ├── [archive-notes/01] Archive a note   ← tasks/archive_a_note.md
      └── [archive-notes/02] Unarchive a note ← tasks/unarchive_a_note.md
```

Defaults baked in:

- Assignee on every issue: `hagai.akibayov@catonetworks.com`
- No Linear Project, no Linear labels, no cycle assignment
- Parent and sub-issue states are managed by the skill — see "State derivation"
  below

Team defaults to `FW` (AI Security for Apps) for new slugs. Existing slugs keep
the team of their current parent issue. Fetches are **not** team-scoped — the
skill discovers any plan parent issue regardless of team so manually moved
issues stay tracked.

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

## Idempotency: `$PLAN_DIR/.linear.json`

A single mapping file at the root of `$PLAN_DIR/` is the source of truth for
"which Linear records belong to which slug." Schema:

```json
{
    "slugs": {
        "archive-notes": {
            "team": "FW",
            "parent_issue": "FW-100",
            "documents": {
                "prd": "<doc-uuid>",
                "design": "<doc-uuid>",
                "plan": "<doc-uuid>"
            },
            "tasks": {
                "archive_a_note.md": "FW-101",
                "unarchive_a_note.md": "FW-102"
            },
            "hashes": {
                "prd.md": "<sha256>",
                "design.md": "<sha256>",
                "plan.md": "<sha256>",
                "tasks/archive_a_note.md": "<sha256>",
                "tasks/unarchive_a_note.md": "<sha256>"
            }
        }
    }
}
```

If the file is missing, every slug is treated as a first-time push.

## Steps

### 1. Load state

Read `$PLAN_DIR/.linear.json` if present. Enumerate `$PLAN_DIR/*/` to find all
local slugs. For each slug, compute `sha256` for every file in the "What to
push" table that exists locally.

### 2. Fetch Linear state in one pass

To minimize API calls, do a **single** `list_issues` with `assignee` =
`hagai.akibayov@catonetworks.com`, no team filter, including sub-issues and
attachments. Build an in-memory index:

- by Linear issue ID → issue
- by `[<slug>]` title prefix → parent issue
- by `[<slug>/<NN>]` title prefix → sub-issue

Call `list_issue_statuses` lazily, **once per team** encountered in step 4, and
cache each team's name → status-ID map. Status name → ID mapping is per-team in
Linear.

Reconcile against `.linear.json`:

- For each slug in `.linear.json`, verify `parent_issue` still exists in the
  fetched set. If a parent is missing (user deleted it manually), prompt:
  - **Recreate**: clear that slug's entry from `.linear.json` and treat it as a
    first-time push.
  - **Skip**: leave the slug's local state alone and exclude it from this run.
    Default to skip; never pick for the user silently.
- For each sub-issue ID listed in `.linear.json` but missing from the fetch,
  warn and drop the entry from `tasks`/`hashes` after user confirmation.

Documents are not returned by `list_issues`; rely on the stored UUIDs in
`.linear.json` and only call `get_document` lazily if a divergence check is
warranted.

### 3. Compute the diff per slug

For each local slug, bucket every tracked file:

- **new** — exists locally, not in `.linear.json`
- **changed** — hash differs from `.linear.json`
- **unchanged** — hash matches (skip, no Linear writes)
- **gone** — listed in `.linear.json` but no longer present locally

**new** and **changed** files are pushed without prompting. Report what was
pushed after the fact (slug, file path, change summary).

For **gone** task files: the Linear sub-issue is **left in place** (it may have
comments, PR links, etc. that must not be discarded). Warn and prompt to remove
the entry from `.linear.json`. Default to keeping the entry until confirmed.

Slugs whose every file is unchanged are skipped entirely.

### 4. Write to Linear

Iterate slugs that have any new/changed file. Use the `mcp__claude_ai_Linear__*`
tools. Order per slug so child records always have a parent:

1. **Parent issue** (`save_issue`):
   - Title: `[<slug>] <H1 from prd.md or plan.md>`
   - Description: summary of the slug's purpose (from prd.md or plan.md intro).
     Documents and sub-issues appear automatically in the Linear UI — do not add
     `# Documents` or `# Sub-issues` index sections.
   - Team: existing slugs reuse their parent's team; new slugs default to `FW`.
     Persist the resolved team in `slugs.<slug>.team`.
   - Assignee: `hagai.akibayov@catonetworks.com`.
   - State: derived from sub-issue states (see "Parent state derivation" below).
     On first push, default to **Todo**.

2. **Documents** (`save_document`): for each pushed doc, create or update with
   `issue: <parent-issue-id>`, `title` from the table above, `content` =
   verbatim file body. Store the returned UUID in `.linear.json`.

3. **Sub-issues** (`save_issue`): for each `tasks/*.md`:
   - Title: `[<slug>/<zero-padded-index>] <H1 from task file>` where the index
     is the 1-based position of the task file in alphabetical sort order of
     `tasks/*.md`, zero-padded to two digits (e.g. `01`, `02`)
   - Parent: parent issue ID
   - Team: same as parent. Assignee: `hagai.akibayov@catonetworks.com`.
   - Description: the task file body, with **path-style references rewritten to
     Linear Document URLs** (see "Reference rewriting" below).
   - State: see "State derivation" below — GitHub-linked tasks are left
     untouched; others derive from acceptance-criteria checkboxes.
   - Markdown checkboxes (`- [ ]` / `- [x]`) stay verbatim — Linear renders them
     as native interactive checkboxes.

4. **Blocked-by relations**: for each task file with a `Blocked by:` line,
   resolve each named blocker to its Linear sub-issue ID via `.linear.json` (any
   slug — cross-slug blockers are allowed) and create the corresponding
   `blocked-by` issue relation. If a blocker name doesn't resolve (typo, slice
   not yet pushed), warn and skip — don't fabricate. Defer blocked-by writes
   until **all** slugs have been written in this run so cross-slug references
   resolve.

### 5. Persist state

Update `$PLAN_DIR/.linear.json` with new IDs and the fresh hashes for every file
actually pushed. Don't update hashes for files that weren't pushed (e.g., user
declined the prompt) — the next run should still see them as changed.

Write the file once at the end of the run, not per slug.

## Reference rewriting

Task files commonly contain references like:

```
## References
- Design: NoteStore, NotesListView ($PLAN_DIR/archive-notes/design.md)
- PRD: S3, S4 ($PLAN_DIR/archive-notes/prd.md)
```

Inside the Linear sub-issue description (only — leave the local file untouched),
rewrite each `($PLAN_DIR/{slug}/<name>.md)` to a markdown link pointing at the
corresponding Linear Document URL from `.linear.json`. Cross-slug references are
allowed; resolve via `slugs.<slug>.documents`. The Linear Document URL format is
`https://linear.app/hakasecurity/document/<slug-or-id>` — prefer fetching the
URL from the `save_document` response when available.

If a reference points at a file the skill doesn't push (`*_interview.md`,
`GLOSSARY.md`), strip the parenthetical entirely rather than leaving a dead
relative path.

## State derivation

Resolve status names through the per-team cache populated in step 4 (each team's
statuses are fetched once on first use).

### Sub-issue state (tasks)

**GitHub-linked tasks** (sub-issue has any GitHub PR attachment, e.g. from
`create_attachment` or Linear's GitHub integration): **do not touch state** —
Linear's GitHub integration updates it automatically. Skip the state field in
the `save_issue` call.

**Non-GitHub tasks** (no PR attachment — typically non-code rollout, ops, or
research tasks): derive state from top-level checkboxes inside
`## Acceptance criteria`:

- 0 of N checked → **Todo**
- 1..(N-1) of N checked → **In Progress**
- N of N checked → **Done**

Only checkboxes inside `## Acceptance criteria` count. Checkboxes elsewhere
(e.g., `## Out of scope`) are informational and ignored. If the section is
missing or has zero checkboxes, default to **Todo** and warn.

### Parent state derivation

The parent issue's state is set explicitly by the skill (it tracks the overall
plan/prd/design lifecycle). Derive from sub-issue states _after_ sub-issues have
been written in step 4.3:

- All sub-issues **Todo** → parent **Todo**
- Any sub-issue **In Progress**, or mixed Todo/Done → parent **In Progress**
- All sub-issues **Done**, and parent has at least one open (non-merged) GitHub
  PR attachment → parent **In Review**
- All sub-issues **Done**, and no open PR attachments remain → parent **Done**

If the parent has no sub-issues yet (first push with only docs), default to
**Todo**.

The parent's state is re-evaluated on every push — bumping a sub-issue to Done
may roll the parent forward. Issue this `save_issue` update _after_ the
sub-issue writes in step 4.3 so it reflects the latest sub-issue state.

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
- Don't issue a per-slug `list_issues` — the run uses a single fetch in step 2.
