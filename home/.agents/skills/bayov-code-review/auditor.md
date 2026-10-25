# Auditor brief

Fill in the placeholders and send one brief per finding. Keep the "Check"
section specific: name the files, callers, versions or threads that would prove
or disprove this finding.

```
You are auditing ONE candidate code-review finding on {PR_URL} (head SHA
{HEAD_SHA}, base {BASE}). Decide if it is worth raising as a PR comment. Try
hard to refute it. Check the real code, not just the claim. Read-only: do not
post anything to GitHub and do not modify files.

Resources:
- PR diff: {SCRATCH}/diff.patch
- PR head checkout: {SCRATCH}/repo (base version: `git -C {SCRATCH}/repo show
  base:<path>`). Prefer this over the GitHub API, which is rate-limited.
- Existing review threads: {SCRATCH}/threads.json
- {EXTRA_RESOURCES}

Finding to audit:
"{FINDING}"

Check:
{CHECKS}

Also weigh:
- Is it new in this PR, or copied from code that already shipped?
- Did a bot or the author already discuss it? What did the author decide?
- Is it something the author made on purpose (commit messages, replies)?
- Would a busy reviewer bother saying it?

Reply in this exact format, nothing else:
VERDICT: RAISE | DROP
CONFIDENCE: high | medium | low
ANCHOR: <repo-relative path>:<line on the PR head (RIGHT side), inside a diff
hunk of that file> | review-body
EVIDENCE: <2-4 short bullets with what you verified>
CORRECTIONS: <what in the finding is wrong, overstated or should be reframed,
or "none">
SIDE FINDINGS: <a separate real issue you noticed on the way, or "none">
```
