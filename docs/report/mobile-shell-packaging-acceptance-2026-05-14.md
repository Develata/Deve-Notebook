# Mobile Shell Packaging Acceptance

Date: 2026-05-14

## Scope

- Added mobile Tauri shell manifest metadata under `apps/mobile/tauri.conf.json`.
- Added mobile shell packaging acceptance constants and tests behind `native-packaging`.
- Reused the shared shell icon path convention without opening a mobile runtime entrypoint.

## Boundary

- Android/iOS project generation remains closed.
- Platform package build remains closed.
- Child-process runtime remains closed.
- Native authority writes to ledger, vault, source-control, search index, Git mirror, or `.notegit` remain forbidden.
- Foreground reprobe and session handoff remain required before writable UI.

## Verification

- `cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
