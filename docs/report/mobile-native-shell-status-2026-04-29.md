# Mobile Native Shell Status - 2026-04-29

## Result

P3-10 Mobile native shell skeleton now has a minimal code boundary.

Implemented scope:

- New workspace crate `deve_mobile` under `apps/mobile`.
- No Tauri Mobile dependency yet; this is a contract/skeleton crate, not a
  packaged mobile app.
- `MobileShell` models service start, endpoint bound, session bound, Web shell
  loading, runtime ready, background suspended, foreground reprobe, service
  offline, and session invalid.
- Endpoint validation reuses `deve_core::native_adapter`.
- Web bootstrap injection emits `window.__DEVE_NATIVE_BOOTSTRAP` only after the
  endpoint is loopback-valid and session-bound.
- Bootstrap data includes endpoint/session status only; it does not expose token,
  secret, or other session material.
- `Background` / `Suspended` transitions block bootstrap until foreground
  reprobe.
- `Foreground` / `Resumed` clears auth/session freshness, repo handshake,
  writer-ready, and `scope_nonce_current`; write state is restored only after a
  complete `NativeRuntimeReadiness`.
- Network, safe-area, and keyboard lifecycle events are hints only and do not
  grant write authority.

Out of scope:

- Tauri Mobile runtime/window/permission bridge/push/file picker/store packaging.
- Real embedded service process supervision.
- Native session material generation.
- Full Web UI recovery copy for service offline and foreground reprobe states.

## Verification

Commands run:

```bash
cargo test -p deve_mobile -- --nocapture
cargo check --workspace --all-targets
cargo fmt --all --check
scripts/plan-coverage.sh
git diff --check
```

Observed result:

- Mobile shell unit tests passed.
- Workspace all-target check passed.
- New mobile source files stay below the soft-size threshold.
- Plan coverage and diff whitespace checks passed.

## Next Work

The next P3-10 implementation step should improve Web native recovery semantics:

1. Surface invalid native bootstrap, service offline, foreground reprobe, and
   session invalid as explicit UI/runtime statuses.
2. Keep invalid native bootstrap fail-closed without port guessing.
3. Route session invalid to Unauthorized rather than a generic disconnected
   state.
