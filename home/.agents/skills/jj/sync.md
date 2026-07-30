# Sync

Fetch latest trunk and rebase every local commit ahead of it onto new trunk — PR
bookmarks and unpushed bookmarkless work. Bookmarkless commits rebase but aren't
pushed/retargeted (no PR). Rebasing rewrites pushed commits, so the push
force-updates ([Pushing](SKILL.md#pushing)). Resolve conflicts by default (§7).

`trunk()` resolves the trunk bookmark.

## 1. Fetch

`jj git fetch`. Merged PRs absorb into `trunk()` and drop out of "ahead of
trunk" automatically. Then set `$TRUNK` to the default branch name:
`gh repo view --json defaultBranchRef -q .defaultBranchRef.name`.

## 2. Enumerate

Rebase roots (§3) — every local chain ahead of trunk; `mutable()` drops others'
branches:
`jj log -r 'roots((trunk()..) & mutable())' --no-graph -T 'commit_id.short() ++ "\n"'`

PR bookmarks — pushed (§5) and retargeted (§4):

- Push list:
  `jj log -r 'bookmarks() & ~remote_bookmarks() & (trunk()..)' --no-graph -T 'bookmarks ++ "\n"'`
- Stack roots (for retarget):
  `jj log -r 'roots(bookmarks() & ~remote_bookmarks() & (trunk()..))' --no-graph -T 'bookmarks ++ "\n"'`

## 3. Rebase

For each rebase root: `jj rebase -s {root-commit} -d 'trunk()'`. jj carries
descendants along, preserving chain order.

## 4. Retarget bases

Each PR stack root now sits on `trunk()`. If its base still points at a merged
bookmark:

1. `gh pr view {root-bookmark} --json baseRefName -q .baseRefName`
2. If not `$TRUNK`: `gh pr edit {root-bookmark} --base "$TRUNK"`

Non-root bookmarks keep their bases (the chain moved together).

On a **native GitHub stack** (haka) GitHub retargets the new bottom itself as
PRs land, so there is nothing to retarget here. Its server-side rebase moves the
remote bookmarks, so after §1's fetch adopt the remote heads rather than
re-pushing local ones ([Stacked PR Guidelines](stacked-pr-guidelines.md) §5).

## 5. Push

Push §2's bookmarks, ≤2 per `jj git push`, each via `--bookmark {name}`.
Rewrites force-update with lease. If jj aborts (remote moved since fetch /
bookmark conflict), stop and ask; don't force past it.

## 6. Working copy

§3 already carried `@` along with its root/stack — usually nothing to do. Rebase
manually only if `@` is still stranded on old trunk:
`jj rebase -s @ -d 'trunk()'`. Don't re-point a stacked `@` at trunk — it'd tear
off its base.

## 7. Conflicts

jj doesn't halt — the rebase completes with conflicts recorded in-commit. List:
`jj log -r 'conflicts()' --no-graph -T 'commit_id.short() ++ "\n"'`. Resolve
bottom-up (jj propagates fixes to descendants): `jj edit {commit}`, fix the
markers, re-check `conflicts()`. Ask only on genuinely ambiguous (semantic)
conflicts.
