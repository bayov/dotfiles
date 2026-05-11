---
name: review-code
description: Perform code review
argument-hint: (pr <number> | range <revset> | code diff) [axis <axis>, ...]
disable-model-invocation: true
model: opus
effort: high
---

Produce a code review of the current change, dispatching specialist subagents in
parallel — each with a focused lens — and consolidating their findings into a
single ranked list of review comments.

## Input

Determine the diff to review:

- If the argument contains `pr <number>`: fetch via `gh pr diff <number>` and
  use that.
- If the argument contains `range <revset>`: use the given jj revset.
- Otherwise (default): identify the bookmark at or near `@` and use its commits
  vs `trunk()`. If no bookmark exists yet, fall back to `trunk()..@`.

Determine explicit axes (if any):

- If the argument contains `axis <name>[,<name>...]` (e.g. `axis security`,
  `axis security,tests`): parse the comma-separated list as the **explicit axis
  set**. Expand any sub-axis names (e.g. `conventions/python`) literally; expand
  a parent name (e.g. `conventions`) to include its sub-axes.
- Store the explicit axis set; it overrides the trigger filter and judgment gate
  in steps 1–2 below.

## Axes and dispatch triggers

Each axis lives in its own file under this skill's directory. The dispatcher
reads axis files but never executes them itself — they are subagent prompts.
**Do not read an axis file until its trigger conditions are met and it is about
to be dispatched.** Trigger decisions use only the descriptions below.

### general (`general.md`)

Always runs when any code file changed.

### conventions (`conventions.md`)

Always runs when any code file changed.

### conventions/python (`conventions/python.md`)

Runs when any diff path matches `**/*.py`.

### conventions/go (`conventions/go.md`)

Runs when any diff path matches `**/*.go`.

### production (`production.md`)

Always runs when any code file changed.

### tests (`tests.md`)

Always runs when any code file changed.

### performance (`performance.md`)

Always runs when any code file changed.

### observability (`observability.md`)

Always runs when any code file changed.

### security (`security.md`)

Runs when any diff path matches: `**/*auth*`, `**/*authn*`, `**/*authz*`,
`**/*crypt*`, `**/*secret*`, `**/*credential*`, `**/*password*`, `**/*token*`,
`**/*session*`, `**/migrations/**`, `**/schema/**`, `**/Dockerfile*`, `**/*.tf`,
`**/*.yaml`, `**/*.yml`

OR any added/changed line contains: `exec(`, `execSync`, `child_process`,
`subprocess.`, `os.system`, `shell=True`, `eval(`, `Function(`,
`dangerouslySetInnerHTML`, `innerHTML`, `raw SQL`, `fmt.Sprintf("SELECT`,
`fmt.Sprintf("INSERT`, `fmt.Sprintf("UPDATE`, `fmt.Sprintf("DELETE`, `f"SELECT`,
`f"INSERT`, `f"UPDATE`, `f"DELETE`, `crypto/md5`, `hashlib.md5`, `hashlib.sha1`,
`crypto/sha1`, `Math.random`, `rand.Int`, `verify=False`, `InsecureSkipVerify`,
`rejectUnauthorized: false`

### dependencies (`dependencies.md`)

Runs when any diff path matches: `**/package.json`, `**/package-lock.json`,
`**/yarn.lock`, `**/pnpm-lock.yaml`, `**/requirements*.txt`,
`**/pyproject.toml`, `**/poetry.lock`, `**/uv.lock`, `**/Pipfile*`, `**/go.mod`,
`**/go.sum`, `**/Cargo.toml`, `**/Cargo.lock`, `**/Gemfile*`,
`**/composer.json`, `**/composer.lock`

## Process

### 1. Mechanical trigger filter

**If an explicit axis set was provided**, skip this step entirely — all named
axes are candidates by definition. Jump to step 2.

Otherwise:

1. Compute the changed-file list and the changed content. For a PR, use
   `gh pr diff`; otherwise use `jj diff --name-only`.
2. For each axis in the sections above, apply its trigger rule:
   - `always` → candidate if at least one code file changed.
   - `path matches` → candidate if any diff path matches any listed glob.
   - `contents` → candidate if any added/changed line contains any listed
     string.
3. Discard non-candidates immediately. They do not proceed.

### 2. LLM judgment gate

**If an explicit axis set was provided**, skip the judgment gate — run all named
axes unconditionally. Print:

```
Running (explicit):  <comma-separated explicit axes>
Skipping:            (none — explicit axes override filtering)
```

Otherwise, you hold the dispatcher's context. For each candidate axis:

- Glance at the diff's shape (file types, magnitude, what it's doing) and the
  axis name and trigger description in the sections above.
- Decide: would running this axis plausibly find something? Bias toward running.
  Skip only when the axis is clearly orthogonal to this diff (e.g.
  `dependencies` survived mechanical filter because `package.json` was touched,
  but the change is only a version-pin to a patch release of a dev-only lib).

Print the decision before fanning out:

```
Running:   security, tests, production, conventions, conventions/python, observability
Skipping:  dependencies (only a dev-dep patch bump; no transitive risk)
```

### 3. Parallel subagent fan-out

Read each axis file **only now**, only for axes that will run. Do not read any
axis file before this step; do not read files for axes that were filtered out.

For each axis to run, spawn one `Agent` with:

- `subagent_type: "Explore"` — review subagents must not edit code.
- `description`: e.g. `"Security review of current diff"`.
- `prompt`: a self-contained brief composed of:
  1. The axis file's full content (so the subagent has its lens and severity
     calibration).
  2. The diff (or the path/PR identifier and instructions to fetch it).
  3. The output schema (see below).
  4. The explicit instruction: do not edit any files; report findings only.

Send all subagent tool calls in a single message to maximize parallelism.

#### Subagent output schema

Each subagent returns a JSON-shaped list of findings:

```
[
  {
    "severity": "critical" | "major" | "minor",
    "file": "<repo-relative path>" | null,
    "line_start": <int> | null,
    "line_end":   <int> | null,
    "title":      "<one-line description>",
    "detail":     "<one or two paragraphs>",
    "recommendation": "<concrete suggested fix>"
  },
  ...
]
```

`file`/`line_*` are null for repo-wide findings (e.g. "new dep with known CVE",
"AC X has no covering test"). The subagent may return an empty list.

### 4. LLM consolidator

You receive N findings lists from the subagents. Produce the final review:

1. **Dedupe semantically.** Group findings that describe the same underlying
   issue, even when they cite different files/lines or use different vocabulary.
   Use judgment: same root cause = one comment. When in doubt, keep separate;
   over-merging is worse than under-merging.
2. **Merge each group** into one comment:
   - `severity`: max across the group (critical > major > minor).
   - `detected_by`: union of axes from the group.
   - `title`, `detail`, `recommendation`: synthesize a single coherent version
     that captures the strongest framing across the group. Do not lose useful
     specifics from any contributor.
3. **Sort**: critical first, then major, then minor. Within a severity, preserve
   a stable order (by file path, then line).
4. **Format as PR-review-style comments** (see Output below).

## Output

Local report only. Never post to a PR. Never edit code.

Header:

```
review-code on <diff-identifier>

Ran:     <comma-separated axes that fanned out>
Skipped: <axes filtered out, with reason>

<N> comments — <C> critical, <M> major, <m> minor
```

Then each finding as a comment block:

```
[<severity>] <title>
<file>:<line_start>-<line_end>            (or "(repo-wide)")
Detected by: <axis>, <axis>

<detail>

Suggested fix:
<recommendation>
```

Separate comments with a blank line. If there are zero findings, print:

```
No findings. <N> axes ran clean.
```

## IMPORTANT: AVOID

- Do not read axis files before step 3. Trigger decisions in steps 1–2 use only
  the trigger descriptions in this file. Read an axis file only when building
  its subagent prompt, and only if that axis will run.
- Do not edit code. Subagents use `Explore`; the dispatcher should never call
  Write/Edit either.
- Do not post comments to the PR (no `gh pr review`, no `gh api`). Local output
  only.
- Do not run subagents serially when they could run in parallel. Send all
  `Agent` tool calls in one message in step 3.
- Do not invent findings to fill out the report. An axis returning an empty list
  is a valid result.
- Do not skip the LLM judgment gate (step 2) unless an explicit axis set was
  provided. The gate is also where you print the skip list for auditability.
- Do not over-merge in consolidation. Two findings that share a file but
  describe different bugs stay separate.
