# Storage / Repo Acceptance Drift - 2026-05-12

本报告记录 `Storage / Repo acceptance command drift audit` 批次。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/acceptance-cases/07_storage_repo.md`
- `scripts/check-storage-repo-baseline.sh`
- `crates/core/tests/*storage*`
- `apps/cli/src/commands/{init,recover,export}/`
- `apps/cli/src/server/open_doc_scope_test.rs`

## Changes

- `STORE-001..010` 不再引用当前 CLI surface 不存在的伪命令。
- 新增 storage/repo baseline guard，阻止以下伪命令回流：
  - `deve repo create`
  - `deve db inspect`
  - `deve doc edit`
  - `deve api call`
  - `deve path normalize`
  - `deve recover --from-ledger`
  - ad hoc PowerShell filesystem mutation steps
- 新增或补强最小测试证据：
  - CLI init 创建 ledger/vault/.notegit/.gitignore 物理布局。
  - 同名不同 URL repo 分配 collision-safe 物理名。
  - required Redb tables 覆盖 `SNAPSHOT_DATA`。
  - Markdown export 保留用户 frontmatter 且不注入 system metadata。
  - `recover` 从 ledger facts 重建 workspace 文件。
- 修正 `RequestHistory` 删除态测试期望：删除文档返回 `DOC_NOT_FOUND`，与 `OpenDoc` 删除态保持一致。

## Verification

已运行：

- `scripts/check-storage-repo-baseline.sh`
- `cargo fmt --check`
- `cargo test -p deve_core --test store_acceptance_test --test local_repo_metadata_repair_test --test path_normalize_structure_test --test watcher_create_modify_delete --test watcher_internal_ignore --test store_ledger_first_test --test ledger_seq_monotonic_test --test rebuild_projection_test -- --nocapture`
- `cargo test -p deve_cli request_history_on_deleted_doc_returns_error_without_history -- --nocapture`
- `cargo test -p deve_cli`
- `scripts/plan-coverage.sh`

结果：

- storage/repo baseline: pass
- targeted core storage/repo tests: pass
- `deve_cli`: `778 passed` plus `agent_bridge_test`: `2 passed`
- plan coverage: pass, blocking violations 0
