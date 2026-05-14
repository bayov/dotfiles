# Rebase

Pull latest `main` and rebase the PR (or non-merged stack) onto it. Force push
is forbidden; stop and ask on any conflict.

## 1. Single PR

1. `jj git fetch`.
2. `jj rebase -b <pr-bookmark> -d main@origin`.
3. Push: `jj git push --bookmark <pr-bookmark>`. On non-fast-forward rejection,
   the bookmark's commits became immutable — switch to
   [update-pr §3 follow-up](update-pr.md#3-follow-up-path) instead.

## 2. Stack

Rebase non-merged PRs only, preserving the diffbase chain (bottom on `main`,
each next on the one below).

1. `jj git fetch`. Any PR already merged into `main` will show as absorbed in
   `jj log`; skip it. If the merged PR was the bottom of the stack, the next
   bookmark up becomes the new bottom — see §2.5 for its base retarget.
2. Rebase the whole non-merged subtree in one shot:
   `jj rebase -s <lowest-non-merged-commit> -d main@origin`. jj carries
   descendants along, so the chain order (and each bookmark's diffbase
   relationship) is preserved automatically.
3. Verify with `jj log` that bookmarks still chain bottom → top.
4. Push each non-merged bookmark: `jj git push --bookmark <name>`. Same
   immutable-fallback rule as §1.
5. PR bases stay correct for every position **except** when the previous
   bookmark was a merged PR. For the new bottom (the lowest non-merged bookmark,
   now parented on `main@origin`):
   1. `gh pr view <new-bottom> --json baseRefName -q .baseRefName`.
   2. If the result is not `main`, `gh pr edit <new-bottom> --base main`.

   GitHub auto-retargets when the merged bookmark's remote branch is deleted on
   merge; this step catches repos where that setting is off. All other
   bookmarks' bases keep pointing at the bookmark directly below and stay valid
   because the chain moved together.
