# Stacked PR

A stack is a chain of PRs where PR N's GitHub base is PR N-1's bookmark. Each PR
has its own bookmark and its own `gh pr` — the stack lives in how the bases are
wired, not in commit count.

## 1. Add a new PR on top

1. The edits sit on top of the current top bookmark (place via
   `jj new
   <top-bookmark>` if needed; if edits already build on it, fine).
2. [Create the Linear issue](new-pr.md#2-create-the-linear-issue).
3. [Publish](new-pr.md#3-publish): describe, bookmark, push.
4. `gh pr create --base <previous-bookmark>`.

## 2. Update a PR inside the stack

Apply [update-pr](update-pr.md) to the target PR. After pushing, if PRs above it
now diverge from the updated tip, stop and ask the user before rebasing the
upper PRs — never auto-rebase across multiple stacked PRs.

## 3. After the bottom PR merges

`jj git fetch`, then `jj rebase -d main@origin` for the remaining stack. Stop
and ask the user on any conflict.
