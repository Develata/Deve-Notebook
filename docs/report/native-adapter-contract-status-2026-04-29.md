# Native Adapter Contract Status - 2026-04-29

## Result

P3-10 desktop/mobile native adapter has a core contract surface in
`deve_core::native_adapter`.

Implemented scope:

- Platform-neutral state and event model for Desktop and Mobile adapters.
- Loopback-only endpoint validation for injected `http_base` and `ws_base`.
- Session-bound readiness gate before the native web shell can become ready.
- Runtime writable gate requiring endpoint reachability, auth status, node role,
  repo handshake, writer readiness, and current `scope_nonce`.
- Foreground/resume reprobe semantics that do not restore stale write scope.
- Network online/offline classified as hint-only and never as a write grant.
- Service offline/restarting and session invalid states force recovery or
  unauthorized UI instead of half-writable mode.

Out of scope:

- Tauri desktop shell, mobile packaging, system tray, native menu, updater,
  app-store distribution, and platform permission bridges.
- Embedded service process launcher and IPC bridge implementation.
- Web connection-manager consumption of this contract.

## Verification

Commands run:

```bash
cargo test -p deve_core native_adapter -- --nocapture
cargo fmt --all --check
git diff --check
scripts/plan-coverage.sh
```

Observed result:

- `cargo test -p deve_core native_adapter -- --nocapture`: 10 passed.
- `cargo fmt --all --check`: passed.
- `git diff --check`: passed.
- `scripts/plan-coverage.sh`: blocking violations 0, soft warnings 26.

## Next Work

Next native-adapter work should connect this core contract to a real adapter
consumer, in this order:

1. Web bootstrap/connection manager consumes injected endpoint/session data.
2. Server exposes a native-safe launch mode that binds only loopback and reports
   service readiness/offline in the same schema.
3. Desktop shell prototype starts the embedded service and injects the bootstrap
   payload before Web shell loading.
4. Mobile shell prototype adds background/foreground reprobe and suspended
   service semantics.
