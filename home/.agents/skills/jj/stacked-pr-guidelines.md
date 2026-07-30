# Stacked PR Guidelines

A PR stack is a chain of PRs where PR N's GitHub base is PR N-1's bookmark.

`cato-aisecurity/haka` has **GitHub Stacked PRs** enabled: GitHub tracks the
chain as a first-class stack, renders a stack map in every PR header, retargets
bases as PRs land, and merges the chain. So on haka:

- **Do not** hand-write a `### PR Stack` section in PR bodies. The stack map
  replaces it.
- Register the chain with GitHub (`gh stack link`, §2) after creating the PRs.

Any other repo has no native stacks: fall back to the hand-written section (§6).
Check enablement (404 = not enabled):

```bash
gh api "repos/$OWNER/$REPO/stacks" --silent 2>/dev/null \
  && echo "native stacks" || echo "no native stacks"
```

The CLI extension: `gh stack --version`, install with
`gh extension install github/gh-stack`.

## 1. jj owns the bookmarks

`gh stack` has its own local branch-tracking mode that fights jj. Only these are
in play: `gh stack link`, `gh stack merge`, and the REST endpoints under
`repos/$OWNER/$REPO/stacks`. Never run `gh stack init` / `add` / `submit` /
`push` / `sync` / `rebase` / `checkout` / `modify` / `switch` in a jj repo -
bookmarks, pushes, and rebases stay with jj ([SKILL.md](SKILL.md#pushing),
[rebase.md](rebase.md), [sync.md](sync.md)).

## 2. Create or extend a stack

1. Push each bookmark with jj and create each PR against the bookmark below it
   (`gh pr create --base {parent-bookmark}`, [new-pr.md](new-pr.md) §4), so the
   bases already chain bottom to top. GitHub rejects a link whose bases don't
   chain (each PR's base ref must equal the head ref of the PR below).
2. Link the PRs, **bottom to top**, by PR number:

   ```bash
   gh stack link 15052 15071 13571 --base "$TRUNK"
   ```

   Pass PR **numbers**, not bookmark names: branch arguments make `gh stack`
   push branches behind jj's back. `--base` is the trunk the whole stack targets
   (defaults to `main`); resolve it with
   `TRUNK=$(gh repo view --json defaultBranchRef -q .defaultBranchRef.name)`.

   Creates the stack if there is none, extends it if some of those PRs are
   already stacked (existing PRs are never removed). Re-running with the same
   PRs is a no-op (`Stack with N PRs is already up to date`). Add `--open` only
   if you also want draft PRs flipped to ready for review.

3. Verify:

   ```bash
   gh api "repos/$OWNER/$REPO/stacks?pull_request=15071" \
     --jq '.[0] | {stack: .number, base: .base.ref, prs: [.pull_requests[].number]}'
   ```

   Each PR also carries the stack inline (`null` when not stacked):
   `gh api repos/$OWNER/$REPO/pulls/15071 --jq .stack` →
   `{id, number, size, position, base}`, `position` 1-based from the bottom.

Constraints: 2-100 PRs per stack, all branches in the same repo (no cross-fork),
base may be any branch (not just trunk).

### Appending later

A new PR on top of the current tip: create it with
`gh pr create --base {current-tip-bookmark}`, then re-run `gh stack link` with
the full chain (or just append: `gh stack link {current-tip-pr} {new-pr}`).

REST equivalents, if the CLI misbehaves:

```bash
echo '{"pull_requests": [15052, 15071, 13571]}' \
  | gh api --method POST repos/$OWNER/$REPO/stacks --input -          # create
echo '{"pull_requests": [14573]}' \
  | gh api --method POST repos/$OWNER/$REPO/stacks/{stack}/add --input -   # append to top
```

### Reordering or inserting mid-stack

Not supported: `link`/`add` only append to the top. Dissolve and re-link in the
new order (merged, merging, or queued PRs cannot be unstacked and stay put):

```bash
gh api --method POST repos/$OWNER/$REPO/stacks/{stack}/unstack
gh stack link {pr} {pr} {pr} --base "$TRUNK"
```

## 3. PR descriptions

No `### PR Stack` section, no `---` rule for it. Body rules are unchanged
([Conventions](SKILL.md#pr-title--body)); inline references to sibling PRs
("uses API introduced in #15053") are still fine.

If a PR in the stack still carries a hand-written `### PR Stack` section from
before the feature, strip it (and its `---`) the next time you touch that body.

## 4. Keeping the stack mergeable

Linear history is strictly required: a PR merges only when every PR below it is
approved with passing checks **and** the stack is fully rebased. Branch
protection is evaluated as if each PR targeted the stack base, and Actions run
on every PR in the stack.

Rebase with jj, not with `gh stack rebase` or the UI's "Rebase Stack" button
(both rewrite the remote branches under jj). [rebase.md](rebase.md) §2 /
[sync.md](sync.md) preserve the chain, and the stack link survives the
force-push: it's a property of the PRs, not of the branch SHAs.

## 5. Merging

`gh pr merge` does **not** work on a stacked PR: stack merges go through an
async merge API. Merging PR N lands N plus every unmerged PR below it, bottom-up
and atomically. So merge the bottom PR to land one, the top PR to land the whole
stack. Non-contiguous merges are impossible, and a closed PR mid-stack blocks
everything above it.

```bash
gh stack merge {pr}    # {pr} + all unmerged PRs below it
```

Non-interactive (agents, explicit merge method - haka squash-merges):

```bash
echo '{"merge_method": "squash", "merge_action": "default"}' \
  | gh api --method PUT repos/$OWNER/$REPO/pulls/{pr}/merge-async --input -
# 202 + {"status": "pending", "uuid": ...} -> poll until status != pending
gh api repos/$OWNER/$REPO/pulls/{pr}/merge-async/{uuid}
```

Terminal statuses: `merged`, `enqueued` (went to the merge queue), `failed`.
Submit only checks that the PR is open and not a draft - branch protection and
rules are evaluated when the merge actually runs, so a rule failure surfaces in
the poll result, not at submit.

**After a partial merge**, GitHub rebases and retargets the remaining PRs
itself: the new bottom PR points at the trunk with no `gh pr edit --base` from
you. Skip the retarget steps ([rebase.md](rebase.md) §2.5, [sync.md](sync.md)
§4) on a native stack. The server rebase moves the remote bookmarks, so
`jj git fetch` afterwards and adopt the remote heads
(`jj bookmark set {bookmark} -r {bookmark}@origin`) rather than re-pushing local
ones; rebase any unpushed local commits on top.

Merged PRs stay in the stack map (GitHub marks them merged), which is useful
context for reviewers. Nothing to prune.

## 6. Fallback: hand-written `### PR Stack` (repos without native stacks)

Each PR in the stack gets a `### PR Stack` section in its description:

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

Separate the body from the section with a `---` horizontal rule that has a
**blank line both above and below it** (emit `{body}\n\n---\n\n### PR Stack`).

Top entry is the tip (furthest from trunk). Each `↑` means that PR is stacked on
the entry below it. The last line is the repo's trunk, not a PR: mark it with ○
and use the actual trunk name (`main`/`master`/etc.), without the `↑`.

The current PR is wrapped in backticks to prevent GitHub from auto-linking it
(redundant self-link).

Whenever a PR is added to the stack, or the stack's ancestry changes, update
every PR's `### PR Stack` section. Do **not** remove merged PRs from it, even
after a rebase: GitHub shows them as merged, and their presence gives reviewers
context.

Merging is one-by-one from the bottom with plain `gh pr merge`; the new bottom
is then rebased onto the new `trunk()` and retargeted with
`gh pr edit --base "$TRUNK"`.

> Note: Omit a PR reference from `### PR Stack` sections if the user requests
> it. The user sometimes includes an unrelated PR as part of the stack for
> convenience. This will only confuse reviewers.
