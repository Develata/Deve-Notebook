# Process Runtime Gate Decision After Target-host Closure - 2026-05-15

## Scope

Review whether the native child-process runtime gate should open after Desktop target-host installer smoke and Mobile shell package execution evidence.

No real child-process runtime was opened.

## Evidence State

- Desktop macOS `.app/.dmg` package build, packaged startup smoke, and installer install/uninstall smoke are closed by GitHub run `25921302704`.
- Desktop Windows MSI/NSIS package build, packaged startup smoke, and installer install/uninstall smoke are closed by GitHub run `25924163007`.
- Android shell APK package execution is closed by the Android shell-only package gate.
- iOS simulator shell package build is closed by GitHub run `25917428903`.
- Android/iOS device or simulator install/startup smoke remains a separate Mobile runtime gate.

## Decision

Decision: `KeepClosedUntilExplicitRuntimeFeature`.

Target-host package evidence closes the previous packaging blocker, but it does not create a product requirement for native-owned backend process supervision.

The current Desktop/Mobile native layer remains a WebView shell, package surface, menu/tray surface, startup probe, and session-handoff boundary. It must not become a hidden backend launcher only because packaging succeeds.

Opening `Command::new`, `tokio::process`, direct spawn, process ownership, restart supervision, or background service lifecycle must be a separate implementation batch with an explicit feature need, failure contract, tests, and target-host evidence.

## Gate Contract

The current contract remains:

- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY.decision = DeferredUntilPackagingGate`.
- `child_process_runtime_enabled = false`.
- `packaging_gate_required = true`.
- `authority_writes_allowed = false`.
- Desktop/Mobile default builds remain no-process.
- Desktop fake process runtime remains test-only and state-machine-only.
- Core may define process contracts but must not own spawn.
- Native shells must not gain ledger, vault, source-control, search, Git, or `.notegit` write authority through process runtime.

`DeferredUntilPackagingGate` remains the current implementation policy name. `KeepClosedUntilExplicitRuntimeFeature` is this review's post-packaging decision: the old packaging blocker is closed, but runtime ownership still requires a separate feature gate.

## Required Before Opening

- A specific current feature must require native-owned local backend process supervision.
- The implementation must stay under app-level `native-packaging`, not core/cli/web/default builds.
- Spawn spec validation must reject relative executables, unknown env vars, non-loopback endpoints, and unredacted secret/output payloads.
- Health probe, session handoff, restart budget, process exit, and fatal failure paths must be structured and tested before real spawn ships.
- Writable UI must still require endpoint health, auth status, node role, repo handshake, writer-ready, and current `scope_nonce`.
- Mobile process runtime must wait for Android/iOS install/startup evidence and lifecycle review; package build evidence alone is insufficient.

## Verification

- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-target-host-evidence.sh target/native-target-host-evidence-download/deve-native-target-host-evidence-macos/desktop-macos.md`
- `scripts/check-native-target-host-evidence.sh target/native-target-host-evidence-download/deve-native-target-host-evidence-windows/native-target-host-evidence/desktop-windows.md`

## Result

Process runtime remains closed after target-host package and Desktop installer evidence closure.
