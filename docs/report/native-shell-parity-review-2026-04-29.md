# Native Shell Parity Review - 2026-04-29

## Result

P3-10 native shell parity review is closed for the current no-Tauri track.
Desktop, mobile, and Web now agree on the current readiness/recovery/write-gate
contract without opening process or packaging gates.

## Fixed Gap

- Mobile foreground/resume reprobe now clears `node_role_readable` in addition to
  auth status, repo handshake, writer-ready, and current `scope_nonce`.
- This matches the plan requirement that foreground recovery must reprobe both
  `/api/auth/status` and `/api/node/role` before the shell can return to a
  writable state.

## Current Parity

- Desktop and mobile both expose shell snapshots with `NativeRuntimeReadiness`.
- Desktop and mobile both require complete runtime readiness before
  `RuntimeReady`.
- Desktop and mobile both map foreground/resume recovery to
  `foreground_reprobe`.
- Web native bootstrap already maps `foreground_reprobe` to
  `ConnectionStatus::NativeReprobeRequired`.
- Web write gates block native bootstrap invalid, session pending, service
  offline, and reprobe-required states before normal repo write checks.
- Network online/offline remains a hint and does not grant write authority.
- Service offline/session invalid recovery bootstrap still omits token, secret,
  internal reason, and repo write authority.

## Still Deferred

- Real child-process runtime remains deferred.
- Real Tauri/Tauri Mobile packaging dependency gate remains closed.
- Native menus, tray, installers, mobile permission bridge, file picker, push,
  signing, updater, and store packages remain future runtime work.

## Verification

- `cargo test -p deve_mobile`
- `cargo test -p deve_desktop`
- `cargo test -p deve_core native_adapter`
- `cargo check --workspace --all-targets --all-features`
- `cargo fmt --all --check`
- `scripts/plan-coverage.sh`
- `git diff --check`

## Next

The no-Tauri native track is now at a clean stopping point. Next implementation
work should return to the global P1/P2 queue unless the project intentionally
opens a heavier native runtime batch.
