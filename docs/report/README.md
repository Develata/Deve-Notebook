# Report Archive

`docs/report/` contains time-stamped audits, gap scans, progress snapshots, and schedule notes.

These files are **non-authoritative**. They record what an audit believed at a specific time; they do not override:

- `docs/plan/`
- `docs/features/operations/`
- `docs/overview/architecture-diff.md`
- current code

## Reading Rules

1. Treat every report as historical evidence, not as a live contract.
2. Prefer the newest dated baseline when comparing reports.
3. Re-check code and `architecture-diff.md` before acting on any reported gap.
4. Do not copy old report assertions into plan or operation docs without revalidation.

## Current Baseline

- Latest broad baseline: `baseline-2026-04-08.md`
- Latest release smoke status: `release-smoke-status-2026-04-28.md`
- Older 2026-02-28 audits are retained only to explain previous decisions.
- If a report conflicts with the operation-level architecture view, treat the report as stale until re-audited.

## Archive Contents

- `baseline-*`: consolidated state baseline at a specific date
- `gap-*`: domain-specific plan/code gap scan at a specific date
- `release-smoke-status-*`: release/runtime smoke status snapshots
- `*-audit-*`: historical audit notes
- `schedules/`: execution schedules and progress snapshots
- `next-tasks.md`: historical task queue, not an active backlog by itself
