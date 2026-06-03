# Sync

Fetch latest trunk and rebase every local PR bookmark onto it. Scope is
PR-bearing bookmarks; unpushed local commits without a bookmark are out of
scope. Force push is forbidden; stop and ask on any conflict.

`trunk()` auto-resolves the trunk bookmark (`main@origin`, `master@origin`, …);
`$TRUNK` = repo default branch ([trunk](SKILL.md#trunk)). In a shared repo with
many bookmarks from other people, scope enumeration to **local** bookmarks
(`bookmarks() & ~remote_bookmarks()`) — untracked remote branches are not yours.

## 1. Fetch

`jj git fetch`. Any merged PR has its commits absorbed into `trunk()` and drops
out of the "ahead of trunk" set automatically.

## 2. Enumerate

All local PR bookmarks still ahead of trunk:
`jj log -r 'bookmarks() & ~remote_bookmarks() & (trunk()..)' --no-graph -T 'bookmarks ++ "\n"'`.

Stack roots (lowest bookmark of each independent stack):
`jj log -r 'roots(bookmarks() & ~remote_bookmarks() & (trunk()..))' --no-graph -T 'bookmarks ++ "\n"'`.
A single-PR stack appears as a root with no descendants in the set.

## 3. Rebase

For each stack root: `jj rebase -s <root-commit> -d 'trunk()'`. jj carries
descendants along, so intra-stack chain order (and each non-root bookmark's
diffbase relationship to the one below it) is preserved.

## 4. Retarget bases of new stack roots

Each root from §2 now sits directly on `trunk()`. If its PR base on GitHub still
points at a previously-merged bookmark, retarget:

1. `gh pr view <root-bookmark> --json baseRefName -q .baseRefName`.
2. If the result is not `$TRUNK`, `gh pr edit <root-bookmark> --base "$TRUNK"`.

GitHub auto-retargets when the merged bookmark's remote branch is deleted on
merge — this step catches repos where that setting is off. Non-root bookmarks in
a stack keep their existing bases; they already point at the bookmark directly
below and stay valid because the chain moved together.

## 5. Push

Push the bookmarks from §2 in batches of at most 2 per `jj git push` invocation,
passing each via its own `--bookmark <name>` flag. On non-fast-forward rejection
the bookmark's commits became immutable — switch to
[update-pr §3 follow-up](update-pr.md#3-follow-up-path) for that bookmark
instead.

## 6. Update the working copy

Rebase the current working copy onto fresh trunk only when `@` is left stranded
on a stale base: `jj rebase -s @ -d 'trunk()'`. Apply even when `@` is empty or
has no bookmark — its parent should be the latest trunk.

**Skip** when `@` already sits on current work:

- `@` is already a descendant of `trunk()` (nothing to do), or
- `@` is a descendant of a commit rebased in §3 (a PR bookmark / stack). It rode
  along with that rebase and belongs on top of its stack, not on trunk —
  re-pointing it at trunk would tear it off its base.

Only rebase onto `trunk()` when `@`'s ancestry traces to the _old_ trunk and not
through any in-scope bookmark.

## 7. Conflicts

Stop and ask on any rebase conflict; do not resolve.
