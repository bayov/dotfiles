# Fix PR

Fetch a GitHub PR, enumerate everything blocking it (failing CI checks +
unresolved review comments), work through each, and report. Takes a PR number
(explicit, inferred from context, or detected — §2).

**Human** review-comment replies are posted as **pending draft comments** only,
never submitted — the user reviews and submits the draft in GitHub. **Codex**
comments are closed directly (no draft) and its re-review on each push is
awaited: see [Codex reviews](codex-reviews.md).

## 1. Resolve repo

Resolve owner/repo per [Repo owner and name](SKILL.md#repo-owner-and-name).
`$OWNER`/`$REPO` in the snippets are placeholders.

## 2. Determine PR number

Explicit from the user, or inferred from context. Otherwise detect from the repo
— walk `trunk()::@` for any commit carrying a local bookmark with an open PR:

```bash
PR=$(jj log -r 'bookmarks() & (trunk()::@)' \
  --no-graph -T 'local_bookmarks.map(|b| b.name()).join("\n") ++ "\n"' 2>/dev/null \
  | grep -v '^$' \
  | while read -r branch; do
      num=$(gh pr list --head "$branch" --state open --json number --jq '.[0].number' 2>/dev/null)
      [ -n "$num" ] && echo "$num" && break
    done)
```

Non-empty → use it. Still empty → stop and ask.

## 3. Validate checkout & position working copy

Resolve the head branch and confirm this clone has it:

```bash
BRANCH=$(gh pr view $PR --json headRefName --jq .headRefName)
jj bookmark list | grep -q "^${BRANCH}:"
```

Bookmark absent → **STOP**: "Wrong repo checkout: no local bookmark `$BRANCH`.
Check out a clone with this PR's branch fetched, then retry."

**Mergeability against base** — decides push strategy:

```bash
gh pr view $PR --json mergeable,mergeStateStatus
```

- `mergeable == "CONFLICTING"` (or `mergeStateStatus == "DIRTY"`) → the branch
  conflicts with its base and must be rebased onto it and resolved **before**
  any fix. Rebase the bookmark onto its base — `trunk()` for a normal PR, the
  parent bookmark for a stacked one — per [rebase.md](rebase.md) §1, then
  resolve per [rebase.md](rebase.md) §3. That rebase rewrites history and
  force-pushes (the sanctioned exception). Then position the working copy and
  continue.
- Otherwise → **no rewrite.** All fixes go in a new commit on top of the PR tip;
  the push stays fast-forward (the [update-pr.md](update-pr.md) rule). Never
  edit an already-pushed commit.

Position `@` for the fixes:

```bash
PARENT_IS_BRANCH=$(jj log -r "@- & $BRANCH" --no-graph -T '"yes"' 2>/dev/null)  # @ is a direct child of the bookmark?
CURR_DIFF=$(jj diff -r @ 2>/dev/null)                                          # current commit empty?
```

- `$PARENT_IS_BRANCH == "yes"` and `$CURR_DIFF` empty → positioned; continue.
- Otherwise → `jj new "$BRANCH"` for a fresh empty commit on the bookmark tip.

All code changes from here go into the working-copy commit.

## 4. Write `action_items.md`

```
mkdir -p ~/fix-pr/$PR && touch ~/fix-pr/$PR/action_items.md
```

**STOP — write it before anything else.**

```markdown
# PR #$PR action items

## CI

- [ ] <check name> — <one-line summary of the failure>

## Comments

- [ ] @<author> on <path>:<line> — <quoted first line of the comment>
      thread-node-id: <thread `id`, e.g. PRRT_...> first-comment-databaseId:
      <databaseId of first comment>
```

Update this checklist as you work.

## 5. Collect blockers

**Failing CI checks**:

```
gh pr checks $PR --json name,state,link,workflow \
  --jq '.[] | select(.state == "FAILURE" or .state == "CANCELLED" or .state == "TIMED_OUT")'
```

For each, fetch failure context (`gh run view <run-id> --log-failed` for
Actions, or follow the link) — enough to decide later if it's caused by this PR.

**Unresolved review comments**: REST doesn't expose "resolved" — use the GraphQL
`reviewThreads` connection. Include all authors (humans and bots). `--paginate`
for >100 threads:

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

Each unresolved thread is one action item. Record the thread `id` (node id,
`PRRT_...`) and the first comment's `databaseId` — both needed to reply /
resolve (and 👍 a codex comment). **Tag each thread codex vs human** by
first-comment `author.login` (codex closing differs, §6/§7). Also note codex's
current stance ([Codex reviews](codex-reviews.md)) — a 👀 means more threads may
still land.

## 6. Address items

One at a time. Tick each handled item in `action_items.md` with a one-line
resolution note.

**Scope gate (applies to every item):** if addressing it means a large or
non-trivial change — a broad refactor, a behavior/API change, a design decision,
or edits sprawling across many files — **stop and ask the user** before doing
it. Outline what it would take and the options. Only small, mechanical, locally
obvious fixes proceed without asking.

**Failing CI check:**

1. Decide if this PR caused it: read the log, then `gh pr diff $PR` for anything
   touching the failing area.
2. **Related → fix it.** Push the fix; rerun the check if needed.
3. **Unrelated → don't fix.** Tick it, note it's unrelated, and either point at
   an existing fix PR (`gh pr list --search '<keyword>'`) or recommend a
   separate PR. Surface prominently in the final report.

**Unresolved review comment:**

1. Read the comment and surrounding code.
2. Unambiguous fix → make the change.
3. Unsure → **stop and ask**, offering 2–3 concrete options with tradeoffs.
   Don't guess, don't silently pick.
4. **Closing depends on the author** (tagged in §5): **human** → never resolve
   yourself, reply as a pending draft (§7); **codex** → close per
   [Closing a codex comment](codex-reviews.md#closing-a-codex-comment) (after
   the push, § Push fixes, then close threads).

### Push fixes, then close threads

Comments **do not** wait for CI. Once the fix is pushed, close the threads; CI
is its own blocker, handled independently (below).

After all code fixes are in the working copy:

1. Run the pre-PR checks scoped to each service touched
   ([Before opening or updating a PR](SKILL.md#before-opening-or-updating-a-pr)).
   Stop and surface failures; don't push past a red pre-commit.
2. Describe the fix commit ([Commit description](SKILL.md#commit-description)),
   then push — fast-forward per [update-pr.md](update-pr.md) (or the
   already-done force-push if §3 rebased).
3. **Once pushed** (no CI wait): resolve fixed/dismissed **codex** threads (§6
   step 4) and post pending draft replies for **human** threads (§7). Replies
   reference the pushed commit.

CI runs on the push as its own action item (§6), addressed independently of the
review threads: related failure → fix and re-push; unrelated → surface in the
report. Never hold up replies for it.

### Codex re-review gate (ping-pong)

The push re-triggers codex. Before reporting, settle codex on the pushed HEAD
([Codex reviews](codex-reviews.md)): wait out 👀; 👍 → done. **New** findings →
add to `action_items.md` and loop back through §6 (fix → push → re-review),
ping-ponging until 👍 or the only thing left is a thread escalated to the user.
A standalone run does this loop itself; under [babysit-pr.md](babysit-pr.md) the
outer loop drives it.

## 7. Drafting pending review comments

Applies to **human** review threads only — codex threads are closed per §6 step
4, never drafted.

### Reply conventions

Follows the shared [Conventions](SKILL.md#conventions) (no LLMisms, no
em-dashes) and the [PR body](SKILL.md#pr-title--body) spirit. A reply states
what changed (and where) or answers the question. One read, done.

- No greetings, sign-offs, thanks, apologies, or praise ("Good catch", "You're
  right", "Great point").
- Don't restate the reviewer's comment back at them.
- Usually one sentence. Point at the fix: commit, file, or symbol.
- Pushing back: give the reason, not deference.
- Backticks for code/identifiers; link paths per the body conventions.

Examples:

- Good: `Fixed in {commit}: moved the nil check above the deref.`
- Good:
  `Kept as-is. {func} is only called post-validation, so the input can't
  be empty here.`
- Bad:
  `Great catch! 🙏 You're absolutely right, I've gone ahead and fixed this
  by adding a nil check. Let me know if there's anything else!`

### Posting mechanics

Exact — small deviations cause 422s or accidentally publish the review.

**Critical rules:**

- Never include `"event"` in the payload — its absence keeps the review
  `PENDING`.
- Always pipe JSON via `--input -`. `--field` for array values causes a 422.

### Setup

```bash
HEAD_SHA=$(gh api repos/$OWNER/$REPO/pulls/$PR --jq '.head.sha')
```

### New comment on a diff line

**Only to raise a brand-new comment of your own** — it opens a new standalone
thread. **Never to respond to an existing review thread** (that creates a
duplicate beside theirs); use _Reply_ below. Use `line` (actual file line) +
`side` (`"RIGHT"` added/unchanged, `"LEFT"` removed):

```bash
echo '{
  "commit_id": "'$HEAD_SHA'",
  "body": "",
  "comments": [{"path": "path/to/file.go", "line": 42, "side": "RIGHT", "body": "your comment"}]
}' | gh api repos/$OWNER/$REPO/pulls/$PR/reviews --method POST --input -
```

### Reply to an existing comment

Two-step REST+GraphQL flow to keep the reply PENDING.

**Step 1 — reuse the existing pending review, or create one, capture its
node_id:**

GitHub allows only **one pending review per user per PR**. Always reuse an
existing one — a second pending review orphans the drafts on the first (they
silently vanish). Holds across rounds and sessions.

```bash
REVIEW_NODE_ID=$(gh api repos/$OWNER/$REPO/pulls/$PR/reviews \
  --jq '.[] | select(.state=="PENDING") | .node_id' | head -n1)
if [ -z "$REVIEW_NODE_ID" ]; then
  REVIEW_NODE_ID=$(gh api repos/$OWNER/$REPO/pulls/$PR/reviews \
    --method POST -f body="" --jq .node_id)
fi
```

Omitting `event` keeps it PENDING.

**Step 2 — attach the reply to the thread via GraphQL:**

```bash
gh api graphql -f query='
  mutation($threadId: ID!, $reviewId: ID!, $body: String!) {
    addPullRequestReviewThreadReply(input: {
      pullRequestReviewThreadId: $threadId,
      pullRequestReviewId: $reviewId,
      body: $body
    }) {
      comment { id databaseId state }
    }
  }' \
  -f threadId="$THREAD_NODE_ID" \
  -f reviewId="$REVIEW_NODE_ID" \
  -f body="your reply"
```

`$THREAD_NODE_ID` is the thread's `id` (`PRRT_...`) from §5 — **not** the
comment's `databaseId`. Response `state` should be `PENDING`.

**If either call fails, stop and surface the error. Do NOT fall back to
`pulls/comments/{id}/replies` — that endpoint publishes immediately.**

### Verify pending state

```bash
gh api repos/$OWNER/$REPO/pulls/$PR/reviews \
  --jq '.[] | select(.state == "PENDING") | {id, state, submitted_at}'
# submitted_at: null confirms unpublished
```

### Do NOT submit

The user submits. Don't call `/reviews/$REVIEW_ID/events`; don't pass
`"event": "COMMENT"`/`"APPROVE"`/`"REQUEST_CHANGES"` anywhere.

## 8. Final report

When all items are ticked, print a summary. **Collapse items sharing a root
cause into one entry** (three CI failures + a reviewer comment from one missing
import = one entry listing all four sources).

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

State codex's final stance: `👍 (clean)`, `still reviewing (👀)`, or
`N comments
open` (noting any escalated to the user). End with: "Pending draft
review comments are queued. Review and submit in GitHub when ready."

## Checklist

- [ ] Repo owner/repo resolved.
- [ ] PR number determined.
- [ ] Checkout and working-copy position validated.
- [ ] Mergeability checked; rebased+resolved if CONFLICTING, else fast-forward.
- [ ] `~/fix-pr/$PR/action_items.md` created.
- [ ] All failing CI checks and unresolved threads enumerated into it.
- [ ] Each item addressed, resolution noted.
- [ ] Unrelated CI failures left untouched and surfaced in the report.
- [ ] Ambiguous comments paused for user input with concrete options.
- [ ] Pre-PR checks ran on each service touched; passing.
- [ ] Fixes pushed before threads closed; replies reference the pushed commit
      (no CI wait).
- [ ] Codex threads closed per §6 step 4; human replies posted as **pending**
      drafts.
- [ ] Codex re-review settled on HEAD (👍, looped to 👍, or escalated).
- [ ] Final report collapses items sharing a root cause and states codex's
      stance.
