# Network Repo Scope Browser Recovery Smoke - 2026-05-13

本报告记录 `Network / repo scope browser recovery smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/05_network.md`
- `docs/plan/06_repository.md`
- `docs/features/05_network.md`
- `docs/features/operations/net_sync_handshake.md`
- `docs/acceptance-cases/06_network.md` NET-001 / NET-005 / NET-009

## Environment

- Frontend build: `bash scripts/smoke-web-release-build.sh`
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-network-scope-20260513-qRs0QB/ledger DEVE_VAULT_PATH=/tmp/deve-network-scope-20260513-qRs0QB/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 31993`
- Browser URL: `http://127.0.0.1:31993/`
- Data root: `/tmp/deve-network-scope-20260513-qRs0QB`
- Local repos: `default`, `notes`

## Results

Passed:

- Web shell reached `Ready` through same-origin embedded frontend and relative `/ws`.
- `default` repo created `default-smoke.md`; edit content persisted to isolated vault.
- Repo switcher exposed both `default` and `notes`.
- Switching `default -> notes` returned a new active repo scope and sent a new browser `SyncHello`; `default-smoke.md` disappeared from the active list.
- `notes` repo created `notes-smoke.md`; edit content persisted under `vault/notes`.
- Switching `notes -> default` returned to `default-smoke.md`; opening it restored the default-repo content and footer `v2/2`.
- Killing the backend changed the header to `Reconnecting`, showed the disconnected lock dialog, and made the editor read-only.
- During disconnection, editor state was `contenteditable=false` and `aria-readonly=true`.
- Restarting the backend on the same port and data root restored `Ready` without browser reload.
- After reconnect, the current repo remained `default`, `default-smoke.md` remained open, and the editor returned to `contenteditable=true`.

## Notes

- Browser console showed expected `ERR_CONNECTION_REFUSED`, WS close, and node-role probe errors only during the forced backend outage.
- No `UnsupportedVersion`, stale-scope lockout, auth lockout, or uncaught application panic was observed in the current-page console after recovery.
- Browser smoke validates observable repo isolation. Stale old-scope frame injection is covered by targeted server and frontend tests because the production UI does not expose a safe manual stale-frame injection surface.

## Verification

已运行：

- `bash scripts/smoke-web-release-build.sh`
- `bash scripts/check-network-baseline.sh`
- `cargo test -p deve_cli ws_endpoint_sync_hello_uses_switched_repo_scope -- --nocapture`
- `cargo test -p deve_cli ws_endpoint_register_writer_after_sync_hello_returns_write_ready -- --nocapture`
- `cargo test -p deve_cli browser_sync_hello_rejects_stale_scope_nonce -- --nocapture`
- `cargo test -p deve_cli browser_sync_hello_rejects_stale_active_db_binding -- --nocapture`
- `cargo test -p deve_cli browser_sync_hello_rejects_stale_bound_repo_and_writer_identity -- --nocapture`
- `cargo test -p deve_web write_gate -- --nocapture`
- `cargo test -p deve_web message_refresh -- --nocapture`
- `cargo test -p deve_web message_repo_scope -- --nocapture`
- Chrome MCP browser smoke as described above

结果：

- Network baseline: pass
- Browser repo switch / reconnect / write gate smoke: pass
- Targeted stale scope and write gate tests: pass
