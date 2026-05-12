> **Writing rules:** Bullets only outside the Problem paragraph. ≤15 words per
> bullet. Omit optional sections entirely — do not include empty headings.

## Problem & Solution

**Problem:** the situation and pain, from the user's perspective. One short
paragraph (≤60 words).

**Target user:** who this is for. One sentence.

**Solution:** what changes for the user — bullets, outcome only, no
implementation detail.

## Acceptance Criteria

The checklist `implement` works against. Each criterion is observable through
the public interface — outcome, not implementation.

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Glossary / docs owned by this change are updated (list each: e.g.
      `$GLOSSARY_DIR/GLOSSARY.md` term **X**, README example).

> Drop the docs criterion if the change introduces no domain terms and touches
> no doc surface.

## Scope

**In scope:** bullets of capabilities this plan commits to.

**Out of scope:** bullets a reader might reasonably expect but that aren't
included.

## Notes

Optional. Add a bullet here only if it would surprise a reviewer or change the
AC. Candidates: a non-obvious decision, a risky failure mode, a migration
ordering constraint, an open question with an owner. Skip otherwise — do not
pad.

> Omit this section if there is nothing load-bearing to record.

## Glossary Changes

Proposed additions, updates, and removals to `$GLOSSARY_DIR/GLOSSARY.md` that
this plan depends on. Carry forward changes from `plan_interview.md` plus any
new terms surfaced while drafting.

Only include terms meaningful to domain experts. Internal technical names stay
in this doc.

### New term: **Term**

A concise definition of the new term.

### Updated term: **Term**

The proposed change to the existing definition.

### Removed term: **Term**

Reason for removing the term.

> Omit this section if there are no glossary changes.
