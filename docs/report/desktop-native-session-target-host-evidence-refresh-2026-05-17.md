# Desktop Native Session Target-host Evidence Refresh - 2026-05-17

本报告记录 Desktop native-session package smoke 在 macOS / Windows target-host 上的刷新结果。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/09_auth.md`, `docs/plan/15_release.md`.
- Dispatch head: `50efefdf6389d4309dbc42889c79bbf4e92d91e8`.
- Boundary: Desktop `native-packaging` package build、startup smoke、native-session package smoke。
- Non-goal: installer required smoke、Android process runtime、native authority writes、signing、store、physical-device readiness、Web Git writer、server-backed Settings API。

## Fixes

- Windows packaged child process keeps `env_clear()` but preserves `SystemRoot` / `WINDIR`, so Winsock service provider initialization remains available without inheriting broad ambient env.
- Local repo catalog validation now reuses the already-open main repo database handle and only opens secondary repo files from the catalog scan.

## Target-host Results

| Target | Run | Result |
| --- | --- | --- |
| Desktop Windows | `25988752970` | success |
| Desktop macOS | `25989352452` | success |

URLs:

- https://github.com/Develata/Deve-Notebook/actions/runs/25988752970
- https://github.com/Develata/Deve-Notebook/actions/runs/25989352452

## Evidence

Windows:

- `desktop_preflight=success`
- `process_gate=success`
- `package_build=success`
- `startup_smoke=success`
- `native_session_smoke=success`
- `installer_smoke=skipped`
- `Process runtime gate: closed`
- `Native authority writes: closed`

macOS:

- `desktop_preflight=success`
- `process_gate=success`
- `package_build=success`
- `startup_smoke=success`
- `native_session_smoke=success`
- `installer_smoke=skipped`
- `Process runtime gate: closed`
- `Native authority writes: closed`

Downloaded and validated artifacts:

```bash
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
DEVE_NATIVE_TARGET_HOST_RUN_ID=25988752970 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-windows \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-25988752970 \
scripts/collect-native-target-host-evidence.sh

DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
DEVE_NATIVE_TARGET_HOST_RUN_ID=25989352452 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-macos \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-25989352452 \
scripts/collect-native-target-host-evidence.sh
```

Validator result:

- `desktop-windows.md`: ok.
- `desktop-macos.md`: ok.

## Local Validation

- `cargo fmt --check`
- `cargo test --locked -p deve_desktop --features native-packaging service_entrypoint -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging process_runtime -- --nocapture`
- `cargo test --locked -p deve_core database_cache -- --nocapture`
- `cargo test --locked -p deve_core local_repo_metadata -- --nocapture`
- `cargo clippy --locked -p deve_desktop --all-targets --features native-packaging -- -D warnings`
- `cargo clippy --locked -p deve_core --all-targets --features search -- -D warnings`
- `bash scripts/plan-coverage.sh`
- `git diff --check`

