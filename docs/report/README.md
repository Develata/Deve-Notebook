# Report Index

`docs/report/` 保存非权威审查基线、历史 gap scan、runtime smoke 摘要与当前执行队列。报告只记录某一时间点的证据，不覆盖：

- `docs/plan/`
- `docs/features/operations/`
- `docs/acceptance-cases/`
- `docs/overview/architecture-diff.md`
- 当前代码

`next-tasks.md` 只记录 active execution queue；已完成历史进入 dated baseline。

## Reading Rules

1. 报告是历史证据，不是 live contract。
2. 比较状态时优先读最新主题 baseline。
3. 执行任何 TODO 前必须复查当前代码、plan、feature operations 与 acceptance cases。
4. 不得把旧报告断言直接复制到 plan 或 feature 文档。
5. 若报告与 `architecture-diff.md` 或当前代码冲突，先视为 stale，重新审查后再行动。

## Current Baselines

| Topic | Current Report |
| --- | --- |
| Global code review | `code-review-2026-04-28.md` |
| Core hardening | `core-hardening-baseline-2026-05-01.md` |
| Git ecosystem bridge | `git-ecosystem-bridge-baseline-2026-05-01.md` |
| Native shell | `native-shell-baseline-2026-05-01.md` |
| Mobile UI | `mobile-ui-baseline-2026-05-01.md` |
| Graph | `graph-baseline-2026-05-01.md` |
| Settings / AI | `settings-ai-baseline-2026-05-01.md` |
| Source Control runtime | `source-control-runtime-baseline-2026-05-01.md` |
| Release verification | `release-verification-baseline-2026-05-01.md` |
| File cohesion | `soft-size-audit-2026-04-27.md` |
| Plan/report boundary | `plan-code-mapping-extracted-2026-05-01.md` |

## Archived Inputs

- `baseline-2026-04-08.md` 与 `gap-*-2026-04-08.md` 是 raw historical scans，只作 forensic input。
- 已知过时断言包括：Watcher backend 缺失、WS Unauthorized 为 plain text、Agent Bridge 默认拉起 CLI、locale detection 缺失、`server/mod.rs` 过大，以及 MCP 可作为产品 runtime 方向。
- 旧 acceptance checklist 已删除；权威入口是 `docs/acceptance-cases/00_index.md`。

## Retired Cleanup Policy

重复短报告会被合并到主题 baseline；被合并的原文件只在 git history 中保留。需要追溯具体批次时，先读主题 baseline 的 `Retired Source Reports` 列表，再查对应提交历史。
