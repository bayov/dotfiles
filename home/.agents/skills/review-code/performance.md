---
axis: performance
---

# Performance review

Performance hazards visible from a static read of the diff. Not a substitute for
profiling — flag patterns that are reliably costly, not speculative
micro-optimizations.

## What to look for

### Database access

- **N+1 queries**: a loop over results from one query that issues a query per
  iteration. Look for `for ... in items: ... .get/.find/.query(...)` shapes. The
  fix is a join or an `IN (...)` batch.
- **Missing index hints**: queries on new tables/columns that filter or sort on
  unindexed fields. Cross-check the migration in the same diff.
- **Unbounded result sets**: queries with no `LIMIT` / pagination on user-facing
  endpoints. Especially `SELECT * FROM big_table WHERE ...` where the result
  feeds a JSON response.
- **`SELECT *` from wide tables when only a few columns are needed**.
- **Transactions held across slow operations**: a DB transaction wrapping an
  HTTP call, a long compute, or a user-interactive step. Locks held too long.

### Network and I/O

- **Serial when parallel would work**: independent awaitable calls done in a
  loop with `await` each iteration. `Promise.all` / `asyncio.gather` /
  `errgroup.Wait`.
- **Sync I/O on a hot async path**: blocking call inside async/event-loop code
  (e.g. `time.sleep` in `async def`, `requests` instead of `aiohttp`,
  synchronous file read inside Node.js request handler).
- **Per-request resource creation** that should be reused: new HTTP client / DB
  connection / large config object built on every request rather than once at
  startup.
- **Missing timeouts** on outgoing calls: a perf hazard as well as a production
  one — slow upstreams become slow us.

### Memory and allocation

- **Loading entire collections into memory** when streaming would work (read
  whole file into a string, fetch all rows then filter).
- **Quadratic algorithms** on user-influenced input sizes: nested loops over the
  same collection, repeated linear lookups inside loops that could use a
  set/map.
- **String building in a tight loop** with `+=` instead of join/buffer.
- **Caching pitfalls**: unbounded in-process caches with no eviction policy;
  caches keyed on objects whose hash changes; cache TTLs that are absent rather
  than chosen.

### Concurrency

- **Mutexes held across slow operations** (I/O, RPC, sleep). Hot lock.
- **Goroutine / task fan-out without a concurrency limit**: unbounded
  parallelism that will overload downstreams.
- **Busy-wait loops** (`while not done: pass`, `for !done {}` with no
  sleep/cond-var).

### Frontend / rendering

- **Heavy work in render path**: large list rendered without virtualization;
  expensive computation on every render with no memo.
- **Synchronous layout thrash**: read-write-read-write in a loop touching
  layout-affecting properties.
- **Bundle bloat**: new large dep imported at the top level when a code-split /
  dynamic import would fit (cross-check `dependencies` axis).

## What this axis does NOT cover

- Whether the code is correct (`production.md`).
- Whether there is enough observability to measure performance after deploy
  (`observability.md`).

## Severity calibration

- **Critical**: N+1 query on a user-visible endpoint; unbounded result set
  feeding a response; sync blocking call on an async hot path; lock held across
  an external call.
- **Major**: missing index on a new filter column; serial-when-parallel calls;
  per-request creation of expensive shared resource; unbounded cache.
- **Minor**: micro-optimization opportunity; string concat in a small loop;
  `SELECT *` in a low-traffic admin path.

## Output

Per the dispatcher's schema. `recommendation` proposes the specific change:
"replace the loop at file:line with a single `IN (...)` query"; "wrap the calls
in `asyncio.gather`"; "move HTTP client creation to a module-level singleton."
