# Pensieve: Codebase Knowledge Management

**Goal:** Maintain a living memory of the codebase, ensuring that hard-won
knowledge is preserved and automatically shared with future developers and AI
agents.

## Core Mandates

1. **Record Immediately & Proactively:** Use the `record-insight` skill
   **IMMEDIATELY** and **PROACTIVELY** whenever you discover new or undocumented
   information about the codebase structure, patterns, or logic.
2. **Proactive Discovery:** Whenever you discover undocumented conventions,
   structural patterns, or logic that isn't immediately obvious, record it as
   an insight. This includes understanding how different components interact,
   identifying "source of truth" files, or uncovering intended but unwritten
   architectural rules.
3. **Codify on Request:** Use the `codify-insights` skill *only* when explicitly
   asked (e.g., "process insights"). This merges recorded insights into the
   appropriate project documentation files (e.g., README.md, source-code docs,
   AGENTS.md; depending on target audience).
