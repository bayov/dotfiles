# Pensieve

> "Those who cannot remember the past are condemned to repeat it."
>
> — George Santayana

**Pensieve** is a set of agent instructions and skills to ensure that hard-won
insights during an agent's session are preserved and eventually shared with
future developers and AI coding agents.

## Core Concepts

## Workflow

Pensieve operates on a simple two-step workflow:

1. **Record (Immediate & Proactive):** Agents are instructed to use the
   `record-insight` skill to capture discoveries as they happen. These are
   stored as raw JSON files in `.pensieve/insights/`.
2. **Codify (On Request):** Periodically, or when requested by a human, the
   `codify-insights` skill processes these raw insights. It merges each insight
   to the appropriate documentation target (README.md, source-code docs, or
   AGENTS.md based on audience), and removes the raw files.

### Insights

An **insight** is a piece of knowledge about the codebase that isn't immediately
obvious. This includes:

- Tricky bugs and their fixes.
- Architectural constraints.
- Hidden dependencies between components.
- Counter-intuitive configurations.
- Undocumented coding conventions.

### Documentation Targets

Insights are codified into the most appropriate documentation:

- **For humans:** Insights are merged into READMe.md files or
  other code documentation files that are meant to be read by humans (including
  source-code comments).
- **For agents**: Insights are merged into `AGENTS.md` and other agent-only
  context files. This is for insights only to coding agents (tool quirks,
  agent-specific workarounds).

## Install as Git Submodule

From your project root:

```bash
# Run the install script directly
bash <(curl -s https://raw.githubusercontent.com/bayov/pensieve/main/install.sh)

# Or, if you've already cloned pensieve:
./path/to/pensieve/install.sh
```

The script will:
1. Add pensieve as a submodule at `.agents/pensieve`
2. Symlink each skill into `.agents/skills/{skill-name}`

After running, add an `AGENTS.md` to your project root:

```markdown
# Agent Guidelines

## File References

When you encounter a file reference (e.g., @path/to/file.md), read it into your
context based on the following rules:

**Lazy References:** (`@path/to/file.md`)
  - **Lazy Loading:** Read the file's contents only when you believe it to be
    relevant to the specific task at hand. Do NOT preemptively load all
    references.
  - **Caching:** Do NOT read a file you have already read.

**Forced References (`FORCE READ @path/...`):**
  - **Preemptive:** You MUST read the file immediately upon discovery.
  - **Caching:** Do NOT read a file you have already read.

## Pensieve

FORCE READ @.agents/pensieve/src/INSTRUCTIONS.md
```

## Project Structure

- `src/`: Contains the contents of the Pensieve plugin/extension.
  - `src/skills/` - The Pensieve skills.
  - `src/INSTRUCTIONS.md` - Instructions that should be added to the context of
    any Agent session, teaching the agent how and when to use the Pensieve
    skills.
- `claude-plugin/`: Manifest to load Pensieve as a Claude Code plugin. Contains
  soft-links to `src/`.
- `gemini-extensions/`: Manifest to load Pensieve as a Gemini CLI extension.
  Contains soft-links to `src/`.
