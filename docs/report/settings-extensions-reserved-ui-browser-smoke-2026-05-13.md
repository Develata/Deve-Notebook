# Settings / Extensions Reserved UI Browser Smoke - 2026-05-13

## Scope

- Source of truth: `docs/plan/10_ai_agent.md`, `docs/plan/13_settings.md`, `docs/plan/17_plugins.md`.
- Acceptance focus: `AI-005`, `SET-006`, `SET-007`, `PLUG-002`.
- Boundary: no server-backed Settings API, no Calculation Runtime executor, no new plugin runtime.

## Environment

- Web build: `bash scripts/smoke-web-release-build.sh`
- Server: `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 31991`
- Static dir: `apps/web/dist`
- Isolated data root: `/tmp/deve-settings-extensions-20260513-Gi2uuM`
- HTTP probes used `curl --noproxy '*'` because the shell has localhost proxy variables.

## HTTP Evidence

- `GET /api/ai/backend-capabilities` returned `200`:
  - `native_available=true`
  - `trusted_cli_available=false`
  - `trusted_cli_reason="external agent disabled"`
  - `effective_backend="native"`
- `GET /api/settings` returned `404` with empty body.

## Browser Evidence

- Settings modal opens from Command Palette.
- Settings boundary copy is visible and includes `deve config set` and `config.toml`.
- Native AI backend is enabled.
- Trusted CLI backend is disabled with title/description `external agent disabled`.
- Trusted CLI backend exposes `aria-disabled="true"`.
- Hybrid Editing reserved card exposes:
  - `data-deve-setting-disabled="true"`
  - `aria-disabled="true"`
  - visible future copy: `未来设置：当前版本不可用`
- Extensions panel is visible.
- Trusted CLI extension card is disabled, shows default-off copy, and exposes `aria-disabled="true"`.
- Calculation Runtime card is visible with planned state and disabled execution copy.
- Calculation Runtime card exposes:
  - `data-deve-extension-reserved="calculation-runtime"`
  - `aria-disabled="true"`
  - title `代码执行已禁用`
- Plugin Runtime reserved card is visible.
- Browser console had no error or warning entries during this smoke.

## Code Adjustment

- Added `aria-disabled` to Settings AI backend buttons.
- Added `aria-disabled` to Extensions AI channel buttons.
- Added reserved marker and `aria-disabled` to the Calculation Runtime card.
- Extended `scripts/check-ai-baseline.sh` to guard the Calculation Runtime reserved marker.

## Status

Pass. Current Settings / Extensions UI correctly presents reserved capabilities as visible but disabled/future, while keeping server-backed Settings API and Calculation Runtime executor outside the current release surface.
