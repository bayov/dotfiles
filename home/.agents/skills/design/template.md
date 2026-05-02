## Context

Background and current state needed to understand the design. Cover:

- **Background:** what exists today and how it works at a high level.
- **Current state:** the relevant pieces of the system this design touches.
- **Constraints:** technical, organizational, or timeline limits the design must
  respect.
- **Stakeholders:** teams or systems that depend on this area or will be
  affected by the change.

Reference `prd.md` for motivation and requirements — don't restate them.

## Goals / Non-Goals

**Goals:** the engineering outcomes this design commits to (e.g. "no new infra
dependencies", "online migration with zero downtime", "preserve existing API
contract").

**Non-Goals:** outcomes a reader might reasonably expect but that this design
explicitly excludes. Make the boundary unambiguous.

Product-level goals belong in the PRD, not here.

## Overview

A short narrative (a paragraph or two) of the proposed design. The shape of the
solution at a glance: the main components, how they fit together, and the core
idea that makes the design work.

## Components

One subsection per component / module. Each component should be deep: hide
substantial complexity behind a small, simple interface. Justify the boundary by
what the interface hides, not by what the component contains. Merge or remove
any component that exists only to forward calls.

### {Component name}

**Responsibility:** one sentence on what this component owns.

**Interface:** the shape exposed to callers — key methods / endpoints / events
with their inputs, outputs, and error cases. Be concrete: field names, types,
nullability.

**Hides:** what complexity lives inside that callers don't have to think about.

**Depends on:** other components or external systems it relies on.

## Data Model

Schemas, tables, message shapes, or other persisted/transmitted structures.
Include field names, types, nullability, indexes, and ownership. Note any
changes to existing schemas.

## Data Flow

Walk a representative request or event through the system end-to-end. Cover the
golden path and at least one failure path (timeout, partial failure, concurrent
write, retry). Sequence diagrams or numbered steps both work.

## Decisions

One subsection per non-trivial technical choice. Focus on architecture and
approach, not line-by-line implementation.

### D1: {The decision}

**Choice:** what was picked.

**Rationale:** why this wins for our constraints.

**Alternatives considered:**

- **{Alternative A}** — why it was rejected.
- **{Alternative B}** — why it was rejected.

## Testing Strategy

How we'll know the design is correct, and how we'll keep it correct.

- **Risk map:** the riskiest parts of the design and the tests that pin them
  down.
- **Levels:** what's covered by unit vs. integration vs. end-to-end tests, and
  why each behavior lives at the level it does.
- **Invariants & failure modes:** concurrency, partial failures, retries,
  migration paths, permission boundaries — each with a test that guards it.
- **Manual / observational checks:** anything that can't be tested automatically
  and how it will be verified instead.

## Risks / Trade-offs

Known limitations and engineering risks. Each item paired with a mitigation, or
an explicit decision to accept it.

- **{Risk or trade-off}** → {mitigation, or "accepted because …"}
- **{Risk or trade-off}** → {mitigation, or "accepted because …"}

Product and adoption risks belong in the PRD, not here.

## Migration Plan

How the change ships safely.

- **Deploy steps:** ordered migration steps (schema, data, config, code) and any
  required ordering between them.
- **Backwards compatibility:** the compatibility window, if any, and what
  callers must do to stay compatible.
- **Rollout controls:** for each gate (flag, staged rollout, kill switch): what
  it gates, launch state, ramp criteria, kill conditions, and — if not permanent
  — removal trigger + owner.
- **Rollback:** how we revert if something goes wrong, and any state that can't
  be rolled back.

## Observability

What we'll watch to know the system is healthy after rollout: metrics, logs,
traces, alerts. Tie each signal to a specific failure mode it would catch.

## Open Questions

Outstanding decisions or unknowns still to resolve, with the owner and the
trigger for resolving each.

- **{Question}** — owner: {who}; resolve by: {milestone or date}.

If an open question is load-bearing for the design, send it back through
`design-interview` before implementation begins.

## Glossary Changes

Proposed additions, updates, and removals to `GLOSSARY.md` that this design
depends on. Carry forward any changes from `design_interview.md` that haven't
yet been applied, plus any new terms surfaced while drafting the design.

Only include terms meaningful to domain experts. Internal technical names
(component names, private types) stay in this doc, not the glossary.

### New term: **Term**

A concise definition of the new term.

### Updated term: **Term**

The proposed change to the existing definition.

### Removed term: **Term**

Reason for removing the term.
