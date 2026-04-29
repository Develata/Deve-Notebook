# Native Packaging Dependency Gate Decision - 2026-04-29

## Result

P3-10 native packaging dependency gate decision is closed for the current
default build: do not introduce real `tauri` / `tauri-build` dependencies yet.

The existing desktop/mobile packaging scaffolds remain planned feature-gated
surfaces. They are inputs for a future runtime batch, not proof that native
packaging is currently available.

## Implemented Guard

- `deve_core::native_adapter::NativePackagingDependencyGateDecision` records the
  current decision as `DeferredUntilRuntimeBatch`.
- `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY` fixes:
  - `real_tauri_dependencies_allowed = false`
  - `default_build_remains_no_tauri = true`
  - `native_feature_gate_required = true`
  - `authority_writes_allowed = false`
- Desktop/mobile default tests assert that the gate remains closed.
- Desktop/mobile feature-gated packaging scaffold tests assert that planned
  scaffold metadata still agrees with the closed gate.
- `scripts/check-native-track-boundary.sh` now checks the policy and the
  desktop/mobile plan anchors.

## Rationale

- Current native work is still contract/skeleton work: endpoint/session
  injection, readiness, recovery UI, supervisor contract, and process-adapter
  decision.
- Opening Tauri dependencies now would combine window runtime, menus/tray,
  mobile permissions, installers, signing, lifecycle behavior, and service
  supervision into one oversized batch.
- The default build must remain fast, no-Tauri, and suitable for unit-test
  regression of adapter/session/readiness boundaries.
- Mobile packaging requires Android/iOS-specific lifecycle and store package
  decisions that should not be mocked as current capability.

## Still Not Implemented

- No real `tauri` or `tauri-build` dependency.
- No Tauri desktop runtime.
- No Tauri Mobile runtime.
- No installer, signing, updater, tray, native menu, mobile permission bridge,
  file picker, push notification, or store package.
- No native authority over ledger, vault, source-control, search, `.git`, or
  `.notegit`.

## Verification

- `cargo test -p deve_core native_adapter`
- `cargo test -p deve_desktop`
- `cargo test -p deve_desktop --all-features`
- `cargo test -p deve_mobile`
- `cargo test -p deve_mobile --all-features`
- `cargo check --workspace --all-targets --all-features`
- `cargo fmt --all --check`
- `scripts/check-native-track-boundary.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`

## Next

The next native-track work should stay in contract/UI/runtime-readiness polish
unless the project intentionally opens a heavier platform batch. Opening the
real packaging gate later must be a separate change with dependency, CI,
installer/signing, and platform smoke-test acceptance.
