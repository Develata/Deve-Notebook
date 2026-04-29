# Native Web Bootstrap Status - 2026-04-29

## Result

P3-10 Web bootstrap consumption of the native adapter endpoint/session contract
has a minimal code surface.

Implemented scope:

- Web reads optional `window.__DEVE_NATIVE_BOOTSTRAP`.
- Accepted bootstrap shape:
  `http_base`, `ws_base`, `session_bound`, optional `node_role`.
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
- UI recovery copy for invalid native bootstrap beyond existing disconnected
  state.

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

The next P3-10 implementation step should be server/native launch surface:

1. Add a native-safe server launch mode or options object that always binds
   loopback unless explicitly overridden for Docker/release server use.
2. Report service readiness/offline in the same endpoint/session vocabulary.
3. Keep production Web server and Docker release behavior unchanged.
