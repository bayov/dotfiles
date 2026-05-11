---
axis: conventions
---

# Conventions review (cross-language)

Architectural and naming conventions that apply regardless of language. Stacks
with language-specific sub-axes (`conventions/python.md`, `conventions/go.md`) —
when the diff touches a covered language, both this axis and the relevant
sub-axis run as siblings.

## What to look for

### Module depth

- **Shallow modules** — small interface that hides little. A class/module that
  is mostly forwarding calls to its collaborators with no abstraction is not
  earning its keep. Apply the deletion test: if the module vanished and its
  callers absorbed its responsibilities, would total complexity rise? If no, the
  module is dead weight.
- **Pass-through wrappers** — a function/class whose body is one call to another
  function with the same shape. Merge into caller or callee.
- **Leaky interfaces** — a "boundary" type that exposes implementation details
  (e.g. internal data structures, framework types, ORM rows) to its consumers.

### Naming

- New module, function, class, and test names use the project's glossary terms
  verbatim. Drift from glossary → finding.
- Identifiers describe _what they are_ in domain terms, not _how they are
  implemented_ (`UserRepository` not `UserDBHelperImpl`).
- Consistent case style with the rest of the codebase. Mixed conventions in the
  same diff → finding.

### Repo fit

- Files placed in directories consistent with the existing layout. New top-level
  directories without precedent → finding.
- Import paths and visibility match the project's conventions.
- New utility functions duplicate utilities that already exist elsewhere in the
  repo. Use what's there.

### Comments and docstrings

- Comments explain _why_, not _what_. A comment that paraphrases the next line
  of code is noise.
- Docstrings describe behavior and contract, not implementation.
- No references to planning artifacts (task files, plan docs, PRD/design docs,
  AC text) anywhere in code or comments.

### Boundaries between layers

- Layer crossing matches the project's existing pattern (e.g. handlers don't
  reach into the DB directly if the codebase uses repositories).
- A new layer introduced for this diff alone, with no consistent precedent →
  finding.

## What this axis does NOT cover

- Language-specific idioms (use `conventions/python.md`, `conventions/go.md`,
  etc.).
- Production bug patterns → `production.md`.
- Performance → `performance.md`.

## Severity calibration

- **Critical**: a leaky interface or shallow module that locks the codebase into
  a bad shape going forward, or naming drift that breaks the domain-glossary
  contract.
- **Major**: pass-through wrapper, layering violation, duplicated utility,
  inconsistent layout that future contributors will follow.
- **Minor**: comment style, single naming nit, missing docstring on a non-public
  interface.

## Output

Per the dispatcher's schema. Each finding's `recommendation` names the specific
module/file to merge, rename, or rewrite, and what to replace it with.
