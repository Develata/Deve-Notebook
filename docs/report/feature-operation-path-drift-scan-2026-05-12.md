# Feature Operation Path Drift Scan - 2026-05-12

本报告记录 feature / acceptance 文档中的代码路径可执行性扫描。`docs/plan/` 仍是唯一权威；本文件只作为执行队列输入。

## Scope

- `docs/features/operations/`
- `docs/acceptance-cases/`
- 当前 `apps/`、`crates/`、`scripts/`、`docs/` 路径

## Findings

### F1. Operation Docs Still Contained Pre-Refactor Flat Paths

自动路径扫描发现若干 operation 文档仍引用重构前扁平模块名，例如：

- `callbacks_sc_write_commit.rs`
- `callbacks_sync_write.rs`
- `message_protocol.rs`
- `message_runtime_sync.rs`
- `app_auth_monitor.rs`
- `serve_support.rs`

这些不是 runtime 缺陷，但会误导后续 Chrome MCP 复现、代码审查与 acceptance trace。

### F2. Drift Concentrated In Runtime Flow Docs

已收敛到当前模块路径的范围：

- Auth session unauthorized flow
- Repo switch / pending navigation / file operations
- Network sync handshake
- Source Control stage, discard, merge, commit and conflict flows
- AI Chat / plugin runtime boundary
- CLI repair feature chapter reference

### F3. Added A Reusable Guard

新增 `scripts/check-feature-operation-paths.sh`：

- 扫描 feature operation 与 acceptance docs 中 backtick 包裹的 source/script/doc 路径。
- 忽略 glob、占位符、brace expansion 与变量模板。
- 对不存在的真实文件或目录 fail closed。

## Next Execution Queue

1. Source Control browser spot smoke：stage/unstage、commit panel read path、source-control status refresh 至少点验一轮。
2. 若 spot smoke 暴露 UI/runtime 缺陷，先修缺陷；否则继续下一轮 feature / acceptance gap scan。
