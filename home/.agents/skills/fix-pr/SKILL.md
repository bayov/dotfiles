---
name: fix-pr
description: Address PR issues
argument-hint: <pr-number>
disable-model-invocation: true
---

# Goal

Fetch a GitHub PR, enumerate everything that's blocking it (failing CI checks +
unresolved review comments), work through each item, and report what was done.

Review-comment replies are posted as **pending draft comments** only — never
submitted. The user reviews and submits the draft in GitHub themselves.

# Process

## 1. Determine input PR number

The PR number is either specified explicitly by the user or inferred from the
conversation context.

If no PR number is available, detect it from the current JJ repo:

```bash
# Walk trunk()::@ for any commit carrying a local bookmark; for each, check for an open PR.
# This covers: @ is the PR branch, @ is empty/non-empty commits on top of it, or a chain.
PR=$(jj log -r 'bookmarks() & (trunk()::@)' \
  --no-graph -T 'local_bookmarks.map(|b| b.name()).join("\n") ++ "\n"' 2>/dev/null \
  | grep -v '^$' \
  | while read -r branch; do
      num=$(gh pr list --head "$branch" --state open --json number --jq '.[0].number' 2>/dev/null)
      [ -n "$num" ] && echo "$num" && break
    done)
```

If `$PR` is non-empty, use it. If still empty, stop and ask the user for the PR
number.

## 2. Validate repo checkout and position working copy

Resolve the PR's head branch name:

```bash
BRANCH=$(gh pr view $PR --json headRefName --jq .headRefName)
```

**Check that a local bookmark exists for this branch:**

```bash
jj bookmark list | grep -q "^${BRANCH}:"
```

If the bookmark is absent → **STOP.** Tell the user:

> Wrong repo checkout: no local bookmark `$BRANCH` found. Check out a repo that
> has this PR's branch fetched, then retry.

**Ensure the working copy sits on top of the PR commit:**

```bash
# Is @ a direct child of the PR bookmark?
PARENT_IS_BRANCH=$(jj log -r "@- & $BRANCH" --no-graph -T '"yes"' 2>/dev/null)
# Is the current commit empty (no changes)?
CURR_DIFF=$(jj diff -r @ 2>/dev/null)
```

- If `$PARENT_IS_BRANCH == "yes"` and `$CURR_DIFF` is empty → correctly
  positioned, continue.
- Otherwise → `jj new "$BRANCH"` to create a fresh empty commit on top of the PR
  bookmark.

All code changes from here on go into the current working copy commit.

## 3. Write `action_items.md`

```
mkdir -p ~/fix-pr/$PR
touch ~/fix-pr/$PR/action_items.md
```

**STOP — write `action_items.md` before doing anything else.**

```markdown
# PR #$PR action items

## CI

- [ ] <check name> — <one-line summary of the failure>

## Comments

- [ ] @<author> on <path>:<line> — <quoted first line of the comment> thread-id:
      <databaseId of first comment>
```

As you work through the next steps, update the `action_items.md` checklist.

## 4. Collect blockers

**Failing CI checks**:

```
gh pr checks $PR --json name,state,link,workflow \
  --jq '.[] | select(.state == "FAILURE" or .state == "CANCELLED" or .state == "TIMED_OUT")'
```

For each failing check, fetch the failure context
(`gh run view <run-id>
--log-failed` for Actions, or follow the link). Capture
enough to decide later whether it's related to this PR.

**Unresolved review comments**: GitHub's REST API does not expose the "resolved"
flag — use the GraphQL `reviewThreads` connection. Include all authors (humans
and bots alike). Add `--paginate` and `pageInfo` for PRs with >100 threads:

```
gh api graphql --paginate -f query='
  query($owner:String!, $repo:String!, $pr:Int!, $endCursor:String) {
    repository(owner:$owner, name:$repo) {
      pullRequest(number:$pr) {
        reviewThreads(first:100, after:$endCursor) {
          pageInfo { endCursor hasNextPage }
          nodes {
            id isResolved isOutdated
            comments(first:20) {
              nodes { id databaseId author{login} body path line }
            }
          }
        }
      }
    }
  }' -F owner=$OWNER -F repo=$REPO -F pr=$PR \
  --jq '.data.repository.pullRequest.reviewThreads.nodes
        | map(select(.isResolved == false))'
```

Each unresolved thread is one action item. Record the first comment's
`databaseId` — that's the ID to reply to.

## 5. Address items

Work through items one at a time. For each handled action-item, tick its
checkbox in `action_items.md` and append a one-line note describing the
resolution.

### Additional subagent instructions

**For a failing CI check:**

1. Decide if the failure is caused by this PR's changes. Read the failure log,
   then look at the diff (`gh pr diff $PR`) for anything that plausibly touches
   the failing area.
2. **If related → fix it.** Push the fix; rerun the check if needed.
3. **If unrelated → do not fix.** Tick the item, but in the resolution note say
   the failure is unrelated and either:
   - point the user at an existing fix PR you found (search open PRs touching
     the same area: `gh pr list --search '<keyword>'`), or
   - recommend a separate PR. Surface this prominently in the final report.

**For an unresolved review comment:**

1. Read the comment and the surrounding code.
2. If the fix is unambiguous → make the code change.
3. If you're unsure how to address it → **stop and ask the user**. Offer 2–3
   concrete options with their tradeoffs. Do not guess. Do not silently pick.
4. Never resolve the thread yourself. The user resolves threads after reviewing
   the draft.

### CI gate — before posting any reply

After all code fixes are in the working copy:

1. Run pre-commit for each service touched: `just <service>/ pre-commit`. Stop
   and surface failures; do not push past a red pre-commit.
2. Push commits, then wait for CI: `gh pr checks $PR --watch`.
3. **Only after all checks pass** → post pending draft replies (see _Drafting
   pending review comments_ below).

If CI fails on something this PR caused → fix it and recheck. If unrelated →
surface in the final report and proceed to replies.

## 6. Drafting pending review comments

These instructions are exact — small deviations cause 422s or accidentally
publish the review.

**Critical rules:**

- Never include `"event"` in the payload — its absence keeps the review
  `PENDING`.
- Always pipe JSON via `--input -`. Using `--field` for array values causes a
  422.

### Setup

```bash
HEAD_SHA=$(gh api repos/$OWNER/$REPO/pulls/$PR --jq '.head.sha')
```

### New comment on a diff line

Use `line` (actual file line number) + `side` (`"RIGHT"` for added/unchanged,
`"LEFT"` for removed). Avoids calculating unified diff position.

```bash
echo '{
  "commit_id": "'$HEAD_SHA'",
  "body": "",
  "comments": [{"path": "path/to/file.go", "line": 42, "side": "RIGHT", "body": "your comment"}]
}' | gh api repos/$OWNER/$REPO/pulls/$PR/reviews --method POST --input -
```

### Reply to an existing comment

Look up the comment's `databaseId` (already captured in step 1), then:

```bash
echo '{
  "commit_id": "'$HEAD_SHA'",
  "body": "",
  "comments": [{"path": "<copy path from original comment>", "in_reply_to": 123456789, "body": "your reply"}]
}' | gh api repos/$OWNER/$REPO/pulls/$PR/reviews --method POST --input -
```

`path` is required even for replies. Copy it verbatim from the first comment's
`path` field captured in step 4.

**If this call fails for any reason, stop and surface the error. Do NOT fall
back to `pulls/comments/{id}/replies` — that endpoint publishes immediately.**

### Verify pending state

```bash
gh api repos/$OWNER/$REPO/pulls/$PR/reviews \
  --jq '.[] | select(.state == "PENDING") | {id, state, submitted_at}'
# submitted_at: null confirms unpublished
```

### Do NOT submit

Submitting the review is the user's job. Do not call
`/reviews/$REVIEW_ID/events`. Do not pass `"event": "COMMENT"` / `"APPROVE"` /
`"REQUEST_CHANGES"` anywhere.

## 7. Final report

When all items are ticked, print a summary to the user.

**Collapse items that share a root cause into one entry.** If three CI failures
and a reviewer comment all stem from the same missing import, that's one entry
listing all four sources — not four.

Format:

```
PR #$PR — N items addressed (M root causes)

[fixed] <root cause / what changed>
  - CI: <check>
  - comment: @<author> on <path>:<line>

[deferred — unrelated CI] <check>
  Cause: <one line>. Suggest: <separate PR | existing fix at #1234>.

[pending user input] @<author> on <path>:<line>
  <question you asked the user>
```

End with: "Pending draft review comments are queued. Review and submit in GitHub
when ready."

# Checklist

- [ ] All failing CI checks enumerated; logs read.
- [ ] All unresolved review threads enumerated via GraphQL (all authors,
      including bots).
- [ ] Each action-item ticked with a one-line resolution note as it's handled.
- [ ] Unrelated CI failures left untouched and surfaced in the final report.
- [ ] Ambiguous comments paused for user input with concrete options offered.
- [ ] Pre-commit ran on each service touched (`just <service>/ pre-commit`);
      passing.
- [ ] CI passing (`gh pr checks $PR`) confirmed before any reply is posted.
- [ ] All review-comment replies posted as **pending** drafts (no `event`
      field); pending state verified.
- [ ] Final report collapses items sharing a root cause.
