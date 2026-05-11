---
axis: observability
---

# Observability review

Logging, metrics, traces, and error context for new and changed code. The goal:
when this code misbehaves in production, an on-call engineer can tell _what
happened_ and _why_ from telemetry alone.

## What to look for

### Logs

- **Decision points logged at the right level**: new branches that change
  business behavior emit info logs with the inputs that drove the decision
  (sanitized — see `security.md` for PII rules).
- **Errors logged with context**: every `except` / `catch` / `if err != nil`
  that ends in "swallow and continue" or "convert to a different error" logs the
  original cause with structured fields. `logger.exception(...)` / `errors.Wrap`
  / `fmt.Errorf("...: %w", err)`.
- **Log levels appropriate**: debug for high-volume per-iteration noise; info
  for normal lifecycle events; warn for recoverable anomalies; error for
  unexpected failures. New code shouldn't fill error logs with normal
  conditions.
- **Structured fields, not string concatenation**: prefer
  `logger.info("user signed up", user_id=..., plan=...)` over
  `logger.info(f"user {id} signed up to {plan}")`. Easier to query in log
  aggregators.
- **No PII / secrets / tokens in logs** (cross-checked by `security.md`, but
  flag here too).
- **No debug-leftover logs**: `print`, `console.log`, `fmt.Println` not removed;
  "got here", "asdf", "TODO remove" markers.

### Metrics

- **New flows have counters / histograms** for the operations that matter:
  request count, error count, duration. Especially for any new external call.
- **Error counters are dimensioned**: failure metric tagged by reason (timeout /
  4xx / 5xx / parse-error), not just a flat counter.
- **Histograms not gauges** for latency / size distributions; gauges for
  point-in-time levels (queue depth, connection count).
- **No high-cardinality tags**: don't tag metrics by user ID, request ID,
  free-form strings — blows up storage and query cost. Tag by enum-like
  categories (status code, region, feature flag value).
- **No new counters with no callers caring**: if a metric isn't on a dashboard
  or alert, that's a soft finding (note as minor).

### Tracing

- **Spans on new external calls** (HTTP, DB, RPC, queue) if the project uses
  distributed tracing. Span name = operation, attributes = parameters that
  matter for debugging.
- **Span context propagates** across the boundaries the project supports. Don't
  drop context when launching a goroutine / task.
- **Errors recorded on spans** (`span.record_exception` /
  `span.SetStatus(codes.Error, ...)`), not only logged.

### Error context

- **Wrap, don't replace**: when converting low-level errors to domain-meaningful
  ones, preserve the cause. Python `raise ... from e`, Go
  `fmt.Errorf("...: %w", err)`, JS `new Error("...", { cause: err })`.
- **Re-raised exceptions keep the stack**: don't `raise` with a new exception
  object that discards the original traceback.

### Health and lifecycle

- New long-running components (workers, schedulers, daemons) emit start/stop
  logs and expose a health endpoint or readiness signal if the service has that
  pattern.

## What this axis does NOT cover

- Whether the metric / log will trip an alert (operational concern, not
  code-review).
- PII redaction enforcement (`security.md`).

## Severity calibration

- **Critical**: a new external call with no error logging, no metric, and no
  trace span (silent failure in production); error swallowed without any signal.
- **Major**: error logged as a string without structured fields or wrapped
  cause; high-cardinality metric tag; missing duration histogram on a new
  endpoint.
- **Minor**: log level slightly off; log message could include one more useful
  field; leftover `print` / `console.log`.

## Output

Per the dispatcher's schema. `recommendation` names the specific log/metric/
span to add and the fields to include.
