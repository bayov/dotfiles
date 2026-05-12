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
   `jj log`; skip it.
2. Rebase the whole non-merged subtree in one shot:
   `jj rebase -s <lowest-non-merged-commit> -d main@origin`. jj carries
   descendants along, so the chain order (and each bookmark's diffbase
   relationship) is preserved automatically.
3. Verify with `jj log` that bookmarks still chain bottom → top.
4. Push each non-merged bookmark: `jj git push --bookmark <name>`. Same
   immutable-fallback rule as §1.
5. Do not touch GitHub PR bases; they already point at the correct
   `<previous-bookmark>` and stay valid once the bookmarks move.
