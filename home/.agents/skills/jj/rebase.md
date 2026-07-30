# Rebase

Pull latest trunk and rebase a PR (or non-merged stack) onto it; rewrites
history, so the push force-updates ([Pushing](SKILL.md#pushing)). Resolve
conflicts by default (§3). `trunk()` resolves the trunk bookmark.

## 1. Single PR

1. `jj git fetch`.
2. `jj rebase -b {pr-bookmark} -d 'trunk()'`.
3. `jj git push --bookmark {pr-bookmark}`. If jj aborts (remote moved since
   fetch / bookmark conflict), stop and ask; don't force past it.

## 2. Stack

Rebase non-merged PRs only, preserving the diffbase chain (bottom on trunk, each
next on the one below).

1. `jj git fetch`; set `$TRUNK`:
   `gh repo view --json defaultBranchRef -q .defaultBranchRef.name`. Merged PRs
   show as absorbed in `jj log` — skip them. If the merged PR was the stack
   bottom, the next bookmark up becomes the new bottom (retarget in §2.5).
2. Rebase the whole non-merged subtree at once:
   `jj rebase -s {lowest-non-merged-commit} -d 'trunk()'`. jj carries
   descendants along, preserving chain order.
3. Verify with `jj log` that bookmarks still chain bottom → top.
4. Push each non-merged bookmark: `jj git push --bookmark {name}` (same abort
   rule as §1.3).
5. Retarget the new bottom only if its previous base was a merged PR:
   1. `gh pr view {new-bottom} --json baseRefName -q .baseRefName`.
   2. If not `$TRUNK`: `gh pr edit {new-bottom} --base "$TRUNK"`.

   Other bookmarks keep their bases (the chain moved together).

   On a **native GitHub stack** (haka) GitHub retargeted the new bottom at trunk
   itself when the PR below it merged, so skip this step. Its server-side rebase
   also moved the remote bookmarks: after §2.1's fetch, adopt the remote heads
   instead of re-pushing local ones
   ([Stacked PR Guidelines](stacked-pr-guidelines.md) §5).

## 3. Conflicts

jj records conflicts in-commit and doesn't halt. Resolve bottom-up:
`jj log -r 'conflicts()'`, `jj edit {commit}`, fix the markers, re-check. Ask
only on genuinely ambiguous (semantic) conflicts.
