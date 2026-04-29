# Git Mirror Web Repair Notice Status - 2026-04-29

## 已完成

- Command Palette 新增 `Git: Repair Mirror`，与现有 `Git: Import Changes` / `Git: Push Mirror` 一样只显示 Source Control CLI-only notice。
- Web notice 新增独立 repair 文案：用户先运行 `deve_cli git status --repo <repo>` 查看 `repair_action[...]`，修复 subject/blocker 后再运行 `deve_cli git export --repo <repo> --retry-out-of-sync`。
- Source Control notice 识别新增 `git-repair-cli-only` 本地 detail，不依赖 server error round-trip，也不触发任何 Web 后端 Git 写入。
- plan 已同步当前边界：Web 只提供 import / push / repair 的可发现 CLI-only notices；自动后台执行、可点击 blocker repair UI 与 Web 后端直接执行 Git repair 仍属后续。

## 工程边界

- `.notegit/` 继续是唯一业务 authority，`.git/` 只是 projection mirror。
- Web repair notice 只解释 repair-action schema 与 retry command；不会自动运行 `git`、`deve_cli git mirror` 或 `deve_cli git export`。
- `repair_action[...]` 仍是 CLI-only 诊断 schema；写 Git 的操作必须由用户显式 CLI 执行。

## 验证

- `cargo test -p deve_web git_bridge_commands_are_localized -- --nocapture`
- `cargo test -p deve_web local_git_cli_notices_are_detected -- --nocapture`
- `cargo test -p deve_web local_git_repair_notice_uses_cli_copy -- --nocapture`
- `cargo test -p deve_web static_commands_include_git_bridge_notices -- --nocapture`
- `cargo fmt --check`
- `scripts/plan-coverage.sh`
- `git diff --check`
