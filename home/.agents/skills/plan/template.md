## Problem & Solution

**Problem:** the situation and pain, from the user's perspective. One short
paragraph.

**Target user:** who this is for and the context they operate in. One or two
sentences.

**Solution:** a short narrative of what changes for the user — what they can now
do that they couldn't before. No implementation detail.

## Scope

### In scope

- Bullet list of capabilities this plan commits to.

### Out of scope

- Bullet list of things a reader might reasonably expect but that aren't
  included.

## Acceptance Criteria

The checklist `implement` works against. Each criterion is observable through
the public interface — outcome, not implementation.

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
- [ ] Glossary / docs owned by this change are updated (list each: e.g.
      `$GLOSSARY_DIR/GLOSSARY.md` term **X**, README example, in-repo doc).

> Drop the docs criterion if the change introduces no domain terms and touches
> no doc surface.

## Design

### Overview

A short narrative (a paragraph or two) of the proposed design. The shape of the
solution at a glance: main components touched, how they fit together, the core
idea.

### Components

One subsection per component this change introduces or meaningfully modifies.
Each component should be deep: hide substantial complexity behind a small
interface. Justify the boundary by what the interface hides.

#### {Component name}

**Responsibility:** one sentence on what this component owns.

**Interface:** the shape exposed to callers — key methods / endpoints / events
with their inputs, outputs, and error cases. Be concrete: field names, types,
nullability.

**Hides:** what complexity lives inside that callers don't have to think about.

**Depends on:** other components or external systems it relies on.

### Data Model

Schema, table, message-shape, or other persisted/transmitted changes. Field
names, types, nullability, indexes. Note changes to existing schemas.

> Omit if there are no data model changes.

### Data Flow

Walk a representative request through the system end-to-end. Cover the golden
path and the riskiest failure path (timeout, partial failure, concurrent write,
retry).

> Omit if the change is contained to a single component with no cross- component
> flow worth diagramming.

### Decisions

One subsection per non-trivial technical choice.

#### D1: {The decision}

**Choice:** what was picked.

**Rationale:** why this wins for our constraints.

**Alternatives considered:**

- **{Alternative A}** — why it was rejected.

> Omit this section if there are no non-trivial alternatives worth recording.

## Testing Strategy

- **Risk map:** the riskiest behaviors and the tests that pin them down.
- **Levels:** what's covered by unit vs. integration vs. end-to-end, and why.
- **Invariants & failure modes:** concurrency, partial failures, retries,
  migration paths — each with a test that guards it.
- **Manual checks:** anything not covered automatically and how it's verified
  instead.

## Migration & Rollout

How the change ships safely.

- **Deploy steps:** ordered migration steps (schema, data, config, code) and any
  required ordering between them.
- **Backwards compatibility:** the compatibility window, if any.
- **Rollout controls:** for each gate (flag, staged rollout, kill switch): what
  it gates, launch state, ramp criteria, kill conditions, removal trigger +
  owner.
- **Rollback:** how we revert, and any state that can't be rolled back.

> If the change has no rollout concerns (no schema change, no contract change,
> no gate), state that explicitly rather than omitting the section.

## Risks / Trade-offs

Known limitations and engineering risks. Each item paired with a mitigation, or
an explicit decision to accept it.

- **{Risk or trade-off}** → {mitigation, or "accepted because …"}

> Omit if there are no risks worth calling out.

## Open Questions

Outstanding decisions or unknowns, with owner and resolution trigger.

- **{Question}** — owner: {who}; resolve by: {milestone or date}.

If a load-bearing question is still open, send it back through `plan-interview`
before implementation begins.

> Omit if there are no open questions.

## Glossary Changes

Proposed additions, updates, and removals to `$GLOSSARY_DIR/GLOSSARY.md` that
this plan depends on. Carry forward changes from `plan_interview.md` plus any
new terms surfaced while drafting.

Only include terms meaningful to domain experts. Internal technical names stay
in this doc.

### New term: **Term**

A concise definition of the new term.

### Updated term: **Term**

The proposed change to the existing definition.

### Removed term: **Term**

Reason for removing the term.

> Omit this section if there are no glossary changes.
