---
linear: not synced - run /linear-sync
---

<!-- Writing rules: Whole doc ≤150 words. Bullets only. ≤15 words per bullet.
Omit optional sections entirely — do not leave empty headings.
Leave the `linear` frontmatter as-is — `/linear-sync` fills it in. -->

A concise description of this vertical slice. Describe the end-to-end behavior,
not layer-by-layer implementation. Use `$GLOSSARY_DIR/GLOSSARY.md` terms
verbatim.

## References

List whichever of `plan/{slug}/`'s `prd.md`, `design.md`, `plan.md` this slice
draws from. Cite the user stories, components, decisions, or acceptance criteria
the slice advances.

Example:

- plan/{slug}/prd.md (User Stories: S1, S2)
- plan/{slug}/design.md (Components A, B; Decision D2)
- plan/{slug}/plan.md (Component A; Decision D1)

## Blocked by

- plan/{slug}/tasks/{task-name}.md

> Omit the "Blocked by" section if there are no blockers.

## Acceptance criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
- [ ] Glossary / docs owned by this slice are updated (list each: e.g.
      `$GLOSSARY_DIR/GLOSSARY.md` term **X**, `design.md` cross-ref, README
      example).

> Drop the docs criterion if the slice introduces no domain terms and touches no
> doc surface.

## Rollout

<!-- Subtasks that track each attached PR through to merge, plus post-merge
verification when it matters. `decompose` seeds this; `implement` fills in PR
links and ticks items. Every PR must at least be merged. Add verification
subtasks only when the change is important enough to confirm after merge (prod
behavior, logs/metrics, feature-flag flip) - omit them otherwise. -->

- [ ] PR opened, reviewed, and merged (`owner/repo#NN` once opened)
- [ ] Verify in prod: <behavior / logs / metrics / flag to check>

> Keep the merge subtask always. Keep, edit, or drop the verification subtask(s)
> based on the slice's importance.

## Out of scope

- Behavior a reader might reasonably expect in this slice but that lives in
  another slice or is deferred. Make the boundary unambiguous.

> Omit the "Out of scope" section if there is nothing worth calling out.
