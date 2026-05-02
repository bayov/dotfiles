## Problem Statement

The problem the user is facing, from the user's perspective. Describe the
situation and the pain — not the solution.

## Solution

A short narrative (a paragraph or two) of the solution from the user's
perspective. What changes for them? What can they now do that they couldn't
before? No implementation detail.

## Target Users

The personas this PRD is for. One short paragraph per persona covering who they
are, what they're trying to accomplish, and the context they operate in. Keep it
tight — this is not a research deck.

## User Stories

A numbered list of user stories, one per subsection. Each subsection's heading
is a short descriptor; the body is the story sentence plus any clarifying notes.

<user-story-example>
### S1: See account balance

As a mobile bank customer, I want to see balance on my accounts, so that I can
make better informed decisions about my spending.
</user-story-example>

The list should be extensive and cover all aspects of the feature, including
edge cases and failure modes from the user's perspective.

## Success Criteria

How we'll know the feature is working. Measurable outcomes, not feature
checklists. Examples:

- {Behavior X} happens for {Y%} of {actor} within {timeframe}.
- {Pain point} reported in support drops by {amount}.
- {Workflow} that previously took {N} steps now takes {M}.

If a criterion can't be measured, state how it will be observed instead (e.g.
"product team reviews session recordings and confirms…").

## Scope

### In scope

- Bullet list of capabilities this PRD commits to.

### Out of scope

- Bullet list of capabilities explicitly deferred. Note adjacent things a reader
  might reasonably expect but that aren't included, so the boundary is
  unambiguous.

## Risks

Product and adoption risks — not engineering risks. Each risk paired with a
mitigation. Examples:

- Users may keep using the old workflow out of habit. Mitigation: …
- The new flow conflicts with {other feature}'s assumptions. Mitigation: …

Engineering risks (scaling, migration, dependencies) belong in the design doc.

## Glossary Changes

Proposed additions, updates, and removals to `GLOSSARY.md` that this PRD depends
on. Carry forward any changes from `prd_interview.md` that haven't yet been
applied, plus any new terms surfaced while drafting the PRD:

### New term: **Term**

A concise definition of the new term.

### Updated term: **Term**

The proposed change to the existing definition.

### Removed term: **Term**

Reason for removing the term.
