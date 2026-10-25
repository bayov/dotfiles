# Backend

Applies to: `*.py`, `*.go`, `*.java`, `*.rs`, `*.sql`, and service code (APIs,
consumers, crons, workers, migrations).

Status: not calibrated yet. These are starting guesses. Tighten them after the
first backend review.

## Bar

- Lower bar than frontend. More comments are normal.
- Keep a narrow edge case when the failure is data loss, a security hole, a
  race, or a prod incident.
- Keep convention nits that the repo's own guidelines ask for (its `CLAUDE.md`,
  `AGENTS.md` or `.agents/` guides).
- Drop refactor nits that the repo's guidelines don't mention.

## Calibration log

None yet.
