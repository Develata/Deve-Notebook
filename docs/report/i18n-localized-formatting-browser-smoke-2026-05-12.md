# I18N Localized Formatting Browser Smoke - 2026-05-12

本报告记录 `feature-acceptance-gap-scan-2026-05-12-03.md` 指定的 I18N localized formatting browser spot smoke。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Environment

- Web assets: `scripts/smoke-web-release-build.sh`
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-i18n-smoke-*/ledger DEVE_VAULT_PATH=/tmp/deve-i18n-smoke-*/vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3104`
- Browser URL: `http://127.0.0.1:3104/`
- Frontend delivery: embedded frontend
- Browser tool: Chrome MCP

## Results

Passed:

- Web release build completed before browser smoke, so embedded frontend included the latest i18n formatting code.
- Web shell reached `Ready` against an isolated data root.
- Chat example created visible chat timestamps.
- Locale toggle from zh-CN to en-US changed chat timestamp display from `19:13:46` to `7:13:46 PM`.
- Created external `i18n-history.md`, staged it through Source Control, committed it, and opened History.
- Source Control History showed the commit relative time as `just now` in en-US.
- Locale toggle back to zh-CN changed the same history relative time to `刚刚`.
- Chat timestamp changed back to Chinese locale time display after the same locale toggle.
- Current-page console error/warn list was empty after the smoke.

## Notes

- Source Control history setup used an external file write under the isolated vault to exercise the watcher / `pending_fs_ops` path.
- Native AI returned the expected missing API key error during chat example execution; this was not an i18n formatting defect.
- The temporary backend and data root were cleaned up after the smoke.

## Verification

已运行：

- `scripts/smoke-web-release-build.sh`
- Chrome MCP manual smoke for chat timestamp and Source Control history relative time locale switching.
- `scripts/check-i18n-formatting-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`
