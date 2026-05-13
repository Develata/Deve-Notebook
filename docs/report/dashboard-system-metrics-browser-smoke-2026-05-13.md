# Dashboard SystemMetrics Browser Smoke - 2026-05-13

本报告记录 `Dashboard SystemMetrics browser smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/08_ui_design_01_web.md` §3 Server Dashboard
- `docs/features/08_ui_design_01_web.md` WEB-UI-01 / WEB-UI-02
- `docs/acceptance-cases/05_ui.md` UI-WEB-003

## Environment

- Frontend build: rebuilt with `scripts/smoke-web-release-build.sh`.
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-dashboard-smoke-20260513-AglCs9/ledger DEVE_VAULT_PATH=/tmp/deve-dashboard-smoke-20260513-AglCs9/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32025`
- Browser URL: `http://127.0.0.1:32025/?isolatedContext=deve-dashboard-smoke`
- Data root: `/tmp/deve-dashboard-smoke-20260513-AglCs9`
- Chrome MCP viewport: desktop `1280x900`
- Auth: development defaults, existing authenticated session.

## Results

Passed:

- Dashboard root view rendered with server dashboard marker:
  - `data-deve-dashboard="server"`
  - `data-deve-dashboard-metrics-state="live"`
- `SystemMetrics` arrived through WebSocket and refreshed in place:
  - health sample increased from `4` to `5` before disconnect.
  - after reconnect, sample increased from `11` to `13`.
  - final rebuilt smoke showed health/sync sample `3` after reload.
- Health and sync cards expose WS-backed markers:
  - `data-deve-dashboard-health-source="ws-system-metrics"`
  - `data-deve-dashboard-sync-source="ws-system-metrics"`
- Storage and quick action cards rendered after metrics arrived:
  - `data-deve-dashboard-card="storage"`
  - `data-deve-dashboard-storage-source="ws-system-metrics"`
  - `data-deve-dashboard-storage-db-size-bytes="1589248"`
  - `data-deve-dashboard-storage-doc-count="0"`
  - `data-deve-dashboard-card="quick-actions"`
- Intentional server stop changed Dashboard to frozen snapshot state:
  - `data-deve-dashboard-metrics-state="frozen-disconnected"`
  - visible copy: `已断开连接；显示最后一次指标快照。`
  - write action `新建文档` became disabled.
  - reconnect overlay was visible.
- Restarting the same data root restored Dashboard to `live` and resumed metric refresh.
- RAM-only boundary held:
  - no dashboard / metric / system keys in `localStorage`.
  - IndexedDB stores remained limited to WebLightPeer storage: `offline_cache`, `peer_identity`, `repo_meta`.
  - no dashboard / metric / system IndexedDB store was created.
- Stable final reload had no browser console `error` or `warn`.
- Stable final reload network requests returned `200` or `304`.

Expected during the intentional disconnect:

- Browser emitted normal WebSocket close / connection refused diagnostics while the server was stopped. These were not present after the final stable reload.

## Fixed Gaps

- Added DOM test markers for the Storage and Quick Actions dashboard cards.
- Extended `scripts/check-ui-dashboard-refresh-baseline.sh` to guard the new markers.
- No `docs/plan/` files were changed.

## Verification

已运行：

- `bash scripts/check-ui-dashboard-refresh-baseline.sh`
- `cargo test -p deve_web dashboard_metrics -- --nocapture`
- `scripts/smoke-web-release-build.sh`
- Chrome MCP browser smoke as described above

结果：

- Dashboard refresh baseline guard: pass
- Dashboard metrics tests: pass, 5 tests
- Web release build: pass
- Browser Dashboard SystemMetrics smoke: pass
