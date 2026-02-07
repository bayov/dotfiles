---
name: codify-insights
description: Codifies pending insights from .pensieve/insights into project documentation (README.md, source-code docs, AGENTS.md). Use ONLY when explicitly requested by the user (e.g., "process insights", "codify insights"). NEVER use this skill proactively.
---

# Codify Insights

This skill processes all previously recorded codebase insights into permanent
project documentation. Each insight is routed to the most appropriate
documentation target based on its audience:

- **Human-relevant insights** (conventions, APIs, setup, architecture, gotchas
  that affect developers) go into `README.md`, source-code comments/docstrings,
  or other human-facing project docs.
- **Agent-only insights** (tool quirks, prompt patterns, agent-specific
  workarounds that humans would never encounter) go into `AGENTS.md` (either
  root-level or nested `AGENTS.md`, depending on the relevancy of the insight).

Most insights are human-relevant. Merge insights into `AGENTS.md` files only if
a human developer would find it irrelevant.

The goal is high-quality, reusable documentation for both humans and agents,
while keeping docs clean and avoiding duplication.

## Workflow

### 1. Discovery

First, list all pending insights, **grouped by their target path**.

Run the helper script:

```bash
# IMPORTANT: RUN FROM PROJECT ROOT

# If you're Claude Code:
export PENSIEVE_ROOT="${CLAUDE_PLUGIN_ROOT}/"

# Every other agent:
export PENSIEVE_ROOT=""

python3 ${PENSIEVE_ROOT}scripts/get_pending_insights.py
```

This returns a JSON list where each entry represents a **target file/directory**
and contains a list of pending insights for that directory:

```json

[
  {
    "target_path": "foo/bar/",
    "insights": [
      {
        "filename": "...",
        "trigger": "...",
        "content": "..."
      }
    ]
  }
]

```

### 2. Analysis & Integration

Iterate through the **groups** returned by the script. For **EACH**
`target_path` group:

#### A. Analyze Target Context

**Survey documentation**: For this `target_path`, identify and read all existing
documentation files:
    * `README.md` in the target directory (or nearest parent).
    * `AGENTS.md` in the target directory (or nearest parent).
    * Source files that contain relevant docstrings/comments.
    * Any other docs (e.g., `CONTRIBUTING.md`, `API.md`).

#### B. Classify & Formulate Updates

Review **ALL** insights in this group together.

1. **Classify audience**: For each insight, determine:
    * **Human-relevant**: Would a human developer benefit from this? (e.g., API
      usage, setup steps, architectural decisions, non-obvious conventions,
      gotchas, dependency notes). Most insights fall here.
    * **Agent-only**: Is this exclusively relevant to AI coding agents? (e.g.,
      tool invocation quirks, prompt-specific workarounds, agent context
      management). Only classify as agent-only if a human developer would find
      it irrelevant.

2. **Consolidate**: Combine related observations into coherent paragraphs or
   rules. Group by documentation target.

3. **Conflict Authority**: If a new insight contradicts existing documentation,
   the **new insight takes precedence**. Treat it as the most recent truth and
   overwrite the outdated information.

#### C. Apply Changes

1. **Update human documentation** (for human-relevant insights):
    * Merge into `README.md` if you deem the insight important enough to be
      merged into the project's or subdirectory's `README.md`. Create one if no
      relevant doc exists.
    * Where appropriate, update source-code comments/docstrings in files under
      `target_path`.
    * Match the style and structure of existing docs.

2. **Update `AGENTS.md`** (for agent-only insights):
    * Merge into the `AGENTS.md` at the relevant path. Create if none exists.
    * Avoid duplicating content already covered in human-readable docs.

#### D. Remove

Once the insight is successfully incorporated (or deemed redundant/invalid),
remove the raw file to prevent re-processing.

Run for each processed file in the group:

```bash
# IMPORTANT: RUN FROM PROJECT ROOT

# If you're Claude Code:
export PENSIEVE_ROOT="${CLAUDE_PLUGIN_ROOT}/"

# Every other agent:
export PENSIEVE_ROOT=""

python3 ${PENSIEVE_ROOT}skills/codify-insights/scripts/remove_insight.py "<filename>"
```

### 3. Completion

After processing all pending insights, report a summary of what was updated to
the user.