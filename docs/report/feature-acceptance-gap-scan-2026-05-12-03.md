# Feature Acceptance Gap Scan - 2026-05-12 03

本报告记录 release delivery smoke 之后的 feature / acceptance / code 交叉扫描。`docs/plan/` 仍是唯一权威；本文件只记录执行结果与下一步队列。

## Scope

- `docs/features/operation-coverage.md`
- `docs/features/operations/`
- `docs/acceptance-cases/`
- `apps/web/src/i18n/`
- `apps/web/src/utils/time.rs`
- `apps/web/src/components/chat/message_item.rs`
- 当前 baseline / smoke scripts

## Verification Snapshot

已运行：

- `scripts/check-acceptance-bindings.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-auth-baseline.sh`
- `scripts/check-network-baseline.sh`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-rendering-baseline.sh`
- `scripts/check-search-baseline.sh`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-ai-baseline.sh`
- `scripts/check-graph-baseline.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-i18n-formatting-baseline.sh`
- `cargo fmt --check`
- `cargo test -p deve_web time -- --nocapture`
- `cargo test -p deve_web mobile_chat_readability_markers_are_mobile_only -- --nocapture`

## Findings

### F1. Binding / Registry Baseline Remains Closed

结果：

- automated acceptance bindings: 68
- feature walkthrough bindings: 67
- manual acceptance bindings: 49
- unbound acceptance cases: 0
- architecture registry: 72 flows, 0 active drift
- feature operation path drift: ok

`I18N-005` 已从 feature-only walkthrough 覆盖推进为自动 guard 覆盖。

### F2. I18N-005 Had A Concrete Formatting Gap

问题：

- `docs/plan/11_i18n.md` 要求 Date/Time/Number 不得由组件手写拼装。
- `docs/features/operations/i18n_localized_formatting.md` 将 manual date/time formatting cleanup 标为 plan/code convergence target。
- `apps/web/src/components/chat/message_item.rs` 仍以 `Date.get_hours()` / `Date.get_minutes()` 手写 `HH:MM`。

处理：

- 新增 `apps/web/src/utils/time.rs::format_time_of_day`。
- Chat timestamp 改为通过 `format_time_of_day(ts_ms, locale)` 渲染，并随 locale signal 重算。
- 新增 `scripts/check-i18n-formatting-baseline.sh` 防止回退到组件内手写 `HH:MM`。
- `I18N-005` 增加自动 guard run line。
- `docs/features/operations/i18n_localized_formatting.md` 移除过时的 current-code warning，改为引用 guard。

### F3. Remaining UI Validation Is Browser-Level

当前代码与自动 guard 已闭合。剩余风险是浏览器实际渲染层：

- Settings locale 切换后，chat timestamp 是否随 locale context 重渲染。
- Source Control history relative time 是否继续通过 `format_relative` 正常工作。

这属于 Chrome MCP spot smoke，不需要扩大当前代码改动。

## Next Execution Queue

1. I18N localized formatting browser spot smoke：隔离后端 + Chrome MCP，验证 locale 切换后 chat timestamp / history relative time 可见格式更新。
2. 若 smoke 暴露 UI runtime 问题，先修；否则继续下一轮 feature / acceptance gap scan。
