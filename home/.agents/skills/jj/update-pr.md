# Update PR

Edits exist locally, a PR already covers this work. The new edits are already on
`@` or its ancestors. Apply them to the PR.

## 1. Decide the path

Inspect the PR's bookmark range (commits between its base bookmark and the
bookmark tip):

- **Mutable** (no commit in the range is in the immutable set) → §2.
- **Immutable** (any commit in the range is immutable, e.g. already pushed) →
  §3.

Never pass `--ignore-immutable`. If §2 errors with `Commit X is
immutable`,
switch to §3.

## 2. Mutable path

Fold the new edits into the PR however is cleanest (e.g. `jj squash` into the
relevant commit). Then advance the bookmark to the new tip and
`jj git push --bookmark <name>`. Force push is forbidden — on non-fast-forward
rejection, [recovery](recovery.md).

## 3. Follow-up path

Add the edits as one or more new commits on top of the PR's bookmark, described
per [naming](SKILL.md#naming-and-writing). Advance the bookmark to the new tip
and `jj git push --bookmark <name>`.
