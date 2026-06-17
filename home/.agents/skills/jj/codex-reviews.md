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

**Codex-clean for HEAD** (`$HEAD_SHA`): 👍 on the issue, OR its latest review's
`commit_id == $HEAD_SHA` with no unresolved threads. Still 👀, or latest review
on an **older** commit → not finished; wait and re-poll. Read its stance:

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
  concise** with the reason, then resolve. One line, no hedging or restating.
  E.g. `Intended: $func runs only post-validation, input can't be empty.` /
  `Incorrect: $x is already bounded by the caller.`
- **Needs a human decision** (not a wrong-vs-right call you can make) or trips
  the scope gate → do **not** resolve; stop and ask the user.

Mechanics (`$COMMENT_DB_ID` = first-comment `databaseId`, `$THREAD_NODE_ID` =
thread `id`, `PRRT_...`). 👍 (fixed case) or a published reply (dismiss case —
unlike human replies, codex dismissals publish immediately), then resolve:

```bash
# fixed: react 👍
gh api repos/$OWNER/$REPO/pulls/comments/$COMMENT_DB_ID/reactions -f content='+1'
# dismiss: publish the one-line reason
gh api repos/$OWNER/$REPO/pulls/$PR/comments/$COMMENT_DB_ID/replies -f body='<reason>'
# both: resolve the thread
gh api graphql -f query='
  mutation($t:ID!){ resolveReviewThread(input:{threadId:$t}){ thread{ isResolved } } }' \
  -f t="$THREAD_NODE_ID"
```
