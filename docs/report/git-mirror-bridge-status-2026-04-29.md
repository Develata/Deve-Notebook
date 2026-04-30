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
- `GitMirrorOutOfSync` records 已补充兼容 failure metadata：路径类失败可持久化 `failure_subject`，Git 命令失败可持久化 `failure_command` / `failure_exit_status`；CLI `git status` / `git mirror` / `git export` record 明细会输出 `failure_meta[...]`，旧记录缺字段时保持可读。
- 已实现 CLI-only structured repair action schema：core 根据 `failure_stage` / legacy `last_error` 计算 `GitMirrorRepairAction`，CLI record 明细输出 `repair_action[...]` 的 `code`、`retryable_after_fix` 与 `subject`；缺少持久化 subject 的旧记录会 fallback 到 command 或 stage subject。CLI 同时输出 `repair_guidance[...]`，包含 `manual_only=yes`、具体 next step 与 retry command。该 schema 只用于诊断与显式 retry 指引，不自动执行 Git，不把 Web/后台提升为 Git writer。
- `deve_cli git mirror` 已输出 per-record outcome，并在 no-op、out_of_sync、retry 场景给出 mirror/repair/retry hint；失败位置包括 `mirror_not_ready`、`deve_source_control`、`notegit_protection`、`projection_scope`、`git_history_mapping`、`git_worktree`、`git_command` 或 `mirror_executor`。
- CLI 新增 `deve_cli git export [--repo <repo>] [--retry-out-of-sync]`，复用 explicit mirror executor 将 queued Deve commits 导出到 Git mirror，并输出 `git_export[...]` report 与 export/retry hint。
- `git export` 已支持首次 snapshot bootstrap：当 `git_mirror_commits` side table 为空、Git history 为空、source-control clean 且当前 Git changed paths 不越出 Deve projection snapshot 时，从最新 Deve commit 的完整 projection 建立首个 Git commit，并只写入最新 Deve commit 的映射；若 Git 已有 HEAD，则 fail-closed 为 `GitMirrorOutOfSync` / `git_history_mapping`。
- CLI 新增 `deve_cli git import [--repo <repo>]`，只读 dry-run 规划外部 Git/worktree changes；当前会检查 mirror readiness、Git worktree、`.notegit` tracked 泄漏与 Git HEAD，并把 tracked/untracked changes 输出为 change/blocker，不写 ledger、pending_fs、staging 或 `.notegit`。
- CLI 新增 `deve_cli git import --apply [--repo <repo>]`，在无 blocker 时把 Git import plan 写入 `pending_fs_ops`，并通过 `has_conflict` 标记冲突；该路径不写 ledger、`StagedEntry` 或 `.notegit`。
- CLI 新增 `deve_cli git push [--repo <repo>] [--remote <remote>] [--branch <branch>]`，只推送已导出的 `.git` mirror HEAD；当前会 fail-closed 于 mirror 未 ready、Source Control pending/staged 未清、Git worktree 脏、queued/out_of_sync mirror record 未处理、Git HEAD 未映射到最新 `GitMirrorCommitted` record 或 remote/branch 配置错误。失败只输出 blocker，不回滚 ledger，也不写 `.notegit`。
- Command Palette 新增 `Git: Import Changes`、`Git: Push Mirror` 与 `Git: Repair Mirror` 可发现入口；当前只在 Source Control 面板显示 CLI-only notice，不直接执行 Web 后端 Git import/push/repair。repair notice 会引导用户查看 `repair_action[...]`，修复 blocker 后用 `deve_cli git export --repo <repo> --retry-out-of-sync` 重试。Source Control conflict 条目会提示 import 后需在暂存前选择保留文件系统或账本版本。
- Git push CLI 输出与 Web CLI-only notice 已补充 blocker/remote polish：`git_remote` 指向 upstream/origin 或显式 `--remote/--branch`，`git_history_mapping` 指向 export/repair，`git_worktree` 指向 clean/import，`deve_source_control` 指向 stage/commit/discard。

## Future / Deferred

以下项目不属于当前 active queue，仍保持 future/deferred。当前边界是：Git 写操作只允许通过显式 CLI surface；Web 只提供 CLI-only notice 与只读 repair review，不直接执行 Web 后端 Git import/push/repair。

- 自动后台 Git mirror executor 与更完整的 retry / repair UI。
- 可点击 blocker repair UI；当前 repair action schema 与 Web repair notice 已可观测，但仍保持 CLI-only，不自动生成或执行写 Git 的修复操作。
- Web 后端直接执行 Git import/push 与更完整冲突交互。

## 验证

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p deve_core`
- `cargo test -p deve_core notegit -- --nocapture`
- `cargo test -p deve_core git_bridge -- --nocapture`
- `cargo test -p deve_core --test git_mirror_queue_test -- --nocapture`
- `cargo test -p deve_core git_bridge::executor -- --nocapture`
- `cargo test -p deve_core git_bridge::store -- --nocapture`
- `cargo test -p deve_cli status_lines_include_git_mirror_failure_metadata -- --nocapture`
- `cargo test -p deve_core git_bridge::repair_action -- --nocapture`
- `cargo test -p deve_cli status_lines_include_cli_only_repair_action -- --nocapture`
- `cargo test -p deve_core watcher_internal_ignore -- --nocapture`
- `cargo test -p deve_core scan_ignores_git_mirror_markdown_paths -- --nocapture`
- `cargo test -p deve_core rebuild_projection_force_overwrites_and_prunes_stale_markdown -- --nocapture`
- `cargo test -p deve_core git_mirror_paths_are_protected_plugin_targets -- --nocapture`
- `cargo test -p deve_cli git -- --nocapture`
- `cargo test -p deve_cli git_status_accepts_repo_selector -- --nocapture`
- `cargo test`
