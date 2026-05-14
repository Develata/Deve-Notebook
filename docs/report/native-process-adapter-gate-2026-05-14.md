# Native Process Adapter Gate

Date: 2026-05-14

## Scope

- Added `scripts/check-native-process-adapter-gate.sh`.
- Added release workflow, runbook, acceptance, and release-baseline coverage for the gate.
- Kept the native process adapter as a state-machine-only model.

## Boundary

- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` remains `DeferredUntilPackagingGate`.
- `child_process_runtime_enabled` remains `false`.
- `authority_writes_allowed` remains `false`.
- Desktop/Mobile/native adapter code must not import or call `std::process`,
  `Command::new`, `tokio::process`, or direct spawn APIs.
- Existing endpoint bind, session handoff, probe timeout, and process-stopped
  observations remain modeled as process snapshots only.

## Verification

- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
