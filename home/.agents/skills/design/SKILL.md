---
name: design
description: Produce a design doc
disable-model-invocation: true
---

Produce a design doc: `plan/{slug}/design.md`.

A design doc describes **how** the change will be built — at a level engineers
can review and implement against. It assumes the PRD has settled the **what /
who / why**.

Focus on architecture and approach, not line-by-line implementation. Reference
`prd.md` for motivation and requirements rather than restating them.

## Input

Before writing, read whichever of these exist:

- `plan/{slug}/prd.md` — the product contract. Treat it as authoritative; do not
  relitigate scope or success criteria here.
- `plan/{slug}/design_interview.md` — the design alignment session's Q&A and
  proposed glossary changes. Treat answers as authoritative; do not relitigate
  them.
- `GLOSSARY.md` (and relevant sub-glossaries) — the project's existing
  ubiquitous language. Use these terms verbatim in the design doc.

## Steps

1. Confirm with the user: component boundaries, key interfaces, the testing
   strategy, and the rollout plan. If any of these are unresolved or contested,
   send them back through `design-interview` — do not invent answers in the
   design doc.

2. Copy [template.md](./template.md) to `plan/{slug}/design.md` and fill out
   each section.

## Principles

### Prefer deep modules

Following Ousterhout (_A Philosophy of Software Design_).

A module's **interface** is everything callers must know to use it: method
names, parameters, return shapes, error cases, ordering rules, side effects,
configuration. A module's **implementation** is what it actually does.

- **Deep**: small interface, large implementation. Callers learn little and get
  a lot.
- **Shallow**: interface size ≈ implementation size. The module costs about as
  much to use as it saves.

Prefer deep. Every interface element is a cost paid by every caller and every
future change; shallow modules add cost without proportional benefit.

Rules:

- Push complexity down behind narrow interfaces. Callers should know less than
  implementers.
- Delete pass-through layers, thin wrappers, and adapters that only forward
  calls or rename fields. Merge into caller or callee.
- Hide parameters with obvious defaults; expose overrides only where needed.
- If two modules must share many internal details to function, the boundary is
  wrong — redraw it.
- Information leakage: if a change inside module A regularly forces a change in
  module B, A's interface is exposing internals it shouldn't.

Smell test: list the interface and the implementation. If they are similar in
size, the module is shallow — redesign.

Example — a `RateLimiter`:

Shallow (avoid):

```
newBucket(key, capacity, refillRate, refillUnit, burstSize, clock)
tryAcquire(bucket, tokens, now)
peek(bucket) -> currentTokens, lastRefillTime
setCapacity(bucket, capacity)
setRefill(bucket, rate, unit)
serializeBucket(bucket) -> bytes
deserializeBucket(bytes) -> bucket
```

Callers must understand token buckets, manage clocks, serialize state, and pick
refill units. Switching to a sliding-window algorithm breaks every caller.

Deep (prefer):

```
allow(key) -> Allowed | Denied(retryAfter)
```

The limiter owns the algorithm, clock, persistence, and bursts. Swapping
algorithms or moving state to Redis is invisible to callers.

In the design doc, justify each module by what its interface hides — not by what
it contains.

### Testing is part of the design

Every design must say how it will be tested. Treat the testing strategy as a
first-class section, not an afterthought. The strategy should:

- Identify the riskiest parts of the design and explain how tests pin them down.
- State the level each behavior is tested at (unit, integration, end-to-end) and
  why that level is the right one.
- Call out invariants and failure modes the tests must cover (concurrency,
  partial failures, retries, migration paths).
- Note anything that _can't_ be tested automatically and how it will be verified
  instead.
