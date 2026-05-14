# Chrome MCP Isolated Browser Smoke After Native Port Validation - 2026-05-14

本报告记录 URL / endpoint boundary hardening 后的 Chrome MCP 隔离浏览器 smoke。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- Plan basis: `05_network.md`, `06_repository.md`, `15_release.md`, `16_web_thin_client_ledger.md`.
- Runtime: `serve --dev` with isolated `DEVE_LEDGER_DIR` and `DEVE_VAULT_PATH`.
- Static frontend: freshly rebuilt `apps/web/dist`.
- Browser tool: Chrome MCP.
- Non-goal: 修改 `docs/plan/`、打开 native packaging gate、测试 Tauri/Desktop/Mobile packaging。

## Environment

- URL: `http://127.0.0.1:38210/`.
- Temporary data root: `/tmp/deve-chrome-smoke-20260514-PG0sEd`.
- Dev auth: development defaults.
- Created document: `Untitled.md`.
- Test content:

```md
# Browser smoke note

Edited at 2026-05-14 after native port validation.
```

The temporary data root was removed after the smoke.

## Checks

### Build and Startup

- Ran `bash scripts/smoke-web-release-build.sh`.
- Started server with:

```bash
DEVE_LEDGER_DIR=/tmp/deve-chrome-smoke-20260514-PG0sEd/ledger \
DEVE_VAULT_PATH=/tmp/deve-chrome-smoke-20260514-PG0sEd/vault \
DEVE_STATIC_DIR=apps/web/dist \
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 38210
```

- Server served static files from `apps/web/dist`.
- Startup scan initialized an empty isolated default repo.

### Login and Ready

- Loaded static Web frontend through the CLI server.
- Reached `Ready` / `就绪`.
- Runtime shape showed `main (ws:38210) | v0.0.1 | standard | static-dir-override | development | repos:healthy (0/1)`.
- `/api/auth/status` and `/api/node/role` returned `200`.

### Create, Open, Edit, Save

- Created `Untitled.md` through the Web quick action.
- Opened the document in the CodeMirror editor.
- Edited content through the active browser editor runtime.
- Verified workspace writeback on disk:

```text
/tmp/deve-chrome-smoke-20260514-PG0sEd/vault/default/Untitled.md
# Browser smoke note

Edited at 2026-05-14 after native port validation.
```

### Reload Recovery

- Reloaded the page with cache ignored.
- Reopened `Untitled.md` from the Explorer.
- Verified editor read back `Edited at 2026-05-14 after native port validation.`
- Reached `v2/2` and `Ready`.

### Disconnect and Reconnect

- Stopped the isolated backend.
- Browser entered `Reconnecting` / `已断开连接` with disconnect overlay.
- Restarted the backend with the same isolated data root.
- Browser reconnected without manual logout/login.
- The document remained open and retained the saved content.

### Stable Console and Network

- Final stable reload reached `Ready` and listed `Untitled.md`.
- Final stable console had no `warn` or `error`.
- Final stable network requests were `200`, including static assets, `/api/auth/status`, and `/api/node/role`.

## Result

Chrome MCP isolated browser smoke passed. No runtime bug was found in login, Ready, document create/open/edit/writeback, reload recovery, or forced disconnect/reconnect recovery after the URL / endpoint boundary hardening batches.

## Decision

No code change is required from this smoke. The remaining active decisions are process-gated: explicitly authorize the `docs/plan/` error-code catalog patch, explicitly open the native packaging gate, or continue with a fresh mainline implementation gap scan.
