# Codex reviews

`chatgpt-codex-connector[bot]` auto-reviews on PR open, draft-ready, and **every
push** (no manual trigger). Two verdict signals per commit:

- **PR-issue reaction** (`gh api repos/$OWNER/$REPO/issues/$PR/reactions`,
  `.user.login=="chatgpt-codex-connector[bot]"`): `eyes` 👀 = reviewing, wait;
  `+1` 👍 = no (further) findings, happy with the current commit (no comment
  accompanies a 👍 — the reaction is the whole signal).
- **A review** (`/pulls/$PR/reviews`, same author, state `COMMENTED`, body opens
  `### 💡 Codex Review`, names a **Reviewed commit:** `<sha>`). Findings come as
  inline threads, each tagged with a priority badge (`P1`/`P2`/`P3`).

**Codex-clean for HEAD** (`$HEAD_SHA`) needs **both**:

1. **Not mid-review** — reaction is 👍, or a review with
   `commit_id == $HEAD_SHA` exists. Still 👀, or the latest review sits on an
   **older** commit → not finished; wait and re-poll. Never declare clean off a
   stale reaction.
2. **No unresolved bot threads** — in the always-fetched unresolved-thread list
   ([fix-pr.md](fix-pr.md) §5), no unresolved thread is authored by codex/a bot.

The reaction is the _mid-review_ gate only, never the cleanliness verdict. A 👍
means codex found nothing (no threads land). A **`COMMENTED` review is a
findings signal, not a done signal**: its findings are inline threads that
appear in the §5 list like any other author's. `commit_id == $HEAD_SHA` alone is
**not** clean. There is no separate codex-only thread query — thread state
always comes from the one unfiltered §5 fetch, routed by author. Read the
reaction gate:

```bash
HEAD_SHA=$(gh api repos/$OWNER/$REPO/pulls/$PR --jq .head.sha)
gh api repos/$OWNER/$REPO/issues/$PR/reactions \
  --jq '.[] | select(.user.login=="chatgpt-codex-connector[bot]") | .content'
gh api repos/$OWNER/$REPO/pulls/$PR/reviews \
  --jq '[.[] | select(.user.login=="chatgpt-codex-connector[bot]")] | last
        | {state, commit_id, body: .body[0:40]}'
```

## Closing a codex comment

**Codex can be wrong** — judge each comment on its merits; don't fix something
just because it flagged it. Pick one disposition; never just react+resolve a
human thread this way.

- **Cleanly fixed** (mechanical, scope gate not tripped) → 👍 the comment and
  resolve. No draft reply.
- **Wrong or not worth fixing** (known/intended gap) → reply **very short and
  concise** with the reason, then resolve. One line, no hedging or restating,
  and prefixed with `**[Written by Claude]** ` like every other AI reply
  ([Reply conventions](fix-pr.md#reply-conventions)). E.g.
  `**[Written by Claude]** Intended: $func runs only post-validation, input
  can't be empty.` / `**[Written by Claude]** Incorrect: $x is already bounded
  by the caller.`
- **Needs a human decision** (not a wrong-vs-right call you can make) or trips
  the scope gate → do **not** resolve; stop and ask the user.

Mechanics (`$COMMENT_DB_ID` = first-comment `databaseId`, `$THREAD_NODE_ID` =
thread `id`, `PRRT_...`). 👍 (fixed case) or a published reply (dismiss case —
unlike human replies, codex dismissals publish immediately), then resolve:

```bash
# fixed: react 👍
gh api repos/$OWNER/$REPO/pulls/comments/$COMMENT_DB_ID/reactions -f content='+1'
# dismiss: publish the one-line reason
gh api repos/$OWNER/$REPO/pulls/$PR/comments/$COMMENT_DB_ID/replies \
  -f body='**[Written by Claude]** <reason>'
# both: resolve the thread
gh api graphql -f query='
  mutation($t:ID!){ resolveReviewThread(input:{threadId:$t}){ thread{ isResolved } } }' \
  -f t="$THREAD_NODE_ID"
```
