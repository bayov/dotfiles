---
name: write-design
description: Produce a design doc
disable-model-invocation: true
model: opus
effort: medium
---

Produce a design doc: `$PLAN_DIR/{slug}/design.md`.

> **Plan root:** `$PLAN_DIR` env var (defaults to `plan` if unset). **Glossary
> root:** `$GLOSSARY_DIR` env var (defaults to the project root if unset).

A design doc describes **how** the change will be built — at a level engineers
can review and implement against. It assumes the PRD has settled the **what /
who / why**.

Focus on architecture and approach, not line-by-line implementation. Reference
`prd.md` for motivation and requirements rather than restating them.

## Input

Before writing, read whichever of these exist:

- `$PLAN_DIR/{slug}/prd.md` — the product contract. Treat it as authoritative;
  do not relitigate scope or success criteria here.
- `$PLAN_DIR/{slug}/design_interview.md` — the design alignment session's Q&A
  and proposed glossary changes. Treat answers as authoritative; do not
  relitigate them.
- `$GLOSSARY_DIR/GLOSSARY.md` (and relevant sub-glossaries under
  `$GLOSSARY_DIR`) — the project's existing ubiquitous language. Use these terms
  verbatim in the design doc.

## Steps

1. Confirm with the user: component boundaries, key interfaces, the testing
   strategy, and the rollout plan. If any of these are unresolved or contested,
   send them back through `design-interview` — do not invent answers in the
   design doc.

2. Copy [template.md](./template.md) to `$PLAN_DIR/{slug}/design.md` and fill
   out each section.

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
- Prefer a few deep modules over many shallow ones, even when each shallow
  module looks clean in isolation. Shallowness lives in the seams _between_
  modules, not just within one module — a system of small narrow modules can be
  shallow as a whole because callers must know all of them and how to compose
  them.
- Delete pass-through layers, thin wrappers, and adapters that only forward
  calls or rename fields. Merge into caller or callee.
- Hide parameters with obvious defaults; expose overrides only where needed.
- If two modules must share many internal details to function, the boundary is
  wrong — redraw it.
- Information leakage: if a change inside module A regularly forces a change in
  module B, A's interface is exposing internals it shouldn't.
- One implementation = hypothetical seam. Two implementations = real seam. Don't
  introduce an interface (mock-able boundary, abstract base, plugin point)
  unless something actually varies across it. Speculative seams add interface
  cost without paying it back.

Deletion test: imagine deleting the module. If complexity vanishes, it was a
pass-through — delete it. If complexity reappears across N callers, the module
was earning its keep.

The interface is the test surface. Callers and tests cross the same seam. If the
testing strategy needs to reach _past_ the interface to verify behavior, the
module is the wrong shape — redesign before writing tests around it.

Example — a `RateLimiter`:

Shallow (avoid) — four small modules, each with a clean narrow interface:

```
Clock
  now() -> Instant
TokenBucket(capacity, refillRate)
  acquire(tokens, now) -> bool
  remaining(now) -> int
KeyedBucketStore
  get(key) -> TokenBucket
  put(key, bucket)
RetryAfterCalculator
  compute(bucket, tokens, now) -> Duration
```

Each module is small, easy to unit-test, and looks fine in isolation. The
shallowness is in the seams: to rate-limit a request, the caller must fetch the
bucket, thread the clock, attempt acquire, and compute retry-after on denial.
Apply the deletion test to `RetryAfterCalculator` — complexity just moves to the
caller. Switching algorithms (sliding window) breaks the seams every caller
depends on.

Deep (prefer) — collapses all four seams:

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
