# Native Process Adapter API Scaffold

Date: 2026-05-14

## Scope

Implemented the typed native process runtime contract scaffold only.

No real child-process runtime was opened.

## Code Changes

- Added `NativeProcessSpawnSpec`, `NativeProcessBindHints`, `NativeProcessEnvBinding`, and `NativeProcessPathResolution`.
- Added `NativeProcessRuntimeHandle`, `NativeProcessRuntimeEvent`, `NativeProcessRuntimeSnapshot`, `NativeProcessRuntimeState`, and `NativeProcessRuntimeFailureKind`.
- Added `NativeProcessRuntimeError` for structured contract validation failures.
- Kept the existing pre-gate `NativeProcessAdapter` boundary separate from the post-gate runtime contract in `process_runtime.rs`.
- Extended `check-native-process-adapter-gate.sh` to pin the scaffold contract surface.

## Guarded Boundaries

- Core still does not import platform process APIs.
- Desktop/Mobile default builds still keep real process runtime disabled.
- Runtime snapshot does not carry session secret, auth token, raw stdout, or raw stderr payload.
- `SpawnSpec` validation rejects empty executable paths, relative executable paths, unknown environment variables, and non-loopback bind hosts.

## Verification

- `cargo test --locked -p deve_core native_adapter::process_test -- --nocapture`
- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo fmt --check`
- `git diff --check`

## Next Step

Implement the Desktop fake runtime harness against this contract. The next batch must still avoid real child-process spawn.
