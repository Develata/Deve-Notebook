# Desktop Native Shell Status - 2026-04-29

## Result

P3-10 Desktop native shell skeleton now has a minimal code boundary.

Implemented scope:

- New workspace crate `deve_desktop` under `apps/desktop`.
- No Tauri dependency yet; this is a contract/skeleton crate, not a packaged
  desktop app.
- `DesktopShell` models the allowed native shell states: service start,
  endpoint bound, session bound, Web shell loading, service offline, and session
  invalid.
- Endpoint validation reuses `deve_core::native_adapter`.
- Web bootstrap injection emits `window.__DEVE_NATIVE_BOOTSTRAP` only after the
  endpoint is loopback-valid and session-bound.
- Bootstrap data includes endpoint/session status only; it does not expose token,
  secret, or other session material.
- Service offline and session invalid states block bootstrap and report recovery
  state instead of granting write authority.

Out of scope:

- Tauri v2 runtime/window/menu/tray/installer/autoupdate.
- Real embedded service process supervision.
- Native session material generation.
- Full runtime-ready gating across auth, WS handshake, writer ready, and current
  `scope_nonce`.

## Verification

Commands run:

```bash
cargo test -p deve_desktop -- --nocapture
cargo check --workspace --all-targets
cargo fmt --all --check
scripts/plan-coverage.sh
git diff --check
```

Observed result:

- Desktop shell unit tests passed.
- Workspace all-target check passed.
- Web native bootstrap non-wasm dead-code warnings were suppressed with narrow
  target-gated `allow(dead_code)` annotations.
- Plan coverage and diff whitespace checks passed.

## Next Work

The next P3-10 implementation step should mirror the skeleton for mobile:

1. Add a mobile shell skeleton with foreground/background/suspended/resumed
   transitions.
2. Keep mobile lifecycle events as reprobe hints, not write grants.
3. Preserve the same endpoint/session/bootstrap contract as desktop.
