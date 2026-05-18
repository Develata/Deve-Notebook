# Mainline Gap Rescan After Mobile iOS Target-host Evidence Refresh - 2026-05-18

本报告记录 Mobile iOS shell-only target-host evidence refresh 后的主线缺口复扫。`docs/plan/` 未修改。

## Scope

- 唯一真源：`docs/plan/`。
- 输入证据：Mobile iOS shell-only target-host evidence、Android shell-only evidence、Desktop installer/native-session evidence、最近 full regression gate、features、acceptance cases 与 guard scripts。
- 本批目标：确认 Current Web/server 与 shell-only platform boundary 是否出现新的 unblocked `MUST` gap。
- 非目标：不打开 signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer 或 server-backed Settings API。

## Verification

- `bash scripts/check-acceptance-bindings.sh`：automated `149`，feature walkthrough `54`，manual `0`，unbound `0`。
- `bash scripts/check-feature-operation-paths.sh`：通过。
- `bash scripts/check-architecture-registry.sh`：`72` flows，active drift `0`。
- `bash scripts/check-release-baseline.sh`：通过。
- `bash scripts/plan-coverage.sh`：blocking `0`，soft warnings `27`。
- `bash scripts/check-native-target-host-evidence.sh target/native-target-host-evidence-download-26026170256/deve-native-target-host-evidence-ios/mobile-ios.md target/native-target-host-evidence-download-26023546240/deve-native-target-host-evidence-android/native-target-host-evidence/mobile-android.md`：通过。
- Domain baselines：foundation、network、auth、rendering、storage/repo、source-control、CLI settings、settings local feedback、AI、search、graph、large-doc、dev-runbook、dev-data-health、diff-color、i18n formatting、i18n hardcoded 均通过。
- UI baselines：dashboard refresh、desktop、disconnect、focus、SPA routing、token、z-index 均通过。
- Native/mobile gates：mobile baseline、native process adapter、native packaging、mobile platform preflight、iOS shell package default diagnostic、iOS install/startup default diagnostic 均通过。
- Runtime smoke：happy path 与 recovery path 均通过。

## Findings

- Mobile iOS shell-only target-host evidence 当前有效：workflow run `26026170256` 在 commit `33ae0fa8` 上通过，package build、simulator install/startup smoke、process gate 与 evidence validator 均为 green。
- Android shell-only target-host evidence 仍有效：workflow run `26023546240` 在 commit `699e5bbd` 上通过，package build、emulator install/startup smoke、process gate 与 evidence validator 均为 green。
- Desktop target-host installer/native-session evidence 仍保持 green；本批未发现该证据与当前 Web/server 骨架之间的新冲突。
- Native packaging gate 在本机 Linux 继续报告预期 appindicator/EGL host warning，并按脚本合同跳过本地 package smoke 后完成 `ok`；macOS / Windows target-host evidence 仍是 Desktop package/install authority。
- Current Web/server 主线未发现新的 unblocked `MUST` gap。
- Mobile process runtime、Android process runtime、native authority writes、signing、store、physical-device readiness、Web Git writer 与 server-backed Settings API 仍保持关闭。

## Decision

- 本批 gap rescan 闭合。
- 下一批进入 full regression gate refresh，确认 iOS evidence closure 后的全仓库健康状态，再选择新的功能或平台实现目标。
- 本批未修改 `docs/plan/`。
