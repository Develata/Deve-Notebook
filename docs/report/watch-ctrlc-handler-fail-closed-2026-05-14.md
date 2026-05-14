# Watch Ctrl-C Handler Fail-Closed

日期：2026-05-14

## 范围

Fresh mainline gap scan after `node-check --projection` fail-closed cleanup.

Plan 来源：

- `docs/plan/04_storage.md#watcher-contract`
- `docs/plan/12_commands.md#cli-commands`

Acceptance 来源：

- `docs/acceptance-cases/11_commands_settings.md` / `CMD-001`
- `docs/acceptance-cases/07_storage_repo.md` / `STORE-007`

## 发现

`deve watch` 是当前 CLI surface。Watcher 初始化失败必须 fail-closed，且同一 repo watcher 生命周期必须可控。

当前实现先完成 scan 并启动 repo watchers，再用 `ctrlc::set_handler(...).expect(...)` 注册 Ctrl+C handler。若 handler 注册失败，CLI 会 panic，且 watcher 已经进入启动路径；这不是 clean fail-closed。

## 修改

- 将 Ctrl+C handler 安装提前到 scan 与 watcher start 之前。
- 将 `expect` 改为 `anyhow::Context`，错误通过 `Result` 返回。
- 抽出 `shutdown_signal_handler`，补测试确认 handler 会把 watch loop 标记为停止。
- 扩展 `scripts/check-cli-settings-baseline.sh`，防止 `deve watch` 回退到 panic handler。

## 验证

- `cargo test -p deve_cli watch -- --nocapture`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`

## 结果

`deve watch` 的 Ctrl+C handler 注册现在是启动前置条件。注册失败会返回 CLI 错误，不会在 watcher 启动后 panic。
