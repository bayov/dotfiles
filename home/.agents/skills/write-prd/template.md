> **Writing rules:** Whole doc ≤400 words. Bullets only outside the Problem
> Statement paragraph. ≤20 words per bullet. Omit any section that has nothing
> load-bearing to record — do not leave empty headings.

## Problem Statement

The problem the user is facing, from the user's perspective. One paragraph, ≤80
words. Situation and pain — not the solution.

## Solution

What changes for the user, from their perspective. Bullets, outcome only, no
implementation detail.

## Target Users

One bullet per persona: who they are, what they're trying to accomplish, context
they operate in. ≤25 words each.

## User Stories

A numbered list, one subsection per story. Heading is a short descriptor; body
is the story sentence plus any clarifying notes (≤2 lines).

<user-story-example>
### S1: See account balance

As a mobile bank customer, I want to see balance on my accounts, so that I can
make better informed decisions about my spending.
</user-story-example>

Cover the behavior the user can observe — golden path plus failure/edge modes
that change the UX. Skip variants that share the same shape as an existing
story.

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

Proposed additions, updates, and removals to `$GLOSSARY_DIR/GLOSSARY.md` that
this PRD depends on. Carry forward any changes from `prd_interview.md` that
haven't yet been applied, plus any new terms surfaced while drafting the PRD:

### New term: **Term**

A concise definition of the new term.

### Updated term: **Term**

The proposed change to the existing definition.

### Removed term: **Term**

Reason for removing the term.
