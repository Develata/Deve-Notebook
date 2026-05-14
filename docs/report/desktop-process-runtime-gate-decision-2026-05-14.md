# Desktop Process Runtime Gate Decision

Date: 2026-05-14

## Scope

Reviewed whether the Desktop child-process runtime gate should open after the
Linux package verification and macOS/Windows target-host preflight batch.

No real child-process runtime was opened.

## Decision

Decision: `KeepClosedUntilTargetHostPackages`.

The process adapter remains closed because target-host package execution is not
complete across Desktop and Mobile platforms:

- Linux Desktop `deb` / `rpm` / AppImage package paths have been verified.
- macOS and Windows Desktop package signing, installer generation, install, and
  startup smoke remain target-host work.
- Android Mobile shell package execution is verified.
- iOS Mobile package execution remains target-host work.

Opening `Command::new`, `tokio::process`, direct spawn, or process ownership
before target-host package execution would make service supervision look ready
without proving package/runtime installation semantics on the platforms that
must carry the native shell.

## Gate Contract

The current contract remains:

- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY.decision = DeferredUntilPackagingGate`.
- `child_process_runtime_enabled = false`.
- `packaging_gate_required = true`.
- `authority_writes_allowed = false`.
- Desktop/Mobile default builds remain no-process.
- Desktop fake process runtime remains test-only and state-machine-only.
- Core continues to define process contracts but must not own spawn.

## Required Before Reopening

- Run `DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 scripts/check-desktop-target-host-preflight.sh` on macOS and Windows target hosts.
- Run the corresponding Desktop package build/sign/install/startup smoke on
  those hosts.
- Run Android/iOS package preflight and package execution on target-capable
  hosts.
- Re-review the process runtime implementation scope so real spawn stays under
  app-level `native-packaging`, not core/cli/web/default builds.

## Verification

- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-desktop-target-host-preflight.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`

## Follow-up

Proceed with target-host package execution. Do not implement real process spawn
until the target-host package evidence exists or the plan is explicitly reopened
with a narrower gate.
