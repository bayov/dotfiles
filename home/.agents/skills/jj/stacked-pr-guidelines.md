# Stacked PR Guidelines

A PR stack is a chain of PRs where PR N's GitHub base is PR N-1's bookmark.

## Stacked PR descriptions

Each PR in a PR stack must contain a `### PR Stack` section in its description.
Example:

```md
<!-- Regular PR description here -->

---

### PR Stack

- ↑ #15052
- ↑ #15071
- ↑ #13571
- ↑ **Current** (`#14573`)
- ↑ #14566
- ↑ #14569
- ○ `main`
```

Top entry is the tip (furthest from trunk). Each `↑` means that PR is stacked on
the entry below it. The last line is the repo's trunk, not a PR: mark it with ○
and use the actual trunk name (`main`/`master`/etc.), without the `↑`.

The current PR is wrapped in backticks to prevent GitHub from auto-linking it
(redundant self-link).

Whenever a new PR is added to the stack, or the stack is modified in a way that
changes the ancestry, each PR's `### PR Stack` section must be updated to
reflect the new stack order.

> Note: Omit a PR reference from `### PR Stack` sections if the user requests
> it. The user sometimes includes an unrelated PR as part of the stack for
> convenience. This will only confuse reviewers.

## Merging

PRs in a stack are merged one-by-one, starting from the bottom. The new stack
bottom should then be rebased onto the new `trunk()` after a merge.

**IMPORTANT**: Do **not** remove merged PRs from the `### PR Stack` section of
the remaining PRs, even after rebase. Merged PRs will automatically show up as
such in GitHub UI, and their presence provides context for reviewers.
