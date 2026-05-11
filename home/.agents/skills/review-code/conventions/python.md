---
axis: conventions/python
---

# Python conventions review

Python-specific idiom and style review. Runs alongside the cross-language
`conventions.md` axis when the diff touches `.py` files. This axis handles
language-level concerns; the parent axis handles architecture/naming.

## What to look for

### Typing

- Public functions/methods have type hints on parameters and return.
- `Any` used where a real type would fit. Prefer `Protocol`, `TypedDict`,
  `Literal`, or domain types over `Any` or `dict`.
- `Optional[X]` vs `X | None` consistent with the rest of the file.
- Generic types parameterized (`list[str]`, `dict[str, int]`), not bare (`list`,
  `dict`).

### Idioms

- Comprehensions vs explicit loops: prefer comprehensions for filter/map, loops
  when there's real branching or side effects.
- `with` statements for any resource that has a close/cleanup (files, locks, DB
  sessions, HTTP clients).
- `pathlib.Path` over `os.path` for new path manipulation.
- f-strings over `%` or `.format()` (except in logging — see below).
- `enumerate` over manual index counters; `zip` over parallel indexing.

### Errors

- Exceptions are domain-meaningful subclasses, not bare `Exception` or
  `RuntimeError` from production code paths.
- `except` clauses catch specific types, not bare `except:` or
  `except
  Exception:`.
- `raise ... from e` is used when re-raising to preserve the cause chain.

### Logging

- Logging uses `%s`-style placeholders, not f-strings — preserves the message
  template for log aggregators and avoids eager string formatting on
  filtered-out levels.
- `logger.exception(...)` (or `exc_info=True`) inside `except` blocks rather
  than logging `str(e)` alone.

### Data classes and models

- `@dataclass` or Pydantic models for record-shaped data, not bare dicts or
  tuples-of-fields.
- Consistent choice across the diff (don't mix dataclass and Pydantic for
  similar concerns).
- `frozen=True` for value objects that should not mutate.

### Async

- `async def` only when there's actual `await` inside (no
  async-for-its-own-sake).
- No mixing of sync blocking calls (`time.sleep`, sync HTTP libs) inside async
  code paths.
- `asyncio.gather` rather than serial `await` for independent coroutines.

### Imports

- No `from x import *`.
- Standard library, third-party, local — three groups, blank line between.
- No relative imports across package roots; relative imports within the same
  package are OK if the repo uses them.

### Things to avoid

- Mutable default arguments (`def f(x=[])`).
- `assert` for runtime validation in production code (asserts are stripped under
  `python -O`); use exceptions.
- Catching and silently passing on exceptions.

## Severity calibration

- **Critical**: silent exception swallowing in production paths; mutable default
  args that will cause cross-call state leakage; `assert` used for runtime
  validation of user input.
- **Major**: missing type hints on public APIs; `Any` masking a real type;
  bare-`Exception` catch; async function with blocking sync call inside.
- **Minor**: f-string in logging, comprehension preferable, `os.path` instead of
  `pathlib`, import ordering.

## Output

Per the dispatcher's schema.
