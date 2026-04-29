# Native Web Bootstrap Status - 2026-04-29

## Result

P3-10 Web bootstrap consumption of the native adapter endpoint/session contract
has a minimal code surface.

Implemented scope:

- Web reads optional `window.__DEVE_NATIVE_BOOTSTRAP`.
- Accepted ready bootstrap shape:
  `http_base`, `ws_base`, `session_bound`, optional `node_role`.
- Accepted recovery bootstrap shape: optional `service_state` with
  `service_offline`, `foreground_reprobe`, or `session_invalid`.
- Bootstrap endpoint validation reuses `deve_core::native_adapter`.
- Valid native bootstrap replaces inferred WebSocket candidates with the
  injected `ws_base + /ws`.
- Native auth and node-role probes use injected `http_base`.
- Invalid or session-unbound native bootstrap is fail-closed and does not fall
  back to `?ws_port=`, same-origin guessing, or debug localhost candidates.
- Normal browser/Web defaults remain unchanged when native bootstrap is absent.

Out of scope:

- Actual Tauri/mobile shell injection.
- Native embedded service launcher.
- Full native runtime packaging and shell-specific recovery controls.

## Verification

Commands run:

```bash
cargo test -p deve_web native_bootstrap -- --nocapture
cargo test -p deve_web connection_urls -- --nocapture
cargo test -p deve_web auth_status_url -- --nocapture
cargo test -p deve_web http_base_from_ws_url -- --nocapture
cargo check --locked -p deve_web --target wasm32-unknown-unknown
cargo fmt --all --check
scripts/check-network-baseline.sh
git diff --check
```

Observed result:

- Native bootstrap and connection URL tests passed without warnings.
- `cargo check --locked -p deve_web --target wasm32-unknown-unknown`: passed.
- `scripts/check-network-baseline.sh`: passed.

## Next Work

This report was superseded for UI recovery by
`native-web-recovery-status-2026-04-29.md`.

The next P3-10 implementation step should be native packaging dependency
gating:

1. Define the minimal Tauri v2/Tauri Mobile dependency surface behind
   `apps/desktop` and `apps/mobile`.
2. Keep no-Tauri shell skeletons as the fast unit-test boundary.
3. Keep packaging acceptance separate from adapter/session/readiness
   correctness.
