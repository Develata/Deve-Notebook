# Git Mirror Bridge Status - 2026-04-29

## 已实现

- Core 新增 `deve_core::git_bridge` 只读状态骨架，当前可判断 `.git` metadata 类型、`.notegit` 是否存在、repo-local `.gitignore` 是否保护 `.notegit/`。
- `.git/` 与 `.notegit/` 已统一为 repo internal path segments；watcher、startup scan、projection rebuild、drift enumeration 不再把 `.git` 内部 Markdown 摄入 pending / projection。
- `materialize`、`rebuild_projection`、`deve init`、server `.notegit` prepare 会幂等写入 repo-local `.gitignore` 规则 `.notegit/`。
- `.gitignore` 保护判定按顺序处理后续 `!` 否定规则；symlinked `.gitignore` fail-closed，不自动写入 repo 外目标。
- CLI 新增 `deve_cli git status [--repo <repo>]`，作为 Git mirror bridge 的只读诊断入口。
- Core 新增 lazy-created `git_mirror_commits` side table，记录 `DeveCommit -> GitMirrorQueued / GitMirrorCommitted / GitMirrorOutOfSync`；缺表读取按空表处理，避免 `git status` 或 repo init 隐式迁移旧库。
- 当 repo 工作区存在 `.git/` 且 `.gitignore` 已保护 `.notegit/` 时，成功的 Deve staged commit 会写入 `GitMirrorQueued`；queue 写入失败只记录 warning，不回滚 Deve ledger commit。
- `deve_cli git status` 已输出独立的 mirror readiness `state` 与 queued / committed / out_of_sync summary；queue health 通过 `queue_state` 单独表达。
- CLI 新增 `deve_cli git mirror [--repo <repo>] [--retry-out-of-sync]`，显式执行 queued Git mirror commit。
- 当前 executor 已覆盖单 record 与多 record：单个待处理 record 执行前检查 Git worktree、`.notegit` tracked 泄漏、Source Control pending/staged 清洁度与当前 Git changed paths 是否属于该 Deve commit diff 或 `.gitignore`，通过后执行 `git add -A` / `git commit`；多个积压 records 通过临时 Git index 从 Deve projection diff 逐个生成 `commit-tree` 并 `update-ref`，成功后写入 `GitMirrorCommitted` 与 Git commit hash。Git 命令失败、mirror 未 ready、路径越界、父映射缺失或无 projection diff 都写入 `GitMirrorOutOfSync`，不回滚 Deve ledger commit。
- `deve_cli git status` 已输出 queued/out_of_sync 的 per-commit lagging records，包括 `deve_commit`、`ledger_seq`、`attempts`、`git_commit`、`queued_lag_ms`、`updated_lag_ms`、原始时间戳与失败位置。
- `GitMirrorOutOfSync` records 已持久化结构化 `failure_stage`；CLI 优先读取该字段输出 `failure_location`，旧记录缺字段时才根据 `last_error` 做兼容 fallback。
- `deve_cli git mirror` 已输出 per-record outcome，并在 no-op、out_of_sync、retry 场景给出 mirror/repair/retry hint；失败位置包括 `mirror_not_ready`、`deve_source_control`、`notegit_protection`、`projection_scope`、`git_history_mapping`、`git_worktree`、`git_command` 或 `mirror_executor`。

## 仍未实现

- 自动后台 Git mirror executor 与更完整的 retry / repair UI。
- 更细的 failure subject / offending path / command exit metadata；当前只稳定到 `failure_stage`。
- `Git: Export Mirror`、`Git: Import Changes`、`Git: Push Mirror`。

## 验证

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p deve_core`
- `cargo test -p deve_core notegit -- --nocapture`
- `cargo test -p deve_core git_bridge -- --nocapture`
- `cargo test -p deve_core --test git_mirror_queue_test -- --nocapture`
- `cargo test -p deve_core git_bridge::executor -- --nocapture`
- `cargo test -p deve_core watcher_internal_ignore -- --nocapture`
- `cargo test -p deve_core scan_ignores_git_mirror_markdown_paths -- --nocapture`
- `cargo test -p deve_core rebuild_projection_force_overwrites_and_prunes_stale_markdown -- --nocapture`
- `cargo test -p deve_core git_mirror_paths_are_protected_plugin_targets -- --nocapture`
- `cargo test -p deve_cli git -- --nocapture`
- `cargo test -p deve_cli git_status_accepts_repo_selector -- --nocapture`
- `cargo test`
