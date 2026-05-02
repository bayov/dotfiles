A concise description of this vertical slice. Describe the end-to-end behavior,
not layer-by-layer implementation. Use GLOSSARY.md terms verbatim.

## References

List whichever of `plan/{slug}/`'s `prd.md`, `design.md`, `plan.md` this slice
draws from. Cite the user stories, components, decisions, or acceptance criteria
the slice advances.

Example:

- plan/{slug}/prd.md (User Stories: S1, S2)
- plan/{slug}/design.md (Components A, B; Decision D2)
- plan/{slug}/plan.md (Component A; Decision D1)

## Blocked by

- plan/{slug}/tasks/{task_name}.md

> Omit the "Blocked by" section if there are no blockers.

## Acceptance criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
- [ ] Glossary / docs owned by this slice are updated (list each: e.g.
      `GLOSSARY.md` term **X**, `design.md` cross-ref, README example).

> Drop the docs criterion if the slice introduces no domain terms and touches no
> doc surface.

## Out of scope

- Behavior a reader might reasonably expect in this slice but that lives in
  another slice or is deferred. Make the boundary unambiguous.

> Omit the "Out of scope" section if there is nothing worth calling out.
