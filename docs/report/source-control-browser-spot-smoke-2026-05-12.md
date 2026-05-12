# Source Control Browser Spot Smoke - 2026-05-12

本报告记录 Chrome MCP Source Control 点验。测试使用隔离数据根，不依赖 checked-in `ledger/` 或 `vault/`。

## Environment

- Backend: `DEVE_LEDGER_DIR=/tmp/deve-sc-spot-6hjFES/ledger DEVE_VAULT_PATH=/tmp/deve-sc-spot-6hjFES/vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001`
- Frontend: `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080`
- Browser URL: `http://127.0.0.1:8080/`
- Data root: `/tmp/deve-sc-spot-6hjFES`

## Results

Passed:

- Web shell reached `Ready`.
- Created `sc-spot.md` through the Web UI.
- Edited `sc-spot.md` through the CodeMirror runtime; content persisted to the isolated vault.
- Added external file changes under the isolated vault to exercise watcher-driven Source Control pending entries.
- Source Control showed `external-new.md` as `A` and `sc-spot.md` as `M`.
- `暂存全部更改` moved both entries into `暂存的更改`; commit message input became enabled.
- `取消暂存全部更改` moved the entries back to `更改`; commit message input became disabled.
- Re-staged changes, entered commit message, clicked `提交`, and observed `Committed: 0bf1807`.
- After commit, Source Control returned to clean local branch state.
- Added `refresh-spot.md` externally; Source Control refreshed to show one `A` pending entry.
- Reloaded the page, re-entered Source Control, and confirmed `refresh-spot.md` was still visible after reconnect.
- Bottom status remained `就绪`.
- Post-reload current-page console error/warn list was empty.

## Notes

- UI-origin editor writes go directly through the ledger authority and do not create Source Control pending entries. The stage/unstage path therefore used external vault writes, which is the intended watcher / `pending_fs_ops` path.
- During the long pre-reload DevTools session, Chrome recorded one transient `/api/auth/status` `ERR_INSUFFICIENT_RESOURCES`. A reload and stable recheck produced no current-page console warnings or errors, so this was not treated as a runtime defect.

## Verification

已运行：

- Chrome MCP manual smoke for Source Control stage / unstage / commit / refresh / reload recovery.
