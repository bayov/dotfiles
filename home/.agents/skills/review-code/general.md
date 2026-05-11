---
axis: general
---

# General review

Holistic, lens-free review of the diff. The other axes look through narrow
specialist lenses; this axis is the catch-all that flags anything a thoughtful
human reviewer would notice on a first read and that no specialist axis owns.

## What to look for

- **Coherence**: does the change tell a coherent story? Is the goal clear from
  the diff and the commit message(s)? Surprising scope creep, half-finished
  edits, dead code added "for later."
- **Reasoning gaps**: places where the diff makes a non-obvious choice with no
  comment, commit message, or test name that explains why. Future-you (or
  another reader) will not understand the intent.
- **Readability** at the diff level: confusingly named identifiers, functions
  doing two unrelated things, control flow that requires a second read.
- **Cross-file consistency**: similar concerns handled differently in different
  files of the same diff (e.g. one call site retries, another doesn't, with no
  rationale).
- **Comment hygiene**: comments that describe _what_ the code does (redundant
  with the code), stale comments left over from a prior version, TODOs without a
  tracked owner or condition.
- **Dead branches and leftover scaffolding**: `if (false)`, commented-out
  blocks, log lines added during development.
- **Documentation drift**: changed behavior that a docstring, README, or example
  still describes the old way.

## What this axis does NOT cover

Skip anything owned by another axis:

- Production bugs / API compatibility → `production.md`
- Security concerns → `security.md`
- Test coverage / quality → `tests.md`
- Performance hazards → `performance.md`
- Observability / logging / metrics → `observability.md`
- Dependency changes → `dependencies.md`
- Language/repo idioms and structure → `conventions.md` (and language sub-files)

If a finding fits cleanly under a specialist axis, leave it to that axis. Flag
it here only if the specialist would miss it because it is more about overall
intent than the axis's specific concern.

## Severity calibration

- **Critical**: the diff is incoherent or self-contradictory in a way that
  suggests a real misunderstanding — shipping it as-is will produce wrong
  behavior or block the next reviewer entirely.
- **Major**: meaningful reasoning gap or readability problem that will cost the
  next reader real time, or documentation drift that will mislead users.
- **Minor**: nits, stale comments, leftover debug lines, naming suggestions.

## Output

Return findings in the schema specified by the dispatcher (`severity`, `file`,
`line_start`, `line_end`, `title`, `detail`, `recommendation`). Empty list if
the diff reads cleanly.

Each `recommendation` must be concrete and actionable — name the file/line to
change, the comment to remove, the rename to apply, the docstring to update.
