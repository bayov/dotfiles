---
name: update-glossary
description: Update the project's GLOSSARY.md
disable-model-invocation: true
model: opus
effort: medium
---

Update `$GLOSSARY_DIR/GLOSSARY.md` to reflect new, changed, or removed terms —
typically from an alignment session (`ALIGN.md`) or a discussion that surfaced
terminology issues.

> **Glossary root:** `$GLOSSARY_DIR` env var (defaults to the project root if
> unset). All glossary paths below resolve relative to it.

## Process

1. Locate the relevant `GLOSSARY.md` under `$GLOSSARY_DIR`. In multi-glossary
   repos, pick the one that owns the term's domain; create a new sub-glossary
   only if no existing one fits.
2. Read it before editing so you preserve tone and existing structure.
3. Apply each proposed change (add / update / remove). Definitions belong only
   if they're meaningful to a domain expert — skip implementation details and
   general programming concepts.
4. Update the `Relationships` and `Flagged ambiguities` sections if the change
   affects them.
5. If the change introduces a synonym for an existing term, list it under
   `_Avoid_` rather than creating a duplicate entry.

## Format

```md
# {Glossary Name}

{One or two sentence description of what this glossary is and why it exists.}

## Language

**Order**: A request from a customer to purchase goods. _Avoid_: Purchase,
transaction

**Invoice**: A request for payment sent to a customer after delivery. _Avoid_:
Bill, payment request

**Customer**: A person or organization that places orders. _Avoid_: Client,
buyer, account

## Relationships

- An **Order** produces one or more **Invoices**
- An **Invoice** belongs to exactly one **Customer**

## Example dialogue

> **Dev:** "When a **Customer** places an **Order**, do we create the
> **Invoice** immediately?" **Domain expert:** "No — an **Invoice** is only
> generated once a **Fulfillment** is confirmed."

## Flagged ambiguities

- "account" was used to mean both **Customer** and **User** — resolved: these
  are distinct concepts.
```

Group terms under subheadings when natural clusters emerge; otherwise a flat
list is fine.

## Rules

- **Be opinionated.** When multiple words exist for the same concept, pick one
  and list the rest under `_Avoid_`. Ambiguity in the glossary defeats its
  purpose.
- **Keep definitions tight.** One sentence. Define what a term IS, not what it
  does.
- **Flag conflicts explicitly.** If a term has been used ambiguously, record the
  resolution under `Flagged ambiguities` so the history isn't lost.
- **Show relationships.** Use bold term names and express cardinality where
  obvious — relationships catch contradictions definitions alone won't.
- **Write an example dialogue.** A short exchange between a dev and a domain
  expert demonstrates how terms interact and clarifies boundaries better than
  prose definitions can.
- **Exclude general programming concepts.** Timeouts, error types, utility
  patterns don't belong even if the project uses them heavily. Before adding,
  ask: is this concept unique to this domain, or general?

## Multi-glossary repos

If the repo has more than one glossary, `$GLOSSARY_DIR/GLOSSARY.md` is an index
rather than a term list. Sub-glossaries live in subdirectories under
`$GLOSSARY_DIR`:

```md
# Glossaries

- [Ordering](./ordering/GLOSSARY.md) — receives and tracks customer orders
- [Billing](./billing/GLOSSARY.md) — generates invoices and processes payments
- [Fulfillment](./fulfillment/GLOSSARY.md) — manages warehouse picking and
  shipping

## Relationships

- **Ordering → Fulfillment**: Ordering emits `OrderPlaced` events; Fulfillment
  consumes them to start picking
- **Fulfillment → Billing**: Fulfillment emits `ShipmentDispatched` events;
  Billing consumes them to generate invoices
- **Ordering ↔ Billing**: Shared types for `CustomerId` and `Money`
```

When adding a term, also update the root index if the change affects
cross-glossary relationships.
