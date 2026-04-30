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
- Latest release smoke status: `release-smoke-status-2026-04-29.md`
- Latest file cohesion audit: `soft-size-audit-2026-04-27.md`
- Latest Git mirror bridge status: `git-mirror-bridge-status-2026-04-29.md`
- Latest Git mirror Web repair notice status: `git-mirror-web-repair-notice-status-2026-04-29.md`
- Latest Git mirror CLI repair guidance status: `git-mirror-cli-repair-guidance-status-2026-04-29.md`
- Latest Git mirror repair UI boundary status: `git-mirror-repair-ui-boundary-status-2026-04-29.md`
- Latest Git mirror read-only repair review status: `git-mirror-readonly-repair-review-status-2026-04-29.md`
- Latest Git mirror repair review data-source status: `git-mirror-repair-review-data-source-2026-04-29.md`
- Latest Git mirror repair review Web consumption status: `git-mirror-repair-review-web-consumption-2026-04-29.md`
- Latest Git mirror repair review UI polish status: `git-mirror-repair-review-ui-polish-2026-04-29.md`
- Latest Git mirror executable repair UI decision: `git-mirror-executable-repair-ui-decision-2026-04-29.md`
- Latest Git mirror future boundary audit: `git-mirror-future-boundary-audit-2026-04-30.md`
- Latest graph HTTP projection status: `graph-http-projection-status-2026-04-29.md`
- Latest graph Web projection panel status: `graph-web-projection-panel-status-2026-04-29.md`
- Latest graph renderer gate decision: `graph-renderer-gate-decision-2026-04-29.md`
- Latest Search/settings current-boundary audit: `search-settings-boundary-audit-2026-04-29.md`
- Latest Native AI Chat boundary audit: `native-ai-chat-boundary-audit-2026-04-30.md`
- Latest AI effective config boundary status: `ai-effective-config-boundary-status-2026-04-30.md`
- Latest browser UI prefs boundary status: `browser-ui-prefs-boundary-status-2026-04-30.md`
- Latest post-P2 plan/code drift rescan: `post-p2-plan-code-drift-rescan-2026-04-30.md`
- Latest native plan post-gate wording split: `native-plan-post-gate-wording-split-2026-04-30.md`
- Latest native packaging gate recheck: `native-packaging-gate-recheck-2026-04-30.md`
- Latest graph blocked/degraded acceptance polish: `graph-blocked-degraded-acceptance-polish-2026-04-30.md`
- Latest graph structured degraded error: `graph-structured-degraded-error-2026-04-30.md`
- Latest rendering current boundary baseline: `rendering-current-boundary-baseline-2026-04-30.md`
- Latest AgentBridge env alias plan sync: `agent-bridge-env-alias-plan-sync-2026-04-30.md`
- Latest AI Settings UI acceptance depth: `ai-settings-ui-acceptance-depth-2026-04-30.md`
- Latest Settings sync UI acceptance depth: `settings-sync-ui-acceptance-depth-2026-04-30.md`
- Latest Settings language UI acceptance depth: `settings-language-ui-acceptance-depth-2026-04-30.md`
- Latest Settings reserved UI acceptance depth: `settings-reserved-ui-acceptance-depth-2026-04-30.md`
- Latest architecture registry operation ID sync: `architecture-registry-operation-id-sync-2026-04-30.md`
- Latest mobile AI Chat keyboard regression status: `mobile-ai-chat-keyboard-regression-status-2026-04-30.md`
- Latest mobile AI Chat viewport smoke: `mobile-ai-chat-viewport-smoke-2026-04-30.md`
- Latest mobile Diff fixture viewport smoke: `mobile-diff-fixture-viewport-smoke-2026-04-30.md`
- Latest watcher external new-file debounce status: `watcher-external-new-file-debounce-status-2026-04-30.md`
- Latest Source Control external new-file runtime smoke: `source-control-external-new-file-runtime-smoke-2026-04-30.md`
- Latest watcher rename-pair debounce status: `watcher-rename-pair-debounce-status-2026-04-30.md`
- Latest Source Control rename-pair runtime smoke: `source-control-rename-pair-runtime-smoke-2026-04-30.md`
- Latest Web release graph warning cleanup: `web-release-graph-warning-cleanup-2026-04-30.md`
- Latest Web release Browserslist warning triage: `web-release-browserslist-triage-2026-04-30.md`
- Latest cargo-chef warning triage: `cargo-chef-warning-triage-2026-04-29.md`
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
