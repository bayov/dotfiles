---
axis: conventions/go
---

# Go conventions review

Go-specific idiom and style review. Runs alongside the cross-language
`conventions.md` axis when the diff touches `.go` files.

## What to look for

### Errors

- Errors are returned, not panicked, except in truly unrecoverable startup
  paths.
- Each error returned is either wrapped with `fmt.Errorf("...: %w", err)` for
  context or passed through unchanged — never reformatted with `%v` (which loses
  the wrap chain).
- `errors.Is` / `errors.As` used to inspect wrapped errors, not string
  comparison.
- Sentinel errors are exported `var Err... = errors.New(...)`.
- Errors are checked immediately after the call that produces them, on the same
  indent level. No skipping `err`.

### Context

- Any function performing I/O, RPC, or potentially-long work takes
  `ctx context.Context` as its first parameter.
- `ctx` propagates through call chains; no `context.Background()` or
  `context.TODO()` mid-stack (only at the top of a request or a goroutine spawn
  that legitimately starts fresh).
- `ctx` cancellation is respected — long loops check `ctx.Done()`, blocking
  calls accept context.

### Goroutines and channels

- Every goroutine has a clear termination path; no orphan goroutines.
- Goroutines spawned with `go func()` capture `ctx` (or a derived one).
- Channels are owned by exactly one sender. The sender closes; receivers do not
  close.
- `sync.WaitGroup`, `errgroup.Group`, or explicit `done` channels coordinate
  concurrent work — no ad-hoc `time.Sleep` to "wait for goroutines."

### Interfaces

- Interfaces are defined at the consumer side, not the producer side ("accept
  interfaces, return structs"). Small interfaces preferred.
- No empty interfaces (`interface{}` / `any`) when a real interface would fit.
- Interface methods take and return concrete types unless polymorphism is the
  point.

### Resource handling

- `defer` for every Close/Unlock/Done immediately after acquisition.
- `defer` in a loop only when intentional; otherwise extract a helper or call
  cleanup explicitly.
- HTTP response bodies are always closed: `defer resp.Body.Close()` after a nil
  error check.

### Naming

- `MixedCaps` (exported) / `mixedCaps` (unexported). Acronyms keep case (`URL`,
  `ID`, `HTTP`).
- Interface names ending in `-er` for single-method interfaces (`Reader`,
  `Closer`).
- Receiver names are short and consistent across methods of the same type.
- No Hungarian prefixes (`iCount`, `sName`).

### Idioms

- Zero values are usable where it costs nothing extra.
- `:=` declares new variables; `=` reassigns. Mixing in the same statement is
  fine when at least one variable is new.
- `if err := f(); err != nil` pattern for scoped error handling.
- No `else` after a returning `if`.

### Things to avoid

- `panic` outside of true invariants (nil-deref-on-startup, programmer errors).
  Library code should not panic.
- Naked returns in long functions (more than ~10 lines).
- Bare `go func()` without a sync mechanism to track completion.
- `init()` functions doing real work (registering globals OK; I/O not OK).

## Severity calibration

- **Critical**: orphan goroutine with no termination path; unchecked error
  return; resource not closed (file/conn/body) on any code path.
- **Major**: error reformatted with `%v` losing wrap chain; missing `ctx`
  parameter on an I/O function; panic in library code; channel ownership
  unclear.
- **Minor**: receiver naming inconsistency, `else`-after-return, naked return in
  moderate-length function.

## Output

Per the dispatcher's schema.
