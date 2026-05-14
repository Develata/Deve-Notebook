# HTTP Surface Response No-Panic

Date: 2026-05-14

## Scope

- `apps/cli/src/server/auth/headers.rs`
- `apps/cli/src/server/static_files.rs`
- `apps/cli/src/server/static_files_embed.rs`
- `scripts/check-auth-baseline.sh`
- `scripts/check-ui-spa-routing-baseline.sh`

## Contract

- `docs/plan/09_auth.md#security-headers`
- `docs/plan/08_ui_design_01_web.md#single-binary-distribution`

## Change

- Replaced static security header `.parse().unwrap()` calls with `HeaderValue::from_static`.
- Replaced SPA fallback and embedded asset `Response::builder().body(...).expect(...)` paths with explicit `Response::new`, status assignment, and header insertion.
- Preserved response status, content type, SPA fallback, API/WS no-fallback, and embedded frontend behavior.
- Added auth and UI SPA baseline guards to prevent reintroducing panic-backed response construction.

## Verification

- `cargo test -p deve_cli csp_allows_current_wasm_bootstrap_without_external_origins -- --nocapture`
- `cargo test -p deve_cli static_files -- --nocapture`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-ui-spa-routing-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`
