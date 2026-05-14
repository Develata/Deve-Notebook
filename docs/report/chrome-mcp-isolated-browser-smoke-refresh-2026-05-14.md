# Chrome MCP Isolated Browser Smoke Refresh - 2026-05-14

本报告记录当前 Web runtime 的 Chrome MCP 隔离数据根 smoke。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- Plan basis: `05_network.md`, `06_repository.md`, `15_release.md`, `16_web_thin_client_ledger.md`.
- Runtime: `serve --dev` with isolated `DEVE_LEDGER_DIR` and `DEVE_VAULT_PATH`.
- Static frontend: `DEVE_STATIC_DIR=apps/web/dist`.
- Browser tool: Chrome MCP.
- Non-goal: 修改 `docs/plan/`、打开 native packaging gate、测试 Tauri/Desktop/Mobile packaging。

## Environment

- URL: `http://127.0.0.1:32191/`.
- Temporary data root: `/tmp/deve-browser-smoke.mEpGM1`.
- Dev auth: `admin` / `admin`.
- Created document: `Untitled.md`.
- Test content:

```md
# Browser smoke note

Edited at 2026-05-14.
```

The temporary data root was removed after the smoke.

## Checks

### Login and Ready

- Loaded static Web frontend from the CLI server.
- Logged in with development credentials.
- Reached `Ready` / `就绪`.
- `/api/auth/status` and `/api/node/role` returned `200`.

### Create, Open, Edit, Save

- Created `Untitled.md` from the Web quick action.
- Opened the document in the CodeMirror editor.
- Edited content through the active browser editor runtime.
- Verified workspace writeback on disk:

```text
/tmp/deve-browser-smoke.mEpGM1/vault/default/Untitled.md
# Browser smoke note

Edited at 2026-05-14.
```

### Reload Recovery

- Reloaded the page.
- Reopened `Untitled.md`.
- Verified the editor read back the saved content.
- Reached `v2/2` and `Ready`.

### Disconnect and Reconnect

- Stopped the isolated backend.
- Browser entered `Offline` / `离线` with disconnect overlay and reconnect status.
- Restarted the backend with the same isolated data root.
- Browser reconnected without manual logout/login.
- Reopened the document and verified content remained intact.

### Stable Console and Network

- During the forced backend downtime, preserved console/network logs contained expected `ERR_CONNECTION_REFUSED` and WebSocket close events.
- After backend restart and final reload, steady-state console had no `warn` or `error`.
- Final steady-state network requests were `200`, including static assets, `/api/auth/status`, `/api/node/role`, and editor bundle.

## Result

Chrome MCP isolated browser smoke passed. No runtime bug was found in login, Ready, document create/open/edit/writeback, reload recovery, or forced disconnect/reconnect recovery.

## Decision

No code change is required from this smoke. The remaining active decisions are process-gated: either explicitly authorize the `docs/plan/` error-code catalog patch, or explicitly open the native packaging gate.
