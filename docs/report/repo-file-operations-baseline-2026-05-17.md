# Repo File Operations Baseline - 2026-05-17

本报告记录 Repo File Operations Closure 的第一批 targeted baseline。`docs/plan/` 未修改。

## Scope

- Previous selection: `docs/report/mainline-feature-implementation-selection-2026-05-17.md`.
- Goal: 为 create / rename / copy / move / delete 文档结构写操作建立窄回归入口。
- Non-goal: 打开 Web Git writer、server-backed Settings API、native process runtime、native authority write 或平台 signing/device gate。

## Changes

- Added `scripts/check-repo-file-ops-baseline.sh`.
- Bound the script to `UI-DESK-003`, `STORE-012`, and `STORE-013`.
- Extended UI desktop and storage baseline guards so the file-op binding cannot silently drift.
- Fixed shellcheck quoting warnings in `scripts/check-storage-repo-baseline.sh`.

## Covered Path

- SearchBox file-op shell parsing and candidate generation.
- FileProvider create candidate and reserved-path filtering.
- Document structure WS `scope_nonce` gate.
- Server docs create/copy/move/delete handler fail-closed and projection-repair cases.
- Degraded local projection write gate.

## Verification

Ran:

- `cargo test -p deve_web file_ops -- --nocapture`
- `cargo test -p deve_web file_provider -- --nocapture`
- `cargo test -p deve_cli docs_scope_nonce_gate -- --nocapture`
- `cargo test -p deve_cli docs_create_test -- --nocapture`
- `cargo test -p deve_cli docs_copy_contract -- --nocapture`
- `cargo test -p deve_cli docs_dir_copy -- --nocapture`
- `cargo test -p deve_cli docs_projection_repair -- --nocapture`
- `cargo test -p deve_cli server::handlers::docs::create::tests -- --nocapture`
- `cargo test -p deve_cli server::handlers::docs::delete::tests -- --nocapture`
- `cargo test -p deve_cli copy_rejects_traversal_source_before_resolving_target -- --nocapture`
- `cargo test -p deve_cli degraded_local -- --nocapture`
- `cargo test -p deve_core source_control_write_gate -- --nocapture`
- `bash scripts/check-repo-file-ops-baseline.sh`
- `bash -n scripts/check-repo-file-ops-baseline.sh scripts/check-storage-repo-baseline.sh scripts/check-ui-desktop-baseline.sh`
- `shellcheck scripts/check-repo-file-ops-baseline.sh scripts/check-storage-repo-baseline.sh scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `git diff --check`

Result:

- All checks passed.
- No product code bug was found in this targeted baseline pass.

## Next

Run a browser smoke against an isolated dev data root:

- Create a document from Unified Search / file surface.
- Move or rename it through file-op mode.
- Copy it to a new destination.
- Delete the copied or moved target.
- Reload and reconnect to confirm projection refresh and no stale UI state.
