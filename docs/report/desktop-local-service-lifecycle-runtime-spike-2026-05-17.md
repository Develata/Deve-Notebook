# Desktop Local Service Lifecycle Runtime Spike - 2026-05-17

本报告记录 Desktop local service lifecycle 的第一片实现。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/14_tech_stack.md`.
- Code scope: `apps/desktop/src/process_runtime.rs`, `apps/desktop/src/process_runtime_test.rs`, native gate scripts.
- Boundary: Desktop `native-packaging` only.

## Implemented

- `apps/desktop` now exposes `DesktopLocalServiceRuntime` behind `native-packaging`.
- Runtime owns a controlled `DesktopProcessLauncher` abstraction.
- Default constructor uses `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY`, so child-process runtime remains disabled unless an explicit policy opens it.
- `DesktopCommandProcessLauncher` can spawn and stop a child process.
- Spawn path validates the service command before launching:
  - executable filename must be `deve_cli` or `deve_cli.exe`;
  - first argv must be `serve`;
  - `NativeProcessSpawnSpec::validate_contract()` still enforces absolute paths, loopback bind hints, and env allowlist.
- Process environment uses `env_clear()` and then applies only the spec allowlist bindings.
- Runtime records spawn success, spawn failure, health probe, session handoff, runtime ready, process exit, and controlled stop.
- Runtime snapshots never grant native authority writes.
- Mobile process runtime remains closed.

## Not Opened

- Tauri startup does not automatically start `deve_cli serve` yet.
- No native authority write path.
- No direct ledger, vault, source-control, search-index, `.git`, or `.notegit` writes from native shell.
- No Android process runtime.
- No signing, store release, physical-device readiness, Web Git writer, or server-backed Settings API.

## Validation

- `cargo fmt --check`
- `cargo test --locked -p deve_desktop --features native-packaging process_runtime -- --nocapture`
- `cargo check --locked -p deve_desktop --features native-packaging`
- `cargo clippy --locked -p deve_desktop --all-targets --features native-packaging -- -D warnings`
- `cargo test --locked -p deve_desktop -- --nocapture`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`

## Next

Desktop local service entrypoint wiring:

- bind the runtime to the Desktop `native-packaging` entrypoint behind an explicit opt-in gate;
- resolve the packaged `deve_cli` path without accepting arbitrary executables;
- allocate loopback bind hints without colliding with an existing server;
- perform health probe and session handoff before Web bootstrap;
- preserve writer readiness as the only UI write gate;
- keep Mobile process runtime closed.
