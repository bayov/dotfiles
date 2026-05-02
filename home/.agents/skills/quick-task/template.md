A concise description of this change. Describe the end-to-end user-visible
behavior, not layer-by-layer implementation. Use GLOSSARY.md terms verbatim.

## References

Point to the code and any in-repo docs this task touches. File paths, function
names, or modules — whatever a reader needs to orient.

Example:

- `src/auth/session.ts` — `createSession`, `revokeSession`
- `GLOSSARY.md` — **Session**

> Omit if the change is genuinely self-contained and a reader can find
> everything from the description alone.

## Acceptance criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
- [ ] Glossary / docs owned by this task are updated (list each: e.g.
      `GLOSSARY.md` term **X**, README example).

> Drop the docs criterion if the task introduces no domain terms and touches no
> doc surface.

## Testing notes

The riskiest behavior and how it's pinned down: test level (unit / integration /
end-to-end) and roughly what the test asserts. Anything that can't be tested
automatically and how it's verified instead.

> Omit if the acceptance criteria themselves make the testing approach obvious.

## Out of scope

- Behavior a reader might reasonably expect here but that's deferred. Make the
  boundary unambiguous.

> Omit if there is nothing worth calling out.
