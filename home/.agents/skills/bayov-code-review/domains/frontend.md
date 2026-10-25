# Frontend

Applies to: `*.ts`, `*.tsx`, `*.js`, `*.jsx`, `*.css`, `*.scss`, and anything
under a web app or frontend package (`web/`, `services/frontend/`).

Status: calibrated on 1 PR.

## Bar

- High bar. A handful of comments even on a 30-file PR.
- Raise what the user sees: wrong or awkward text, the same text shown twice, a
  wrong plural, two words for the same action, a control disabled or enabled at
  the wrong time.
- Drop refactor nits with no behavior change: a redundant field, re-export
  shims, import paths, file layout.
- Drop UI edge cases that need several unlikely steps in a row.
- Drop asks to change a flow the author designed, like adding a confirm dialog.

## Calibration log

### cato-networks/server-services#19445 (2026-10-25)

Custom detector UI, 28 files. Drafted 9 comments. I kept 5 (all rephrased) and
approved.

Kept:

- An EDM table component renamed to a generated API type name (asked as a
  question).
- Delete greyed out during each 10s usage poll (`isFetching` vs `isLoading`).
- "used by 1 Engine Profiles".
- A group explanation that repeats its title on screen.
- "Mark" and "Flag" used for the same action.

Dropped:

- A confirm before "Refresh conditions" wipes saved conditions. The author chose
  replace-all on purpose.
- An open add form that survives a refresh and loses its condition when the
  refresh fails. It needs three unlikely steps, and the author already called a
  similar case out of scope.
- `preventionTitle` now has the same text as `label`. A refactor nit.
- Re-export shims left "so callers are unaffected". A refactor nit.
- A component test that should fail on a renamed label. CI shows it.
- The PR description doesn't mention half the changes. Process, not code.
