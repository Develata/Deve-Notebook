# Browser Storage / Projection Degraded Write-Gate Smoke - 2026-05-13

本报告记录 `Browser storage / projection degraded write-gate smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/04_storage.md`
- `docs/plan/06_repository.md`
- `docs/plan/08_ui_design_01_web.md`
- `docs/acceptance-cases/07_storage_repo.md` STORE-011 / STORE-013

## Environment

- Frontend build: `bash scripts/smoke-web-release-build.sh`
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-browser-degraded-20260513-PBaj7K/ledger DEVE_VAULT_PATH=/tmp/deve-browser-degraded-20260513-PBaj7K/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 31995`
- Browser URL: `http://127.0.0.1:31995/`
- Data root: `/tmp/deve-browser-degraded-20260513-PBaj7K`
- Browser fault injection: Chrome init script returned `indexedDB = undefined` and `crypto.subtle = undefined` before app bootstrap.

## Results

Passed:

- Browser entered degraded storage mode with visible banner: `存储受限（WebCrypto=false, IndexedDB=false, Ed25519=false），当前处于只读模式`.
- Header showed `Read-only`; footer showed `本地 / 只读`.
- Dashboard quick action `新建文档` was disabled.
- Source Control showed local repo `default / 本地` plus readonly status.
- Source Control commit message box and commit button were disabled.
- Source Control readonly hint did not claim the user was on a remote branch; it now says `恢复本地可写状态后才能查看变更、暂存文件或提交。`
- Browser console showed the expected degraded warning and no `UnsupportedVersion`, stale-scope lockout, auth lockout, or uncaught application panic.
- Server logs showed only page connection and `SwitchRepo default`; no writer-ready path was observed for the degraded browser session.
- Degraded local projection server gates rejected docs create, edit, `RegisterWriter`, source-control writes, merge writes, and HTTP source-control writes before mutation.
- Runtime recovery smoke passed with degraded-local, stale-scope cleanup, frontend write-gate, refresh, status, and auth-probe coverage.

## Fix

- Replaced the Source Control `ReadOnly` blocked hint with a generic local write-gate hint.
- Added a unit guard ensuring the `ReadOnly` hint does not mention switching back to local branch.

## Verification

已运行：

- `cargo test -p deve_web storage_capabilities -- --nocapture`
- `cargo test -p deve_web typed_prefs_roundtrip -- --nocapture`
- `cargo test -p deve_web output_write_classification -- --nocapture`
- `cargo test -p deve_cli degraded_local -- --nocapture`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `cargo test -p deve_web readonly_hint_does_not_assume_remote_branch -- --nocapture`
- `bash scripts/smoke-web-release-build.sh`
- Chrome MCP browser smoke as described above

结果：

- Browser storage degraded UI smoke: pass
- Projection degraded write-gate tests: pass
- Runtime recovery smoke: pass
- Source Control readonly hint regression guard: pass
