# Git Mirror Bridge Status - 2026-04-29

## 已实现

- Core 新增 `deve_core::git_bridge` 只读状态骨架，当前可判断 `.git` metadata 类型、`.notegit` 是否存在、repo-local `.gitignore` 是否保护 `.notegit/`。
- `.git/` 与 `.notegit/` 已统一为 repo internal path segments；watcher、startup scan、projection rebuild、drift enumeration 不再把 `.git` 内部 Markdown 摄入 pending / projection。
- `materialize`、`rebuild_projection`、`deve init`、server `.notegit` prepare 会幂等写入 repo-local `.gitignore` 规则 `.notegit/`。
- `.gitignore` 保护判定按顺序处理后续 `!` 否定规则；symlinked `.gitignore` fail-closed，不自动写入 repo 外目标。
- CLI 新增 `deve_cli git status [--repo <repo>]`，作为 Git mirror bridge 的只读诊断入口。

## 仍未实现

- 真实 Git mirror commit：`git add -A` / `git commit` 与 Deve commit id、ledger seq、repo id 的映射。
- `GitMirrorQueued`、`GitMirrorCommitted`、`GitMirrorOutOfSync` 的持久化 side table。
- `GitMirrorOutOfSync` 的 retry / repair / status 完整闭环。
- `Git: Export Mirror`、`Git: Import Changes`、`Git: Push Mirror`。

## 验证

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p deve_core`
- `cargo test -p deve_core notegit -- --nocapture`
- `cargo test -p deve_core git_bridge -- --nocapture`
- `cargo test -p deve_core watcher_internal_ignore -- --nocapture`
- `cargo test -p deve_core scan_ignores_git_mirror_markdown_paths -- --nocapture`
- `cargo test -p deve_core rebuild_projection_force_overwrites_and_prunes_stale_markdown -- --nocapture`
- `cargo test -p deve_core git_mirror_paths_are_protected_plugin_targets -- --nocapture`
- `cargo test -p deve_cli git -- --nocapture`
- `cargo test -p deve_cli git_status_accepts_repo_selector -- --nocapture`
- `cargo test`
