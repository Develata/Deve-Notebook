# Native Packaging Gate Design - 2026-05-14

本报告记录 Desktop/Mobile native packaging gate 的执行设计。`docs/plan/` 仍是唯一权威；本批次不修改 plan，不引入 Tauri 依赖。

## Scope

- Plan basis: `14_tech_stack.md#native-packaging-dependency-gate`, `08_ui_design_02_desktop.md`, `08_ui_design_03_mobile.md`.
- Code scope reviewed: `apps/desktop/`, `apps/mobile/`, `crates/core/src/native_adapter/`, native guard scripts.
- Non-goal: add `tauri`, add `tauri-build`, implement child-process runtime, or claim Desktop/Mobile release readiness.

## Current State

Native track is still a no-packaging skeleton:

- `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY` remains `DeferredUntilRuntimeBatch`.
- `real_tauri_dependencies_allowed = false`.
- `default_build_remains_no_tauri = true`.
- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` remains `DeferredUntilPackagingGate`.
- Desktop and Mobile scaffolds describe target capabilities but import no packaging runtime.
- Guard scripts reject Tauri dependency/import leakage and native child-process runtime leakage.

This is consistent with the plan. The current project has Web, Server, Docker, Desktop skeleton, and Mobile skeleton; it does not yet have real macOS/Windows/Android packages.

## Gate Decision

Do not open the full native packaging gate in the next automatic batch.

Reason:

- Real packaging must be tested on platform-specific runners or hosts. WSL/Linux can validate dependency isolation and no-packaging correctness, but it cannot honestly validate macOS signing, Windows installer behavior, or Android store/package behavior.
- Process adapter and packaging adapter are separate gates. Opening both together would blur service readiness, child-process control, and business write authority.
- Current Web/Server/Docker release shape is stable enough to design the gate, but not a reason to bypass platform-specific acceptance.

## Open-Gate Preconditions

Before adding real `tauri` / `tauri-build` dependencies, the opening patch must satisfy all of:

- Maintainer explicitly approves opening the native packaging dependency gate.
- `scripts/check-native-track-boundary.sh` is updated before dependency changes land.
- Default workspace build remains no-Tauri.
- Tauri dependencies are scoped only to `apps/desktop` or `apps/mobile` behind `native-packaging`.
- No dependency is added to workspace root, `deve_core`, `deve_cli`, or `deve_web`.
- No native layer gains ledger/vault/source-control/search/`.git`/`.notegit` authority.
- Process adapter remains closed unless a separate process-adapter gate is opened.
- Acceptance must distinguish Desktop packaging, Mobile packaging, and process supervision.

## Recommended Implementation Order

### NPG-1 Desktop Packaging Dependency Spike

Goal: prove Desktop Tauri can be feature-gated without contaminating default builds.

Allowed:

- Add optional Desktop-only `tauri` / `tauri-build` under `apps/desktop` `native-packaging`.
- Update native boundary guard to allow those crates only in Desktop feature scope.
- Add tests that default build remains no-Tauri and feature build exposes only shell metadata.

Forbidden:

- Spawning or supervising a real backend process.
- Writing ledger/vault/source-control/search or Git state from native code.
- Treating packaging success as runtime readiness.

Minimum verification:

- `cargo check --workspace --locked --no-default-features` or equivalent no-Tauri default check.
- `cargo test -p deve_desktop --features native-packaging packaging -- --nocapture`.
- `scripts/check-native-track-boundary.sh`.
- `scripts/check-native-packaging-gate.sh` updated to reflect the opened Desktop feature scope.

### NPG-2 Desktop Shell Packaging Acceptance

Goal: validate shell-level desktop capabilities.

Acceptance surface:

- window shell
- menu bar
- system tray
- installer metadata
- auto-update metadata or explicit disabled state
- auth/session handoff still required before writable UI

This phase still must not open child-process runtime unless process adapter gate is separately approved.

### NPG-3 Mobile Packaging Dependency Spike

Goal: prove Mobile Tauri can be feature-gated without weakening foreground reprobe.

Allowed:

- Add optional Mobile-only packaging dependency under `apps/mobile` `native-packaging`.
- Keep lifecycle correctness in no-packaging skeleton tests.
- Add packaging tests for WebView shell, permission bridge, share sheet, deeplink, file picker, push notification, and store package metadata.

Forbidden:

- Reusing Desktop readiness as Mobile foreground readiness.
- Restoring old `scope_nonce` after background/resume.
- Claiming offline-first writable mobile app without service/session/writer readiness.

### NPG-4 Process Adapter Gate

Only after packaging is isolated:

- Open child-process runtime behind a separate process-adapter feature.
- Keep process running distinct from `RuntimeReady`.
- Require endpoint health, auth session, node role, repo handshake, and writer gate before writable UI.
- Add explicit restart budget and failure classification acceptance.

## Required Reports Per Gate

Each native gate patch must produce a dated report covering:

- dependency scope
- default build behavior
- feature build behavior
- authority boundary
- platform tested
- tests run
- residual risks

## Next Action

If the maintainer wants real native packaging now, the next patch should be `NPG-1 Desktop Packaging Dependency Spike`.

If not, native packaging should remain deferred and the next execution batch should return to mainline implementation gap scan.
