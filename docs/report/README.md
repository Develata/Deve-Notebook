# Report Index

`docs/report/` contains dated audits, gap scans, runtime smoke snapshots, and the current execution queue.

Most files here are **non-authoritative**. They record what an audit believed at a specific time; they do not override:

- `docs/plan/`
- `docs/features/operations/`
- `docs/overview/architecture-diff.md`
- current code

`next-tasks.md` is the working queue for the current implementation sequence. It is still below `docs/plan/` and current code when conflicts appear.

## Reading Rules

1. Treat every report as historical evidence, not as a live contract.
2. Prefer the newest dated baseline when comparing reports.
3. Re-check code and `architecture-diff.md` before acting on any reported gap.
4. Do not copy old report assertions into plan or operation docs without revalidation.
5. `next-tasks.md` 中只有 `当前执行队列` 是 active；已完成实现历史应进入 dated reports。

## Current Baseline

- Latest broad code-review baseline: `code-review-2026-04-28.md`
- Previous broad baseline: `baseline-2026-04-08.md` (historical only)
- Latest release smoke status: `release-smoke-status-2026-04-28.md`
- Latest file cohesion audit: `soft-size-audit-2026-04-27.md`
- Latest Git mirror bridge status: `git-mirror-bridge-status-2026-04-29.md`
- If a report conflicts with the operation-level architecture view, treat the report as stale until re-audited.

## Archive Contents

- `baseline-*`: consolidated state baseline at a specific date
- `gap-*`: domain-specific plan/code gap scan at a specific date
- `release-smoke-status-*`: release/runtime smoke status snapshots
- `*-audit-*`: historical audit notes
- `next-tasks.md`: current execution queue plus compact legacy migration notes

## Archived 2026-04-08 Gap Inputs

`baseline-2026-04-08.md` 与 `gap-*-2026-04-08.md` 现在只作为归档证据。
它们仍可解释若干实现批次为何启动，但其中大量结论已被
`code-review-2026-04-28.md` 与当前代码覆盖。

已知过时断言包括：Watcher backend 缺失、WS Unauthorized 为 plain text、
Agent Bridge 默认拉起 CLI、locale detection 缺失、`server/mod.rs` 过大，以及
MCP 可作为产品 runtime 方向。

## Retired In 2026-04-28 Cleanup

The following reports were removed from the tree because they were outdated, duplicated newer baselines, or contradicted the current plan/code state. Use git history only if forensic context is needed.

- `apps-audit-2026-02-28.md` and `plan-audit-2026-02-28.md`: superseded by `baseline-2026-04-08.md`, current plan chapters, and the active queue.
- `deve-note current.md`, `deve-note filetree.md`, `deve-note gaps.md`, `deve-note schedule.md`: stale March snapshots; their useful conclusions were replaced by `next-tasks.md` and current plan/code mapping.
- `schedules/01_core.md`, `schedules/02_ui.md`, `schedules/03_extensions.md`, `schedules/04_release.md`: old checkbox progress tables; they duplicated `next-tasks.md` and contained resolved or future-only items as active-looking TODOs.
