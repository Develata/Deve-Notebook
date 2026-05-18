# Desktop Installer Target-host Evidence Refresh - 2026-05-17

本报告记录 Desktop installer required smoke preflight 后的 macOS / Windows target-host evidence refresh。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/15_release.md`.
- Input implementation: `04723cef50e69b398a421ebe060345ed3e0325c0`.
- Boundary: Desktop package build + installer install/uninstall smoke。
- Non-goal: signing、store、physical-device readiness、native authority writes、Android process runtime、Web Git writer、server-backed Settings API。

## Target-host Results

| Target | Run | Result |
| --- | --- | --- |
| Desktop macOS | `25999444094` | success |
| Desktop Windows | `25999444103` | success |

URLs:

- https://github.com/Develata/Deve-Notebook/actions/runs/25999444094
- https://github.com/Develata/Deve-Notebook/actions/runs/25999444103

## Evidence

macOS:

- `desktop_preflight=success`
- `process_gate=success`
- `invalid_startup_request=skipped`
- `invalid_installer_request=skipped`
- `package_build=success`
- `startup_smoke=skipped`
- `native_session_smoke=skipped`
- `installer_smoke=success`
- `Install result: success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

Windows:

- `desktop_preflight=success`
- `process_gate=success`
- `invalid_startup_request=skipped`
- `invalid_installer_request=skipped`
- `package_build=success`
- `startup_smoke=skipped`
- `native_session_smoke=skipped`
- `installer_smoke=success`
- `Install result: success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

Downloaded and validated artifacts:

```bash
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
DEVE_NATIVE_TARGET_HOST_RUN_ID=25999444094 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-macos \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-25999444094 \
scripts/collect-native-target-host-evidence.sh

DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
DEVE_NATIVE_TARGET_HOST_RUN_ID=25999444103 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-windows \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-25999444103 \
scripts/collect-native-target-host-evidence.sh
```

Validator result:

- `desktop-macos.md`: ok.
- `desktop-windows.md`: ok.

## Decision

Desktop installer target-host evidence is closed for unsigned package-shape smoke. It does not claim signed installer readiness, store readiness, or physical-device readiness. The next batch should rescan mainline gaps after installer evidence closure.

