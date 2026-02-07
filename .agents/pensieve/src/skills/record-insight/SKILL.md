---
name: record-insight
description: Records codebase insights (discoveries, conventions, hidden dependencies) for both human developers and AI agents. Use proactively whenever discovering new or undocumented information about the codebase structure, patterns, or logic.
---

# Record Insight

This tool allows you to permanently record codebase insights into the project's
documentation. An insight is a piece of knowledge, such as a tricky bug, a
hidden dependency, a counter-intuitive configuration or tool usage, out-of-date
documentation, an architectural constraint, or any other discovery that future
developers or agent invocations would benefit from.

## Usage

Run the following python script to record an insight:

```bash
# IMPORTANT: RUN FROM PROJECT ROOT

# If you're Claude Code:
export PENSIEVE_ROOT="${CLAUDE_PLUGIN_ROOT}/"

# Every other agent:
export PENSIEVE_ROOT=""

python3 ${PENSIEVE_ROOT}skills/record-insight/scripts/record_insight.py --slug "<SLUG>" --path "<PATH>" --trigger "<TRIGGER>" --insight "<INSIGHT>"
```

### Arguments

- `--slug`: A short, descriptive topic string (snake_case). Examples:
  `docker_permissions`, `auth_token_format`, `importer_race_condition`.
- `--path`: The directory or file path this insight applies to (e.g.,
  `src/backend`, `importer/import.py`).
    - It is important to choose the correct file/directory for the insight, so
      that later we can easily incorporate the insight into the documentation at
      the right place.
    - Too specific paths may lead to agents missing an important insight when
      operating on sibling or parent project directories.
    - Too general paths may lead to agents' context being polluted with
      irrelevant insights.
- `--trigger`: The specific error message, bug, or context that led to this
  discovery. What triggered the need for this insight?
- `--insight`: The detailed lesson, fix, constraint, etc. Be specific, concise,
  and actionable.

## Protocol (The Constitution)

1. **Record Immediately**: Use this tool **IMMEDIATELY** upon learning a new
   insight about the codebase that future agent invocations should be aware of.
    * *Example:* You discovered that tool `foo` fails when you run it from the
      project root directory, but succeeds when you run it from the `src`
      directory? **Record it.**
    * *Example:* You discovered that under directory `src/roles`,
      `user.get_email()` must be used instead of accessing `user.email`
      directly? **Record it.**
    * *Example:* You found that the bank importer crashes if the date format is
      `DD/MM/YYYY` instead of `YYYY-MM-DD`? **Record it.**
2. **Proactive Discovery**: Do not wait for an error or a bug to record an
   insight. If you explore the codebase and discover undocumented conventions,
   structural patterns, or logic that isn't immediately obvious from the file
   structure, record it immediately. This includes understanding how different
   components interact, identifying 'source of truth' files, or uncovering
   intended but unwritten architectural rules.
3. **Write-Only**: When using this skill, your job is only to record and move
   on. A different agent will handle the analysis and consolidation of these
   insights into the project's documentation later on (README.md,
   source-code docs, or AGENTS.md as appropriate).
