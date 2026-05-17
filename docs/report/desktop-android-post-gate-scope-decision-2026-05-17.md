# Desktop / Android Post-Gate Scope Decision - 2026-05-17

本报告记录 full regression green 后的 Desktop / Android post-gate 范围决策。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design.md`, `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/08_ui_design_03_mobile.md`, `docs/plan/15_release.md`.
- Code inputs: `apps/desktop/`, `apps/mobile/`, `crates/core/src/native_adapter/`, native target-host workflow and scripts.
- Evidence inputs: Docker/native target-host evidence reports and `mainline-gap-rescan-after-platform-evidence-diagnostics-2026-05-17.md`.
- Boundary: scope decision only; no runtime code change in this batch.

## Current State

- Desktop native shell exists as no-Tauri default plus `native-packaging` feature-gated Tauri scaffold.
- Mobile native shell exists as no-Tauri default plus `native-packaging` feature-gated Tauri scaffold.
- Android generated project and emulator install/startup smoke are covered by target-host evidence.
- iOS shell package/simulator smoke are covered by target-host evidence.
- Desktop macOS/Windows package/startup/installer smoke are covered by target-host evidence.
- `crates/core/src/native_adapter/` already owns shared endpoint/session/readiness/supervisor/process-contract types.
- `apps/desktop/src/process_runtime.rs` is still a fake/test runtime; real child-process runtime remains closed.

## Decision

Open the next post-gate work as **Desktop Local Service Lifecycle Spike (authority-free)**.

Do not open Android process runtime in the same batch.

## Rationale

- Desktop is the lower-risk first process-lifecycle target: macOS/Windows package/startup/installer evidence is already green, and desktop foreground/background semantics are simpler than mobile suspend/resume.
- Android package evidence is already useful as shell-only validation; mobile process lifecycle adds background suspension, foreground reprobe, permission, battery, and sandbox complexity.
- The plan allows native shells to start and supervise a controlled local service only after explicit post-gate opening, while still forbidding native authority writes.
- A Desktop-first spike can validate the service lifecycle contract without changing ledger/vault/source-control/search authority.

## Next Implementation Slice

**Desktop Local Service Lifecycle Spike**:

- Add a real Desktop process runtime behind `apps/desktop` `native-packaging` scope and an explicit runtime-open gate.
- Reuse `NativeProcessSpawnSpec`, `NativeServiceSupervisor`, `NativeEndpointReady`, and `NativeRuntimeReadiness`.
- Start only a controlled `deve_cli serve` process with validated executable, allowlisted env, random loopback endpoint, and bounded restart budget.
- Require endpoint health probe and session handoff before Web shell bootstrap.
- Keep UI writability gated by server/core `writer_ready(repo_id, scope_nonce)`.
- Keep default workspace build and no-packaging skeleton fully closed.

## Acceptance Gates

- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-native-packaging-gate.sh`
- targeted `apps/desktop` process runtime tests for:
  - disabled-by-default runtime.
  - invalid executable/env/bind-hint rejection.
  - loopback-only endpoint.
  - bounded retry.
  - fatal session handoff failure.
  - no token/secret/output payload serialization.
- Desktop target-host package/startup evidence remains green.
- Full regression subset: `cargo test --locked -p deve_core native_adapter`, `cargo test --locked -p deve_desktop`, plus runtime happy/recovery smoke.

## Android Position

Keep Android in **shell-only package execution** for now:

- Continue emulator package/install/startup evidence.
- Continue foreground reprobe and writer-gate assertions.
- Do not start, hold, or restart a backend child process on Android yet.
- Do not claim release ready, store ready, physical-device ready, or native authority writes.

Android process runtime should only be reconsidered after the Desktop lifecycle spike proves the shared service lifecycle contract.

## Explicit Non-Goals

- No native authority write path.
- No direct ledger/vault/source-control/search writes from native shell.
- No Web Git writer.
- No server-backed Settings API.
- No signing, notarization, store release, TestFlight, Play Store, or physical-device readiness claim.
