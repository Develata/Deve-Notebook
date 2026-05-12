# WebWrite Pending Navigation Smoke - 2026-05-12

本报告记录 `WebWrite pending navigation browser smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/features/operations/doc_pending_navigation_guard.md`
- `docs/plan/16_web_thin_client_ledger.md`
- `apps/web/src/hooks/use_core/navigation.rs`
- `apps/web/src/components/pending_navigation_modal.rs`
- `apps/web/src/hooks/use_core/effects/message_dispatch_write.rs`
- `apps/web/src/hooks/use_core/effects/message_dispatch_protocol.rs`
- `apps/web/src/editor/sync/live.rs`

## Environment

- Backend: `DEVE_LEDGER_DIR=/tmp/deve-webwrite-pending-xXYjBR/ledger DEVE_VAULT_PATH=/tmp/deve-webwrite-pending-xXYjBR/vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001`
- Frontend: embedded frontend served by `deve_cli serve`
- Browser URL: `http://127.0.0.1:3001/`
- Data root: `/tmp/deve-webwrite-pending-xXYjBR`

## Results

Passed:

- Web shell reached `Ready`.
- Created `pending-a.md` and `pending-b.md`.
- Patched browser `WebSocket.prototype.send` during one local edit to hold the real outbound Edit frame after the frontend accepted it into pending overlay.
- Editor and bottom bar showed `Pending Ack` / `等待确认 (1)`.
- Clicking `pending-b.md` while `pending-a.md` had a pending edit opened the pending navigation modal.
- Modal showed the doc navigation target and the warning that continuing does not mean write success.
- Clicking `取消` closed the modal, kept `pending-a.md` visible, and preserved the pending local edit.
- Clicking `pending-b.md` again reopened the modal.
- Clicking `继续切换` left the current view and opened `pending-b.md`.
- A second held Edit frame was replayed as a matching-scope invalid Delete op with the same `client_op_id`.
- The server returned structured rejection: `变更持久化失败: ... delete beyond end ...`.
- The frontend cleared the matching pending overlay and returned from `等待确认 (1)` to `就绪`.

## Notes

- The Reject check used a controlled Chrome MCP frame mutation: the frontend first created a real pending local edit, then the captured frame was replayed with the same `doc_id`, `client_op_id`, and `scope_nonce`, but with an invalid Delete op. This tests the real server reject path and the real frontend pending-clear path without adding test-only UI hooks.
- The expected browser warning was the structured rejection detail. No `UnsupportedVersion`, disconnect lockout, or stale scope lockout was observed.

## Verification

已运行：

- `cargo test -p deve_web navigation_guard -- --nocapture`
- `cargo test -p deve_web message_dispatch_write -- --nocapture`
- `cargo test -p deve_web message_dispatch_protocol -- --nocapture`
- `cargo test -p deve_web live -- --nocapture`
- Chrome MCP browser smoke as described above
- `cargo fmt --check`
- `git diff --check`
- `scripts/plan-coverage.sh`

结果：

- targeted pending navigation tests: pass
- browser pending modal / Stay / Continue / Reject smoke: pass
- formatting and whitespace checks: pass
- plan coverage: pass, blocking violations 0
