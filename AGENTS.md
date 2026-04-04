Documentation Guidelines
========================

Document _all_ non-trivial items
--------------------------------

**Non-trivial item heuristics:**

- Purpose or behavior not immediately obvious
- Has special constraints
- Contains domain-specific logic
- Non-obvious error conditions
- Wraps external behavior
- Top-level crate documentation (`src/lib.rs`, `src/main.rs`)

**Non-trivial item example:**

```rust
/// Extracts the token from an HTTP Authorization header.
///
/// `auth_header` is expected to have the exact format "Bearer <token>"
/// (case-sensitive, single space).
///
/// Returns `None` if the prefix is missing, misspelled, or if multiple 
/// spaces are used.
pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    auth_header.strip_prefix("Bearer ")
}
```

**Trivial item heuristics:**

- Pure wrappers/delegates
- Derived structs with only standard derives
- one-line field accessors
- simple one-to-one constructors

When in doubt, document it. The cost of an unnecessary comment is lower than a
missing one.

**Trivial item example:**

```rust
pub fn is_ascii_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}
```

**Non-trivial module documentation example:**

```rust
//! Logic for processing customer transactions and tax compliance.
//!
//! This module serves as the integration layer between our internal order
//! system and the external Stripe API. It handles:
//! 
//! - **Validation**: Ensuring orders meet regional legal requirements.
//! - **Calculation**: Applying VAT, sales tax, and restocking fees.
//! - **Execution**: Finalizing the movement of funds.
//!
//! # Safety & Precision
//! Because this module handles currency, all calculations must use `Decimal`
//! types to avoid floating-point errors. Never use `f64` for transaction
//! amounts.

pub mod refund;
pub mod invoice;

// ... rest of the code ...
```

A top-level submodule documentation should be added only when it helps clarify
the module's intent. Do not needlessly repeat documentatin already included in
the items under the module.

Explain _all_ non-trivial code blocks
-------------------------------------

```rust
/// Orchestrates a refund through the internal ledger and external gateway.
pub fn process_order_refund(order: &Order, reason: RefundReason) -> Result<()> {
    // Ensure the refund is requested within the 30-day window. We allow
    // exceptions for "Defective" items regardless of the date.
    let is_eligible = order.is_within_return_window() || reason == RefundReason::Defective;
    if !is_eligible {
        return Err(Error::EligibilityExpired);
    }

    // Calculate refund total. We subtract a 10% restocking fee for "Changed
    // Mind" returns to cover warehouse processing costs.
    let mut amount = order.total_price();
    if reason == RefundReason::ChangedMind {
        amount *= 0.90;
    }

    // Trigger the third-party payment API.
    //
    // Note: This must happen AFTER our local validation to avoid 
    // orphaned transactions in our database.
    payment_gateway::refund(order.id, amount);

    Ok(())
}
```

Separate non-trivial code blocks with one empty newline.

Documentation Structure
-----------------------

### Template (omit empty sections)

````rust
/// A single, concise sentence explaining what the item is.
///
/// An in-depth description of behavior and nuances. Can span multiple
/// paragraphs.
///
/// Formatted as Markdown (with Rust-specific extensions). Use square brackets
/// to link to code items: [`StructName`], [`Option`], [`crate::module::Item`].
///
/// Important: Document all conditions under which a function returns an error.
///
/// Panics
/// ------
///
/// Document all conditions under which a function panics under a `Panics`
/// header.
///
///
/// When relevant, provide at least one code example. By default, code examples
/// are compiled and run as "doctests". Example:
/// ```
/// # setup_code();  // Use `#` to hide code that distracts from the example.
/// let val = foo();
/// assert_eq!(val, "foo");
/// ```
///
/// In doctests, use `?` for idiomatic error handling instead of `unwrap()`:
/// ```
/// use std::io;
/// let mut input = String::new();
/// io::stdin().read_line(&mut input)?;
/// # Ok::<(), io::Error>(())  // Explicity set doctest's error type for `?`.
/// ```
///
/// Use attributes to control doctest behavior:
///
/// ```no_run
/// // Compile but do not execute (useful for I/O).
/// std::fs::remove_file("config.toml")?;
/// ```
///
/// ```ignore
/// // For non-compiling Rust examples. Avoid if possible.
/// some_missing_crate::do_something(); 
/// ```
///
/// Intentional failure examples:
///
/// ```should_panic
/// assert!(false);
/// ```
///
/// ```compile_fail
/// let x = 5;
/// x += 2; // we expect compilation failure here
/// ```
///
/// Non-Rust:
///
/// ```custom,{.language-python}
/// print("hello")
/// ```
///
/// ```text
/// directory/
/// ├── foo.rs
/// └── bar.rs
/// ```
````

Code Style
==========

### Format

Run `bayov-format`.

### Core Rules

- Keep each module/type/function focused on one responsibility or task.
- Avoid shared mutable state across modules.
- Avoid global state.
- Prefer helpers that return meaningful intermediate values.
- Use `Options` struct or _Builder Pattern_ to group long parameter lists

### Compile-time correctness

Leverage the type system to make illegal states unrepresentable:

```rust
// Bad: Could accidentally have `is_verified: true` with `email: None`.
// Bad: Email could be an invalid string.
struct User { email: Option<String>, is_verified: bool }

// Good: Verified user must have email.
// Good: Email will always be valid.
struct Email(String);
enum User { Unverified, Verified(Email) }
```

### Separate logic from side effects

```rust
// Bad: Logic + I/O
fn process(id: u64, amt: f64) { let p = amt * 0.9; db.save(id, p); }

// Good
fn calc(amt: f64) -> f64 { amt * 0.9 }
fn save(r: &impl Repo, id: &Id, p: f64) { /* ... */ }
```

### Single Level of Abstraction (SLAP)

- Each function should operation at one conceptual level.
- High-level functions should read as a sequence of named steps.
- Lower-level mechanics should be moved into helpers.

### Parameter lists

Introduce a type for parameters that always travel together:

```rust
// Bad
fn record_transaction(amount: Decimal, currency: Currency, account: Account);
fn calculate_tax(amount: Decimal, currency: Currency, rate: f64) -> Decimal;

// Good
struct CurrencyAmount { amount: Amount, currency: Currency }
fn record_transaction(amount: CurrencyAmount, account: Account);
fn calculate_tax(amount: CurrencyAmount, rate: f64) -> Decimal;
```

Testing Guidelines
==================

File Locations
--------------

- Unit-tests for `foo.rs` should be named `foo.tests.rs` (same directory) and
  included as such:
  ```rust
  # place at the **VERY END** of foo.rs
  #[cfg(test)]
  #[path = "foo.tests.rs"]
  mod tests;
  ```
- Unit-tests for `foo/main.rs` should be named `foo/tests.rs` (same directory).
- Crate/E2E tests should be placed under `{crate}/tests/`.
- SvelteKit route tests should be named `page.test.ts` inside the route
  directory (e.g. `webapp/src/routes/{path}/page.test.ts`).

Human-Readable Structure (Given/When/Then)
------------------------------------------

Structure test bodies to be read as plain English. Use comments to separate
phases.

When relevant, use:

- **Given (Setup):** Establish the initial state or context.
- **When (Action):** Perform the specific operation being tested.
- **Then (Outcome):** Assert on the observable result.

**Example:**

```rust
#[test]
fn test_serialize_transaction() {
    // Given: A transaction with a payee and one posting
    let mut t = Transaction::new();
    t.payee = Some("Google".to_string());
    t.postings.push(Posting::new("Assets:Bank", 100, "ILS"));

    // When: We serialize it to Beancount format
    let output = serialize(vec![Item::new_transaction(date, t)]);

    // Then: The output matches the expected ledger entry
    assert!(output.contains("2025-01-01 * \"Google\""));
    assert!(output.contains("Assets:Bank  100.00 ILS"));
}
```

Testing Mandates
----------------

- **Declarative Intent:** Focus on _what_ is being tested (business logic), not
  _how_ (implementation details). Hide technical complexity behind descriptive
  helper functions.
- **Concrete Examples:** Use real-world data (specific amounts, dates, names)
  instead of abstract placeholders.
- **When to Test:** Always add tests for new features and bug fixes. Not every
  file requires a test (e.g., glue code, simple types).
- **Run Suite:** Run the full test suite with `uv run dev/test.py`
  - The suite also runs `bean-check` which validates the `ledger/` directory.
- **Bug Fixes:** Reproduce with a test BEFORE fixing.
- **Determinism:** Fixed dates only. No system time, no network.

Error Handling Guidelines
=========================

`Error` and `Result` types
--------------------------

A single `Error` and `Result` types defined in `result.rs` are typically enough:

```rust
#[derive(Error, Debug)]
#[non_exhaustive]  // prevents breaking changes when adding new variants
pub enum Error {
    #[error("failed to read configuration from {0}")]
    ConfigNotFound(String),

    #[error("invalid header: {expected} (found {found})")]
    InvalidHeader {
        expected: String,
        found: String,
    },

    #[error("internal database error")]
    Disconnect(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

Error Handling Mandates
-----------------------

- Avoid `unwrap()` and `expect()`. Propagate errors whenever possible.

Logging Guidelines
==================

Log levels
----------

- `error!` - Very serious errors (e.g., connection lost).
- `warn!` - Less serious issues (e.g., retrying a request).
- `info!` - Useful information (e.g., "server started", "configuration loaded").
  - AVOID noisy info logs (e.g., inside a loop with many iterations).
- `debug!` - Lower priority information (e.g., "parsing headers").
- `trace!` - Very low priority information, often extremely verbose (e.g.,
  low-level state transitions).

Tracing spans
-------------

Use spans whenever performing a _unit of work_ that has a logical start and end.

### `tracing::instrument`

Most common type of span: From start to end of decorated function.

```rust
use tracing::instrument;
use tracing::info;

#[instrument(skip(data), fields(data_len = data.len()))]
pub async fn process_upload(id: u64, data: &[u8]) -> Result<()> {
    info!("starting upload processing");
    // ... logic
    Ok(())
}
```

- Every significant function should be decorated with `#[instrument]`.
- Use `skip(data)` to prevent noise / overhead for complex parameters.
- Use `fields(...)` to manually add relevant metadata.

### `{log_level}_span!`

For spans that don't map 1:1 to a function:

```rust
use tracing::info_span;
use tracing::info;

let _custom_work_span = info_span!("custom_work", task_id = 42).entered();

// Work performed here is inside the span
info!("this event is contextualized");

// When `_custom_work_span` is dropped at the end of the scope, the span exits.
```

### **NEVER** enter span across `.await` points

This will corrupt traces. Instead, use `.instrument()`:

```rust
use tracing::info_span;
use tracing::Instrument as _;

async fn my_async_fn() {
    // Never keep entered `_span` across `.await` -- This is a critical bug.
    // let _span = tracing::info_span!("async_work").entered();

    // Instead, define unentered `span` and `.instrument(span)` the `Future`
    // before `.await`ing it.
    let span = tracing::info_span!("async_work");    
    do_something_async()
        .instrument(span)
        .await;
}
```

Structured logging
------------------

**NEVER** put dynamic data inside the message string. Use fields instead. This
allows log aggregators (like Datadog or ELK) to filter and graph the data.

- Bad: `info!("User {} logged in from {}", user_id, ip);`
- Good: `info!(user.id = ?used_id, user.ip = %ip, "user login successful");`
  - `%` uses the `Display` trait.
  - `?` uses the `Debug` trait.

Logging Mandates
----------------

- When returning an error, avoid logging `error!` or `warn!`. Let the _caller_
  decide when to log.
