# Mobile Packaging Dependency Spike

Date: 2026-05-14

## Scope

- Active queue item: Mobile Packaging Dependency Spike.
- Source of truth: `docs/plan/08_ui_design_03_mobile.md` and `docs/plan/14_tech_stack.md`.
- This batch opens dependency scope only; it does not open Mobile runtime entrypoint,
  Android/iOS package build, child-process runtime, or native authority writes.

## Result

- `apps/mobile` now allows `tauri` and `tauri-build` only as optional
  `native-packaging` dependencies.
- Default `deve_mobile` build remains no-Tauri.
- `native-packaging` feature build includes `tauri` and `tauri-build`.
- `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY` now records
  `DesktopAndMobileDependencySpikeOpen`.
- Boundary scripts allow Tauri dependencies only under `apps/desktop` and
  `apps/mobile`, and continue blocking runtime imports outside declared shell
  bindings.

## Boundary

- No Mobile Tauri runtime entrypoint was added.
- No Android or iOS project/package build was generated.
- No child-process runtime was opened.
- No ledger/vault/source-control/search/`.git`/`.notegit` authority was granted
  to the native shell.
- Foreground reprobe, node-role readiness, repo handshake, writer gate, and
  current `scope_nonce` remain required before writable UI.

## Residual

- Mobile shell manifest acceptance remains separate.
- Android/iOS package build and signing remain unverified.
- Native process adapter remains a separate gate after Desktop/Mobile shell
  packaging acceptance is stable.

## Verification

Run:

```bash
cargo tree --locked -p deve_mobile --no-default-features
cargo tree --locked -p deve_mobile --features native-packaging
cargo check --locked -p deve_mobile --no-default-features
cargo check --locked -p deve_mobile --features native-packaging
cargo test --locked -p deve_mobile --no-default-features -- --nocapture
cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture
cargo test --locked -p deve_core native_adapter::packaging -- --nocapture
scripts/check-native-track-boundary.sh
scripts/check-native-packaging-gate.sh
scripts/check-release-baseline.sh
scripts/check-dev-runbook-baseline.sh
scripts/plan-coverage.sh --summary-missing-plan-ref
cargo fmt --check
git diff --check
```
