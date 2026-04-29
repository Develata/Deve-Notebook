# Git Mirror Executable Repair UI Decision - 2026-04-29

## Decision

Current implementation must stop at read-only Web repair review plus CLI-only writer.

The executable Web repair UI is deferred to future. The existing
`GET /api/sc/git-mirror/repair-review` endpoint remains a protected read-only
record-level data source and must not be expanded into a writer.

## Rationale

- `.notegit` / ledger source-control state remains the authority; `.git` is only
  the ecosystem mirror.
- Git repair is a write path with high authority risk: dirty Git worktree,
  stale scope nonce, unbound repo, remote/spectator scope, writer readiness, and
  `.notegit` tracking leak all need fail-closed gates.
- The current Web UI already provides the useful minimum: multi-record
  out-of-sync review, action/subject/next-step explanation, retry command text,
  and loading/error/empty fallbacks.
- Adding a Web Git writer now would increase authorization surface before it is
  required for the next acceptance step.

## Current Boundary

- Web may read repair review records and render CLI-only guidance.
- Web may not run Git import, Git push, Git repair, `git add`, `git commit`, or
  background mirror replay.
- CLI remains the only implemented Git writer surface for import/export/push and
  mirror repair.
- Any future executable Web repair flow must reopen a separate plan batch with
  manual confirmation and fail-closed gates before code implementation.

## Verification

- Plan boundary updated in `docs/plan/14_tech_stack.md`.
- Feature boundary updated in `docs/features/07_diff_logic.md`.
- Acceptance assertions updated in `docs/acceptance-cases/04_diff.md`.
