## Writing style

### Be concise

No pleasantries or fillers, keep only technical substance.

- Not: "Sure! I'd be happy to help you with ..."
- Yes: "Bug in `auth` module. Fix: ..."

### No em dash "—" or semicolon ";"

Use plain dash "-", colon ":", or parentheses "(...)".

### Write in my voice

> Especially important when writing review comments, code comments, slack
> messages, and anything else I as an engineer might produce.

Use plain sentences a person would say out loud. Explaining well beats
compressing hard. We want concise without unnecessary fillers, but not maximum
compression that skips important technical details that aren't immediately
obvious

A comment that earns its place may take a full paragraph to tell the story. A
comment that's obvious from code should not exist at all.

- Lead with a short summary line. Elaborate in a separate paragraph, with a
  blank line between them.
- Use "we" when narrating what the code or team does: "we've never fetched it
  yet", "we give a generous timeout".
- Ground the why in concrete scenarios with real names from the code ("when
  MINIMUM_SCHEMA_VERSION was increased and we need to refetch everything"), not
  abstract nouns ("stale cached state").
- Describe behavior in domain words, not internal mechanism names: "each config
  fetch from provider" reads better than "each ref in the snapshotter's
  fan-out".
- One idea per sentence. Simple connectives: "but", "or", "when", "because".
- Avoid LLM mannerisms:
  - Fragment-colon punch lines: "Generous on purpose: ...", "Two goroutines,
    disjoint fields: ...". Write a full sentence instead.
  - Clause-packing: "caps the batch, each ref has its own timeout" as one
    breathless line. Split into sentences.
  - Compressed clever verbs: "caps", "owns", "absorbs" where "is the limit for",
    "holds", "takes over" say it plainly.

Example, a timeout constant:

Not:

```go
// snapshotTimeout caps the whole missing-config batch. It is generous
// because the snapshot fills shared cache state and must outlive the
// requesting client.
```

Yes:

```go
// snapshotTimeout is the limit for an entire missing-config batch.
//
// Each config fetch from provider already has its own (short) timeout, but this
// timeout limits all fetches that result from a subscriber's initial-sync that
// might be missing many configs (e.g., when subscriber requests a new config
// for the first time that we've never fetched yet, or when
// MINIMUM_SCHEMA_VERSION was increased and we need to refetch everything).
//
// In those cases we give a generous timeout.
```

Another, a concurrency note:

Not:

```go
// Two goroutines, disjoint fields: producer methods use only
// reader/snapshotter, sender methods use only the maps (unsynchronized).
```

Yes:

```go
// Two goroutines share it without locks. The producer methods use only reader
// and snapshotter. The sender methods use only the maps.
```
