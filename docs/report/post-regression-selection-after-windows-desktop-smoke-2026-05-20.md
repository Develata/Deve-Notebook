# Post-regression Selection After Windows Desktop Smoke - 2026-05-20

本报告记录 Windows Desktop smoke 闭合后的本机 guard refresh、发现的脚本问题、最小修复与下一步 evidence 路线选择。`docs/plan/` 未修改。

## Scope

- Baseline commit before this work: `44fa7e35 Fix Windows regression gates and desktop smoke`
- Host: Windows / Git Bash target host, Windows version `10.0.26200`
- Non-goal / kept closed: signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。
- Local-first rule: Windows/WSL2 + Android Studio/emulator 承接主线回归、Windows Desktop 与 Android evidence；Codex Cloud 只用于补足 macOS / iOS / Apple target-host evidence。

## Commands

All shell gates were run through Git Bash:

```powershell
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "<command>"
```

Validated commands:

```bash
./scripts/check-acceptance-bindings.sh
./scripts/check-feature-operation-paths.sh
./scripts/check-architecture-registry.sh
./scripts/plan-coverage.sh
./scripts/check-release-baseline.sh
./scripts/check-release-audit-gate.sh
./scripts/check-foundation-baseline.sh
./scripts/check-network-baseline.sh
./scripts/check-auth-baseline.sh
./scripts/check-rendering-baseline.sh
./scripts/check-storage-repo-baseline.sh
./scripts/check-source-control-baseline.sh
./scripts/check-repo-file-ops-baseline.sh
./scripts/check-cli-settings-baseline.sh
./scripts/check-settings-local-feedback-baseline.sh
./scripts/check-ai-baseline.sh
./scripts/check-search-baseline.sh
./scripts/check-graph-baseline.sh
./scripts/check-large-doc-baseline.sh
./scripts/check-dev-runbook-baseline.sh
./scripts/check-dev-data-health-baseline.sh
./scripts/check-diff-color-baseline.sh
./scripts/check-i18n-formatting-baseline.sh
./scripts/check-i18n-hardcoded-baseline.sh
./scripts/check-ui-dashboard-refresh-baseline.sh
./scripts/check-ui-desktop-baseline.sh
./scripts/check-ui-disconnect-baseline.sh
./scripts/check-ui-focus-baseline.sh
./scripts/check-ui-spa-routing-baseline.sh
./scripts/check-ui-token-baseline.sh
./scripts/smoke-web-release-build.sh
./scripts/check-ui-z-index-baseline.sh
./scripts/check-mobile-baseline.sh
./scripts/check-native-target-host-evidence.sh
./scripts/check-native-process-adapter-gate.sh
./scripts/check-native-packaging-gate.sh
```

## Results

Passed:

- Acceptance bindings: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.
- Feature operation paths: `feature-operation-path-check: ok`.
- Architecture registry: `architecture-registry-check: ok (72 flows, 0 active drift)`.
- Plan coverage: blocking violations `0`, dangling refs `0`, i18n leaks `0`; soft warnings remain informational.
- Release audit gate: `found 0 vulnerabilities`; `cargo audit` remained unavailable and was skipped by script.
- Domain/UI/native/mobile baselines listed above passed after the fixes below.
- Web release build smoke passed and refreshed local untracked `apps/web/js/editor.bundle.*`.
- Native process adapter gate and native packaging gate passed; process runtime and native authority writes remain closed.

## Bugs Fixed

1. `scripts/check-acceptance-bindings.sh` used recursive `grep` over `apps/`, which traversed generated Android/Tauri outputs and timed out on this Windows host. It now uses `rg --fixed-strings` for the code binding scan.
2. `scripts/plan-coverage.sh` used filesystem `find` over generated source trees and per-file `wc -l`, which made Windows Git Bash runs impractically slow. It now scans tracked Rust files from `git ls-files` and batches line counting.
3. Several baseline scripts passed fixed strings beginning with `/api/...` or `/path/...` to Windows `rg` under MSYS2. Git Bash path conversion rewrote those patterns before `rg` saw them, producing false missing-text failures. The affected scripts now set `MSYS2_ARG_CONV_EXCL` narrowly for the fixed-string pattern:
   - `scripts/check-release-baseline.sh`
   - `scripts/check-source-control-baseline.sh`
   - `scripts/check-ai-baseline.sh`
   - `scripts/check-graph-baseline.sh`
   - `scripts/check-dev-runbook-baseline.sh`
4. `scripts/check-ui-z-index-baseline.sh` initially failed because local untracked editor bundle outputs still contained stale `z-50` / `z-20` strings. Source checks were already clean; `scripts/smoke-web-release-build.sh` refreshed the generated bundle, after which the z-index gate passed. No generated bundle was committed.

## Skipped / Deferred

- macOS Desktop target-host package/startup/installer evidence: deferred to Apple-capable Codex Cloud.
- iOS shell package/install/startup evidence: deferred to Apple-capable Codex Cloud.
- Signing, store, physical-device readiness, native authority writes, Mobile process runtime, Android process runtime, Web Git writer, server-backed Settings API: still closed.

## Selection

No new unblocked Current Web/server `MUST` implementation gap was found in this pass. Windows Desktop evidence is current on this host, and Android evidence is available from the local Android Studio/emulator route. The next evidence step should be Apple-only:

1. Use Codex Cloud only for macOS / iOS target-host evidence refresh.
2. After the cloud report is committed or returned, pull it locally and run the target-host evidence validator plus the relevant local regression gates.
3. Keep normal feature work, full regression, Windows Desktop evidence and Android evidence on this local machine unless Apple hardware/tooling is required.
