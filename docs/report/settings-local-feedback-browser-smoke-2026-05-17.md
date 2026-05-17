# Settings Local Feedback Browser Smoke - 2026-05-17

本报告记录 Settings Local Persistence / Feedback Closure 的浏览器实测。`docs/plan/` 未修改。

## Environment

- Data root: `/tmp/deve-settings-smoke.WtGZPv`
- Server: `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3121`
- Frontend: embedded release assets from `scripts/smoke-web-release-build.sh`
- Browser: Chrome MCP attached to `http://127.0.0.1:3121/`
- Auth: development defaults

## Steps

1. Built embedded Web assets.
2. Initialized isolated data root.
3. Started dev server with isolated `DEVE_LEDGER_DIR` and `DEVE_VAULT_PATH`.
4. Opened Command Palette and selected `打开设置 (config)`.
5. Switched locale from `中文` to `English`, then back to `中文`.
6. Switched Sync Mode from `自动` to `手动`.
7. Observed AI Backend buttons.
8. Observed Hybrid Editing reserved setting.
9. Reloaded the page and reopened Settings.
10. Checked current navigation console and core API status.

## Observations

- Settings modal opened from Command Palette.
- Locale feedback was immediate:
  - English mode showed `Settings`, `Sign out`, and English dashboard labels.
  - Chinese mode restored `设置`, `退出登录`, and Chinese dashboard labels.
- Locale persistence boundary worked:
  - `localStorage["deve.ui.locale"] = "zh-CN"` after switching back.
  - Reload restored Chinese UI.
- Sync Mode feedback was immediate:
  - `手动` became active with the manual highlighted class.
  - This is runtime/local feedback, not config file persistence.
- AI backend feedback matched server capability:
  - `原生` was enabled and selected.
  - `受信任 CLI` was disabled.
  - Disabled reason was visible through `title="external agent disabled"`.
  - `aria-disabled="true"` was present on the disabled button.
- Reserved Hybrid Editing marker was present:
  - `data-deve-setting-disabled="true"`
  - `aria-disabled="true"`
  - copy included `未来设置：当前版本不可用`.
- Core API checks:
  - `/api/auth/status`: `200`
  - `/api/node/role`: `200`, `delivery = embedded-frontend`, `profile = standard`
  - `/api/ai/backend-capabilities`: `200`, `effective_backend = native`, `trusted_cli_available = false`
- Current navigation console:
  - No `error` or `warn` messages after reload.

## Result

PASS.

No product code bug was found in this smoke. The next local step is a closure rescan for Settings baseline plus browser smoke, then select the next mainline implementation batch.
