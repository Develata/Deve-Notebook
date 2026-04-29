# Git Mirror CLI Repair Guidance Status - 2026-04-29

## 已完成

- `GitMirrorRepairAction` 现在会为缺少持久化 `failure_subject` 的旧 record 提供稳定 subject fallback：Git command 失败优先使用 `failure_command`，其他 failure stage 使用 stage-level subject。
- CLI `git status` / `git mirror` / `git export` 的 per-record 明细新增 `repair_guidance[...]` 行，包含 `manual_only=yes`、action-specific `next` 与 retry command。
- `git status` 的 out-of-sync 修复提示已对齐当前 export surface：修复 `repair_action` subject 后运行 `deve_cli git export --repo <repo> --retry-out-of-sync`。
- 覆盖测试新增所有 `GitMirrorFailureStage` 的 guidance 检查，确保每个 repair action 都有明确 next step；`mirror_executor` 仍保持不可自动重试，`retry_command=-`。

## 工程边界

- 本批次只增强诊断与文案，不新增后台 Git writer。
- `repair_guidance[...]` 是 CLI-only 可观测输出，不代表 Web、后台或 repair action schema 获得自动执行权限。
- `.notegit/` 继续是 authority，`.git/` 仍只是 projection mirror。

## 验证

- `cargo test -p deve_core git_bridge::repair_action -- --nocapture`
- `cargo test -p deve_cli status_lines_include_guidance_for_all_repair_actions -- --nocapture`
- `cargo test -p deve_cli status_lines_include_cli_only_repair_action -- --nocapture`
- `cargo test -p deve_cli git_output -- --nocapture`
- `cargo fmt --check`
- `scripts/plan-coverage.sh`
- `git diff --check`
