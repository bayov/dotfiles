---
name: bayov-code-review
description: Review a GitHub PR the way I do. Finder subagents raise findings, one auditor subagent per finding tries to refute it, and the survivors become a draft (pending) review written in my voice. Also calibrates itself from the review I actually submitted.
argument-hint: <pr-url | owner/repo#number | number> | calibrate <pr>
disable-model-invocation: true
model: opus
effort: high
---

# Code review in my voice

Review someone else's PR and leave a draft review I can edit and submit. Many
findings go in, few comments come out. Every comment must be one I would write
myself, in words I would use.

## Input

- A PR URL, `owner/repo#number`, or a bare number (resolve it against the
  current repo's GitHub remote). Run the review below. Set `OWNER`, `REPO`,
  `PR`, `HEAD_SHA`, `BASE`.
- `calibrate <pr>`: skip the review and run "Calibrate" at the end of this file.

## Files

- `voice.md`: how I phrase comments, with real before/after pairs.
- `auditor.md`: the brief for one auditor subagent.
- `domains/*.md`: the bar for each kind of code, plus a calibration log. Each
  file starts with the paths it applies to.

## 1. Gather context

Work in a scratch dir (`$SCRATCH`). Save everything there so subagents read
files and not tool output.

- `gh pr view` with title, body, author, base, head SHA, files and commits.
- `gh pr diff` into `$SCRATCH/diff.patch`.
- All existing review threads, from bots and humans, with replies and resolved
  state (GraphQL `reviewThreads`, as in `../jj/fix-pr.md` §5), into
  `$SCRATCH/threads.json`. Note what was already raised, what the author fixed,
  and what they declined and why.
- A local checkout of the PR head, so agents can grep without burning the GitHub
  API rate limit (one auditor ran out of it once):

  ```bash
  git clone --filter=blob:none --no-checkout --depth 1 \
    https://github.com/$OWNER/$REPO "$SCRATCH/repo"
  git -C "$SCRATCH/repo" fetch --depth 1 --filter=blob:none origin \
    "pull/$PR/head:pr-head" "$BASE:base"
  git -C "$SCRATCH/repo" sparse-checkout set <package roots of changed files>
  git -C "$SCRATCH/repo" checkout pr-head
  ```

  `git show base:<path>` gives the base version. GitHub code search only indexes
  the default branch, so it can't see the PR.

- When the PR calls a service that lives in a repo I have locally (for example
  the haka BFF), note its path. Auditors can check the backend contract there.

Read the diff yourself before fanning out. You need it to brief the agents and
to judge their output.

Then pick the domain files. Read every `domains/*.md` whose paths match a
changed file. A finding is judged by the domain of the file it sits in. When no
domain file matches, use `domains/backend.md` and say so in the report.

## 2. Find

Reuse the axis prompts of the `review-code` skill (`../review-code/`, relative
to this skill's base directory). Pick axes with the trigger rules in
`../review-code/SKILL.md` ("Axes and dispatch triggers"), and use its finding
schema. Ignore its Process and Output sections because this skill has its own.

Spawn one `Explore` agent per axis, all in one message. Each brief holds:

- the axis file's full content
- the paths to `$SCRATCH/diff.patch` and `$SCRATCH/repo`
- a short list of what existing threads already raised, so they don't repeat it
- the instruction to report findings only and not edit or post anything

While they run, do your own lens-free pass over the diff. Your own read found
the best finding last time (an accidental component rename), so don't skip it.

## 3. Consolidate and pre-filter

1. Merge duplicates across finders and your own list. Same root cause means one
   finding.
2. Drop anything already raised in an existing thread and fixed. Drop anything
   the author declined, unless you have evidence it's worse than they thought.
3. Drop findings that clearly fail the bar (the generic bar below plus the
   domain file). Don't spend an auditor on them.
4. If more than about 12 remain, keep the strongest 12.

Print the kept list and the dropped list, one line each with the reason.

## 4. Audit

Spawn one `general-purpose` agent per kept finding, all in one message. Build
each brief from `auditor.md`. Auditors are read-only.

When they report back:

- Drop every `DROP`.
- Apply every `CORRECTIONS` line. Auditors often reframe a finding: a "bug"
  becomes a naming question, a broad claim becomes a narrow one, or a wrong
  argument goes away. Write the comment from the corrected version.
- Auditors sometimes notice a separate issue on the way. Verify it yourself
  before using it.

## 5. Apply the bar again

Read the survivors as I would, with the generic bar and the domain file.
Auditors judge whether a finding is true. You judge whether I would bother
saying it.

## 6. Write the comments

Follow `voice.md`.

## 7. Post as a draft review

- Check every anchor against the PR head. It must be a RIGHT-side line inside a
  diff hunk, and that line must hold what the comment talks about. Pick the line
  where the comment reads naturally. For "you changed Mark to Flag above", that
  is a line that still says "Mark".
- Use the posting mechanics in `../jj/fix-pr.md` ("Posting mechanics"): reuse an
  existing pending review, never send `event`, pipe JSON with `--input -`. Don't
  add the `[Written by Claude]` prefix. These are my comments.
- A comment on an existing thread goes in as a pending reply with the GraphQL
  flow there, not as a new top-level comment.
- Leave the review body empty. Anything that doesn't fit on a diff line goes
  into the report to me, not on the PR.
- Never submit, approve or request changes. I do that.

Then save the draft record for calibration, at
`~/.cache/bayov-code-review/<owner>_<repo>_<pr>.json`:

```json
{
    "pr_url": "...",
    "head_sha": "...",
    "domains": ["frontend"],
    "posted": [{ "path": "...", "line": 0, "body": "...", "finding": "..." }],
    "dropped": [
        {
            "finding": "...",
            "stage": "pre-filter | auditor | bar",
            "reason": "..."
        }
    ]
}
```

## 8. Report

- One line with the review link (`.../pull/$PR/files`) and the comment count.
- A table with every finding: posted, dropped by auditor, or dropped by bar,
  with a one-line reason.
- Anything worth my attention that isn't a comment (a side finding you couldn't
  anchor).

## Generic bar

These hold in every domain. The domain file adds to them.

Raise:

- Bugs a user or caller would hit, even small ones.
- Changes that look accidental, like an unexplained rename. Ask about them, and
  don't claim they're mistakes.
- Real misbehavior that has a cheap, concrete fix.
- Inconsistency the PR itself introduced, like two words for the same action.

Drop:

- Anything CI shows the author anyway: failing tests, type errors, lint.
- Second-guessing a design choice the author made on purpose. Commits and thread
  replies show what was deliberate.
- Asks about the PR itself: description, test plan, scope.
- Style choices copied from code that already shipped elsewhere. They belong in
  a follow-up for both places. A real bug in copied code is still worth a
  comment, with a short "X does the same."

## Calibrate

Run with `calibrate <pr>` after I submit a review that this skill drafted.

1. Read the draft record from
   `~/.cache/bayov-code-review/<owner>_<repo>_<pr>.json`. If it's missing, ask
   me for the draft text.
2. Fetch what I submitted on that PR: my review comments (`pulls/$PR/comments`)
   and my review bodies.
3. Match each draft comment to a submitted one by path and nearby line. Sort
   each into kept as is, rephrased, dropped, or added by me.
4. Update the files:
   - Rephrased: add a before/after pair to `voice.md` only when it shows a
     pattern the rules don't cover yet. Turn a clear pattern into a rule. Keep
     about 8 pairs and replace the weakest one.
   - Dropped, or added by me: log it in the domain file's calibration log, and
     change that domain's bar when it shows a rule. A comment I added that the
     draft record lists as dropped means the bar was too high there.
   - Move a rule into the generic bar only after it holds in two domains.
   - When a domain clearly needs its own bar and has no file (for example infra:
     Helm, Terraform, CI config), create one with the same layout.
5. When the reason for a drop or an edit isn't clear, ask me. Don't guess a
   rule.
6. Show me what changed in a short summary.

## IMPORTANT: AVOID

- Posting anything that isn't pending, or submitting the review.
- Commenting on what an existing thread already covers.
- Writing a comment from the original finding when the auditor corrected it.
- Evidence dumps in comments. The audit is for you, not for the author.
- Running finders or auditors one at a time. Send each batch in one message.
