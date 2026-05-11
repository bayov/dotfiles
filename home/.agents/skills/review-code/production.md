---
axis: production
---

# Production review

Bugs and production hazards that would manifest under real load, real users, or
real deployment. Covers traditional "bugs" plus API/contract compatibility and
migration safety.

## What to look for

### Correctness hazards

- **Null / undefined / None paths**: every return that could be empty is handled
  by every caller. Optional values are unwrapped only after a check.
- **Off-by-one**: loop bounds, slice indices, range queries.
- **Concurrency**: shared state mutated without a lock; read-modify-write
  sequences that aren't atomic; double-checked locking that isn't safe in the
  language's memory model.
- **Resource lifecycle**: anything opened/acquired must be closed/released on
  every path including errors and early returns. Files, sockets, DB sessions,
  locks, contexts, observers, subscriptions.
- **Time and timezones**: naive `datetime` vs UTC, system-local time used where
  it should be UTC, leap-second / DST edge cases on date math.

### Failure modes

- **Timeouts**: every blocking external call (HTTP, DB, RPC, queue) has an
  explicit timeout. Default-infinity is a bug.
- **Retries**: retried operations are idempotent or guarded by an idempotency
  key. Retry storms are bounded (max attempts, backoff with jitter).
- **Partial failure**: batch operations (writes to N targets) define and
  document what "partial success" means. Silent partial writes are bugs.
- **Cancellation**: long-running operations honor cancellation tokens (`ctx`,
  `AbortSignal`, etc.). Cancelled work cleans up.
- **Error propagation**: errors aren't swallowed; they either bubble to a caller
  or are explicitly logged with full context and converted to a
  domain-meaningful result.

### Input handling

- Bounds and length checks on user-supplied data before allocation/loops.
- Numeric parsing handles overflow, NaN, negative when not expected.
- Trust boundaries respected — data crossing from untrusted to trusted is
  validated, normalized, and re-encoded.

### API and contract compatibility

- **Public interface changes** — function signatures, REST/GraphQL/gRPC schemas,
  event payloads, message formats. Any change that breaks existing callers is
  flagged unless versioned or feature-flagged.
- **Schema migrations** — backward-compatible (additive) ideally; if not, the
  diff includes the migration + rollback strategy.
- **Generated clients** — when protos / OpenAPI / GraphQL schemas change, the
  corresponding client regen artifact is part of the same diff (per the repo's
  convention, e.g. `just generate-api-clients`).
- **Wire format**: new fields don't repurpose old IDs; enum values aren't
  reordered.

### Data and state

- Migrations that lock big tables; missing concurrent index creation.
- Default values that change the meaning of historical rows.
- `NOT NULL` added to a column with existing NULLs and no backfill.
- Code that writes new format while readers expect old format (no read fence).

## Severity calibration

- **Critical**: data loss / corruption / silent partial write; unclosed resource
  on the error path of a hot code path; breaking change to a public API or wire
  format; migration that will lock production tables.
- **Major**: missing timeout on external call; non-idempotent retry; swallowed
  error losing context; off-by-one in a range query; missing client regen for a
  schema change.
- **Minor**: defensive check missing on a low-risk path; suboptimal but not
  buggy concurrency primitive.

## Output

Per the dispatcher's schema. Be specific in `recommendation`: name the file and
line, the missing check or wrapper, the migration step that needs to be split,
the timeout value to add.
