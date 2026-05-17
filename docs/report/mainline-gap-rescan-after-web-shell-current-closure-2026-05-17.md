# Mainline Gap Rescan After Web Shell Current Closure - 2026-05-17

本报告记录 Web shell 当前合同闭合并通过全量回归后的主线缺口复扫。`docs/plan/` 未修改。

## Scope

- Input gate: `docs/report/full-regression-gate-refresh-after-web-shell-current-closure-2026-05-17.md`.
- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/`, `docs/acceptance-cases/`, guard scripts, current code, latest platform evidence.
- Boundary: 不打开 Web Git writer、server-backed Settings API、native process runtime、signing、physical-device 或 native authority writes。

## Verification

Ran:

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/plan-coverage.sh`
- `git rev-list --count 154fcc9140c08016975e7778fdaadf9f647e7298..HEAD`
- `git log --oneline 154fcc9140c08016975e7778fdaadf9f647e7298..HEAD`
- `gh auth status`
- `gh repo view --json nameWithOwner,defaultBranchRef`

Results:

- Acceptance bindings: automated `149`, feature walkthrough `54`, manual `0`, unbound soft `0`.
- Feature operation paths: passed.
- Architecture registry: `72` flows, active drift `0`.
- Plan coverage: blocking violations `0`, soft warnings `18`, dangling `plan_ref` `0`, i18n leaks `0`.
- GitHub auth: available for `Develata/Deve-Notebook`, default branch `main`, token includes `workflow`.
- Previous platform evidence dispatch head: `154fcc9140c08016975e7778fdaadf9f647e7298`.
- Current `HEAD` is `31` commits newer than previous platform evidence.

## Findings

- No new unblocked Current Web/server MUST gap was found.
- No acceptance, feature-path, architecture, or plan-coverage blocker was found.
- Remaining Future / Optional surfaces remain gated:
  - Web Git writer.
  - Server-backed Settings API.
  - Native child-process runtime.
  - Signing, notarization, store release, TestFlight, Play Store.
  - Physical-device readiness.
  - Native authority writes.
- Existing Desktop/Android/iOS work is current-head evidence freshness, not a request to open new platform authority.

## Selection

Next batch: **Current HEAD Platform Evidence Refresh After Web Shell Current Closure**.

The batch should trigger and collect:

- `.github/workflows/docker-smoke.yml` on current `HEAD`.
- `.github/workflows/native-target-host.yml` on current `HEAD` with:
  - `target=all`
  - `required_preflight=true`
  - `run_desktop_package_build=true`
  - `run_desktop_startup_smoke=true`
  - `run_desktop_installer_smoke=true`
  - `run_mobile_android_package_build=true`
  - `run_mobile_android_install_startup_smoke=true`
  - `run_mobile_ios_package_build=true`
  - `run_mobile_ios_install_startup_smoke=true`

Evidence collection must use:

```bash
DEVE_NATIVE_TARGET_HOST_RUN_ID=<run-id> \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
scripts/collect-native-target-host-evidence.sh
```

## Non-Goals

- No real native process runtime.
- No native authority write path.
- No signed release, app store, notarization, TestFlight, Play Store, or physical-device readiness claim.
- No broad UI rewrite.

## Exit Criteria

- Docker release smoke passes on the current pushed `HEAD`.
- Desktop macOS and Windows package/startup/installer smoke pass on the current pushed `HEAD`.
- Android emulator package/install/startup smoke passes on the current pushed `HEAD`.
- iOS simulator package/install/startup smoke passes on the current pushed `HEAD`.
- Evidence artifacts continue to state `Process runtime gate: closed` and `Native authority writes: closed`.
