# Native Server Launch Status - 2026-04-29

## Result

P3-10 server/native launch surface now has a minimal implementation boundary.

Implemented scope:

- `ServerLaunchOptions::release(port)` preserves the current release/Docker bind
  behavior: `0.0.0.0:{port}`.
- `ServerLaunchOptions::native_loopback(port, session_bound)` binds only
  `127.0.0.1:{port}` and emits a loopback `NativeEndpointReady` shape.
- Hidden CLI flag `deve_cli serve --native-loopback` exercises the future native
  launcher path without changing normal `deve_cli serve` behavior.
- Native loopback mode refuses plugin-host proxy fallback when the requested
  loopback port is occupied.
- `/api/node/role` now includes nullable `native_service` readiness data. Normal
  server mode returns `null`; native launch mode reports `session_pending` or
  `endpoint_ready` plus endpoint/session fields.

Out of scope:

- Tauri desktop/mobile shell.
- Native pre-auth/session material creation.
- Full runtime ready calculation across auth status, WS handshake, writer ready,
  and current `scope_nonce`.
- Service restart supervisor and real offline retry loop.

## Verification

Commands run:

```bash
cargo test -p deve_cli launch -- --nocapture
cargo test -p deve_cli role_payload -- --nocapture
cargo test -p deve_cli native_loopback -- --nocapture
cargo test -p deve_cli serve_ -- --nocapture
cargo check -p deve_cli
cargo fmt --all --check
scripts/plan-coverage.sh
git diff --check
```

Observed result:

- Release launch still binds `0.0.0.0`.
- Native launch binds `127.0.0.1` and validates through
  `deve_core::native_adapter`.
- Native session-pending state validates endpoint bases but does not pass
  session-bound readiness.
- Native loopback occupied-port behavior fails closed instead of entering proxy
  mode.

## Next Work

Desktop/mobile shell skeletons and Web native recovery semantics were completed
in `desktop-native-shell-status-2026-04-29.md`,
`mobile-native-shell-status-2026-04-29.md`, and
`native-web-recovery-status-2026-04-29.md`.

The next P3-10 implementation step should define the native packaging
dependency gate:

1. Keep Tauri v2/Tauri Mobile dependencies isolated to native app crates.
2. Preserve the current server launch contract for Docker/release behavior.
3. Separate packaging acceptance from adapter/session/readiness correctness.
