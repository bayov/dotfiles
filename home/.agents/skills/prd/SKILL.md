---
name: prd
description: Produce a PRD
disable-model-invocation: true
---

Produce a PRD (Product Requirements Document): `plan/{slug}/prd.md`.

## Inputs

Before writing, read whichever of these exist:

- `GLOSSARY.md` (and relevant sub-glossaries) — the project's ubiquitous
  language. Use these terms verbatim in the PRD.
- `plan/{slug}/align_session.md` — the alignment session's Q&A and proposed
  glossary changes. Treat answers as authoritative; do not relitigate them.

## Ubiquitous language

- Use terms from `GLOSSARY.md` exactly as defined. If a term in
  `align_session.md`'s proposed glossary changes conflicts with the current
  glossary, prefer the proposed term and surface the conflict in
  `Glossary Changes`.
- If you find yourself reaching for a term that isn't in the glossary and is
  meaningful to domain experts, add it to `Glossary Changes` rather than
  inventing it inline.

## Steps

1. Explore the current state of the codebase.

2. Sketch out the major modules to build or modify. Actively look for
   opportunities to extract deep modules that can be tested in isolation. A deep
   module (as opposed to a shallow module) is one which encapsulates a lot of
   functionality in a simple, testable interface which rarely changes.

3. Check with the user that these modules match their expectations. Check with
   the user which modules they want tests written for.

4. Write `plan/{slug}/prd.md` using the template below.

<prd-template>

## Problem Statement

The problem that the user is facing, from the user's perspective.

## Solution

The solution to the problem, from the user's perspective.

## User Stories

A LONG, numbered list of user stories. Each user story should be in the format
of:

1. As an {actor}, I want a {feature}, so that {benefit}.

<user-story-example>
1. As a mobile bank customer, I want to see balance on my accounts, so that I can make better informed decisions about my spending.
</user-story-example>

This list of user stories should be extremely extensive and cover all aspects of
the feature.

## Glossary Changes

Proposed additions, updates, and removals to `GLOSSARY.md` that this PRD depends
on. Carry forward any changes from `align_session.md` that haven't yet been
applied, plus any new terms surfaced while drafting the PRD. Use the same shape
as `align_session.md`'s "Proposed glossary changes" section:

### New term: **Term**

A concise definition of the new term.

### Updated term: **Term**

The proposed change to the existing definition.

### Removed term: **Term**

Reason for removing the term.

## Implementation Decisions

A list of implementation decisions that were made. This can include:

- The modules that will be built/modified
- The interfaces of those modules that will be modified
- Technical clarifications from the developer
- Architectural decisions
- Schema changes
- API contracts
- Specific interactions

Do NOT include specific file paths or code snippets. They may end up being
outdated very quickly.

## Testing Decisions

A list of testing decisions that were made. Include:

- A description of what makes a good test (only test external behavior, not
  implementation details)
- Which modules will be tested
- Prior art for the tests (i.e. similar types of tests in the codebase)

## Out of Scope

A description of the things that are out of scope for this PRD.

## Further Notes

Any further notes about the feature.

</prd-template>
