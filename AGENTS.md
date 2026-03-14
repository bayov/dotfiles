Agent Guidelines
================

File References
---------------

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

AGENTS.md
---------

This projects uses AGENTS.md files to provide context for coding agents. You do
not need to scan for these files, they are **all** listed here:

- AGENTS.md (this file)

**NON-NEGOTIABLE:** You MUST adhere to all AGENTS.md guidelines along the path
to any file (or directory) you read, modify, or execute.

For example, if you are reading, modifying, or executing `path/to/some/file.py`,
you MUST adhere to the following guidelines:

- AGENTS.md (root, this file)
- path/AGENTS.md (if exists)
- path/to/AGENTS.md (if exists)
- path/to/some/AGENTS.md (if exists)

Notes:

- **Conflict resolution:** The more nested AGENTS.md takes priority.
- Instructions from out-of-path AGENTS.md can be ignored. In the example above,
  guidelines in `path/to/other/AGENTS.md` can be ignored when reading,
  modifying, or executing `path/to/some/file.py`.

Pensieve
--------

FORCE READ @.agents/pensieve/src/INSTRUCTIONS.md
