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

This report has been superseded by later P3-10 batches:

- Web bootstrap consumption: `native-web-bootstrap-status-2026-04-29.md`.
- Server native-safe launch surface: `native-server-launch-status-2026-04-29.md`.
- Desktop/mobile shell skeletons: `desktop-native-shell-status-2026-04-29.md`
  and `mobile-native-shell-status-2026-04-29.md`.
- Native recovery UI and packaging scaffolds:
  `native-web-recovery-status-2026-04-29.md`,
  `desktop-packaging-scaffold-status-2026-04-29.md`, and
  `mobile-packaging-scaffold-status-2026-04-29.md`.

The next native-track step is no longer bootstrap wiring. It should be selected
from the active queue in `next-tasks.md`, currently embedded service
supervision, without opening the real Tauri dependency gate by default.
