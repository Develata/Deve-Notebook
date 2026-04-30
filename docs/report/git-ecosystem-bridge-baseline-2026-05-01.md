# Git Ecosystem Bridge Baseline - 2026-05-01

本报告合并 Git mirror、Git import/export/push 与 repair review 的短状态报告。它是历史基线，不覆盖 `docs/plan/04_storage.md`、`docs/plan/07_diff_logic.md`、`docs/plan/12_commands.md` 或当前代码。

## Current Boundary

- `.notegit/` 仍是 Deve authority/runtime 目录；`.git/` 只是 Git ecosystem mirror。
- watcher、scan、projection rebuild 与 drift enumeration 必须忽略 `.git/` 和 `.notegit/` 内部路径。
- repo-local `.gitignore` 必须保护 `.notegit/`，避免 Git mirror 泄漏 Deve runtime state。
- Git 写操作只允许显式 CLI surface：`deve_cli git mirror`、`git export`、`git import --apply`、`git push`。
- Web 只提供 CLI-only notice 与只读 repair review；不得后台执行 Git，不得让 Command Palette 直接写 Git。
- Git import 只能进入 pending/import；后续仍必须经过 Deve stage/commit 才能写 ledger authority。
- Git mirror 失败不得回滚 Deve commit，只能标记 out-of-sync 并暴露 retry/repair/status 路径。

## Verified Surfaces

- Mirror status、queue state、failure metadata、repair action 与 repair guidance。
- Projection replay export、多 queued/out-of-sync record export、snapshot bootstrap export。
- Dry-run import、`--apply` pending/import 写入、conflict resolution 后 stage/commit/export。
- Push blocker：未导出 mirror record、dirty Git worktree、dirty Source Control、未映射 Git HEAD、remote/branch 配置错误。
- Command-layer smoke 覆盖 import/export/push resolved publish path 与 push blocker。
- Resolved import publish chain：`git import --apply` -> Source Control resolved stage/commit -> `git export` -> `git push`。
- Export 前 push 不得创建 remote branch；export 后 Git worktree 变脏必须阻塞 publish，直到脏文件被清理。
- 最终 remote branch 必须指向 resolved imported Deve commit 对应 `git_mirror_commits` 记录中的 Git commit id。
- 证据测试包括 `git_import_export_push_resolved_publish_roundtrip`、`git_import_apply_resolved_commit_exports_roundtrip`、`resolved_import_keep_fs_commits_and_exports_to_git`、`push_mirror_refuses_unexported_queue_without_touching_remote`、`push_mirror_pushes_exported_head_to_remote`。
- Git drift rescan 已关闭“import 必须生成 ledger facts”的过时表述；import 在 Deve stage/commit 前只能停留在 pending/import。

## Retired Source Reports

- `git-mirror-bridge-status-2026-04-29.md`
- `git-mirror-cli-repair-guidance-status-2026-04-29.md`
- `git-mirror-command-smoke-cohesion-2026-05-01.md`
- `git-mirror-executable-repair-ui-decision-2026-04-29.md`
- `git-mirror-future-boundary-audit-2026-04-30.md`
- `git-mirror-import-export-push-drift-rescan-2026-05-01.md`
- `git-mirror-readonly-repair-review-status-2026-04-29.md`
- `git-mirror-repair-review-data-source-2026-04-29.md`
- `git-mirror-repair-review-ui-polish-2026-04-29.md`
- `git-mirror-repair-review-web-consumption-2026-04-29.md`
- `git-mirror-repair-ui-boundary-status-2026-04-29.md`
- `git-mirror-web-repair-notice-status-2026-04-29.md`
- `git-import-apply-cli-runtime-smoke-2026-04-30.md`
- `git-import-conflict-resolution-runtime-smoke-2026-04-30.md`
- `git-import-export-push-publish-smoke-2026-05-01.md`
- `git-import-resolved-cli-roundtrip-smoke-2026-05-01.md`
- `git-import-resolved-roundtrip-smoke-2026-05-01.md`
- `post-git-priority-reselection-2026-05-01.md`
