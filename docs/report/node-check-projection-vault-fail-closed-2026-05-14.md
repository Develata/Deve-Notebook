# Node Check Projection Vault Fail-Closed

日期：2026-05-14

## 范围

Fresh mainline gap scan after code toolbar boundary cleanup.

Plan 来源：

- `docs/plan/12_commands.md#cli-commands`
- `docs/plan/06_repository.md#tree-projection-contract`

Acceptance 来源：

- `docs/acceptance-cases/11_commands_settings.md` / `CMD-009`

## 发现

`deve node-check --projection` 是当前 CLI surface。该路径需要构造 `SyncManager` 以运行 projection authority 诊断。

生产主路径 `serve`、`watch`、`scan`、`repair` 已使用 `SyncManager::new_checked`。但 `node-check --projection` 仍使用 `SyncManager::new`，间接调用 `Vfs::new`；当 vault root 缺失或不可 canonicalize 时，该路径会 panic，而不是作为 CLI 错误 fail-closed。

## 修改

- `apps/cli/src/commands/node_check.rs` 改用 `SyncManager::new_checked`。
- 增加缺失 vault 的 `node_check --projection` 单测，确认返回错误且不 panic。
- 扩展 `scripts/check-dev-data-health-baseline.sh`，绑定 `node-check` 使用 checked constructor 与现有 acceptance 测试入口。

## 验证

- `cargo test -p deve_cli node_check -- --nocapture`
- `bash scripts/check-dev-data-health-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`

## 结果

`deve node-check --projection` 在 vault root 损坏或缺失时返回可传播错误，不再保留 panic 型生产路径。
