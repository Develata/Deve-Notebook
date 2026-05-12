# Feature Acceptance Gap Scan - 2026-05-12

本报告记录 browser smoke 之后的下一轮 feature / acceptance / code 交叉扫描。`docs/plan/` 仍是唯一权威；本文件只作为执行队列输入。

## Scope

- `docs/features/operations/`
- `docs/acceptance-cases/`
- `docs/overview/architecture-diff.md`
- 当前 Commands / Settings / Search / Rendering large-doc search gate 代码路径

## Findings

### F1. Commands / Settings Baseline Is Closed

已验证：

- `deve_cli config --help` 暴露 `print` / `set`。
- `cargo test -p deve_cli config -- --nocapture` 通过。
- `scripts/check-cli-settings-baseline.sh` 通过。

当前未发现 Commands / Settings 用户验收缺口。

### F2. Rendering Large-Doc Search Gate Had Documentation Drift

问题：

- `docs/features/operations/rendering_large_doc_search_gate.md` 仍指向旧路径 `apps/web/src/hooks/use_core/callbacks_misc.rs`。
- 当前代码路径是 `apps/web/src/hooks/use_core/callbacks/misc.rs`。
- `RENDER-LARGE-001` 没有显式绑定现有 `large_doc_search_gate` 单测，导致 rendering acceptance 与 large-doc baseline 的自动化入口不对称。

处理：

- 修正 feature operation 的 Application Entry。
- 在 `RENDER-LARGE-001` 下加入 `scripts/check-large-doc-baseline.sh` 与 `cargo test -p deve_web large_doc_search_gate -- --nocapture`。
- 扩展 `scripts/check-large-doc-baseline.sh`，防止该路径和 acceptance 绑定再次漂移。

## Next Execution Queue

1. Rendering browser spot smoke：checkbox writeback、math block、large-doc search gate 的 Chrome MCP 手工点验。
2. 若 spot smoke 暴露 UI/runtime 缺陷，先修缺陷；否则继续下一轮 feature / acceptance gap scan。
