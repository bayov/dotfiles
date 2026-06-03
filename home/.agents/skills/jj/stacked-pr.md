# Stacked PR

A stack is a chain of PRs where PR N's GitHub base is PR N-1's bookmark. Each PR
has its own bookmark and its own `gh pr` — the stack lives in how the bases are
wired, not in commit count.

## 1. Add a new PR on top

1. The edits sit on top of the current top bookmark (place via
   `jj new
   <top-bookmark>` if needed; if edits already build on it, fine).
2. [Resolve the Linear issue](new-pr.md#2-resolve-the-linear-issue).
3. [Publish](new-pr.md#3-publish): describe, bookmark, push.
4. `gh pr create --base <previous-bookmark>`. `<previous-bookmark>` is the
   bookmark of the PR directly below this one in the stack — not trunk and not
   the bottom of the stack. Every bookmark has a PR. To tell if that PR is
   already merged (and the base should be `$TRUNK` instead), check with jj: if
   the bookmark is an ancestor of `trunk()` (`jj log -r '<bookmark>..trunk()'`
   is empty), the PR merged — use `$TRUNK`. `$TRUNK` = repo default branch
   ([trunk](SKILL.md#trunk)). The diffbase must form the chain: 1st PR → trunk,
   2nd PR → 1st PR's bookmark, 3rd PR → 2nd PR's bookmark, etc.; skip any
   bookmark whose commits are already in `trunk()`.

## 2. Update a PR inside the stack

Apply [update-pr](update-pr.md) to the target PR. After pushing, if PRs above it
now diverge from the updated tip, stop and ask the user before rebasing the
upper PRs — never auto-rebase across multiple stacked PRs.

## 3. After the bottom PR merges

`jj git fetch`, then `jj rebase -d 'trunk()'` for the remaining stack. Stop and
ask the user on any conflict.
