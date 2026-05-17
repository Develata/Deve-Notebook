# Repo File Operations Browser Smoke - 2026-05-17

本报告记录 Repo File Operations Closure 的浏览器实机 smoke。`docs/plan/` 未修改。

## Scope

- Previous baseline: `docs/report/repo-file-operations-baseline-2026-05-17.md`.
- Runtime: embedded Web frontend served by `deve_cli serve --dev`.
- Data root: isolated `/tmp/deve-fileops-smoke.ul9eb0`.
- Port: `3117`.
- Non-goal: Web Git writer、server-backed Settings API、native process runtime、native authority write、platform signing/device gates。

## Setup

Ran:

- `DEVE_LEDGER_DIR=/tmp/deve-fileops-smoke.ul9eb0/ledger DEVE_VAULT_PATH=/tmp/deve-fileops-smoke.ul9eb0/vault cargo run -p deve_cli --bin deve_cli -- init --path /tmp/deve-fileops-smoke.ul9eb0`
- `scripts/smoke-web-release-build.sh`
- `DEVE_LEDGER_DIR=/tmp/deve-fileops-smoke.ul9eb0/ledger DEVE_VAULT_PATH=/tmp/deve-fileops-smoke.ul9eb0/vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3117`
- `chrome-mcp http://127.0.0.1:3117/`

Login:

- Development login `admin` / `admin`.
- Initial page reached `Ready`.
- Initial repo `default` had `0` docs.

## Smoke Steps

1. Opened the create/search surface from the UI.
2. Submitted `+notes/fileops-source.md`.
3. Observed `Created: notes/fileops-source.md` and Explorer update.
4. Opened command surface and submitted `>mv notes/fileops-source.md notes/fileops-moved.md`.
5. Observed `Renamed: notes/fileops-moved.md` and Explorer update.
6. Submitted `>cp notes/fileops-moved.md notes/fileops-copy.md`.
7. Observed `Copied: notes/fileops-copy.md` and both files visible.
8. Submitted `>rm notes/fileops-copy.md`.
9. Observed `Deleted: notes/fileops-copy.md` and only `fileops-moved.md` visible.
10. Reloaded the page.
11. Observed `Ready`, `default`, and `fileops-moved.md` persisted.
12. Stopped the dev server.
13. Observed disconnected/reconnecting lock state.
14. Restarted the dev server on the same isolated data root and port.
15. Observed automatic recovery to `Ready` with `fileops-moved.md` still present.
16. Reloaded again after recovery.

## Evidence

Final vault state:

- `/tmp/deve-fileops-smoke.ul9eb0/vault/default/notes/fileops-moved.md`
- `/tmp/deve-fileops-smoke.ul9eb0/vault/default/.gitignore`
- `/tmp/deve-fileops-smoke.ul9eb0/vault/default/.notegit/keys/repo.key`

Console:

- During intentional server downtime, Chrome recorded expected `ERR_CONNECTION_REFUSED` and WS reconnect errors.
- After server recovery and final reload, stable console error/warn list was empty.

Network:

- Final stable reload had `200` for document, CSS/JS/WASM/static assets, `/api/auth/status`, and `/api/node/role`.

## Result

- Create, move/rename, copy, delete, reload, and reconnect passed.
- No product code bug was found in this browser smoke.
- Repo File Operations Closure is complete for current Web/server scope.

## Next

Return to mainline gap rescan / next feature selection. Do not open platform post-gates unless target hosts or signing material are explicitly provided.
