# Update PR

Edits exist locally and a PR already covers this work.

**Rule:** the new edits go in commit(s) **on top of** the PR's bookmark tip;
never modify an already-pushed commit. Leaving pushed commits untouched keeps
the push fast-forward — no history rewrite, no force-push — and the appended
commits collapse on merge anyway since PRs are squash-merged. Rewriting pushed
history (and the force-push it needs) happens only during rebases
([rebase.md](rebase.md), [sync.md](sync.md)).

## Steps

1. Put the new edits in their own commit(s) on top of the bookmark tip, each
   described per [Commit description](SKILL.md#commit-description).

   If the edits instead landed **inside an already-pushed commit** (e.g. `@` is
   that commit and got dirtied), split them out onto a fresh commit built on the
   pristine remote commit, so the pushed commit is left exactly as reviewed and
   the push still fast-forwards:

   ```
   jj new {bookmark}@origin             # empty commit on the pristine pushed tip
   jj restore --from {bookmark} --into @  # @ := dirtied tree; diff vs parent = just the new edits
   jj describe -m "..."                 # per Commit description
   ```

2. Advance the bookmark to the new tip: `jj bookmark set {bookmark} -r @`.
3. `jj git push --bookmark {bookmark}`. On non-fast-forward rejection — the
   remote bookmark moved, or a pushed commit was rewritten — stop and ask the
   user; never force-push.

> Squash or otherwise rewrite a PR's already-pushed commits only when the user
> explicitly asks. That rewrite forces the push, same as a rebase.
