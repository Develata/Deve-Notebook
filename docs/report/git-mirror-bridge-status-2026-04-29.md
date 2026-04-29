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

## 仍未实现

- 真实 Git mirror commit executor：`git add -A` / `git commit`，并把 Git commit hash 写回 `GitMirrorCommitted`。
- `GitMirrorOutOfSync` 的 retry / repair 完整执行闭环。
- per-commit status detail、落后项列表与 CLI repair/retry 入口。
- `Git: Export Mirror`、`Git: Import Changes`、`Git: Push Mirror`。

## 验证

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p deve_core`
- `cargo test -p deve_core notegit -- --nocapture`
- `cargo test -p deve_core git_bridge -- --nocapture`
- `cargo test -p deve_core --test git_mirror_queue_test -- --nocapture`
- `cargo test -p deve_core watcher_internal_ignore -- --nocapture`
- `cargo test -p deve_core scan_ignores_git_mirror_markdown_paths -- --nocapture`
- `cargo test -p deve_core rebuild_projection_force_overwrites_and_prunes_stale_markdown -- --nocapture`
- `cargo test -p deve_core git_mirror_paths_are_protected_plugin_targets -- --nocapture`
- `cargo test -p deve_cli git -- --nocapture`
- `cargo test -p deve_cli git_status_accepts_repo_selector -- --nocapture`
- `cargo test`
