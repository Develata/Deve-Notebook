# Platform Post-Gate Scope Decision - 2026-05-16

本报告记录 current-head shell-only platform evidence 通过后的 post-gate 范围选择。`docs/plan/` 未修改。

## Inputs

- `docs/plan/08_ui_design_02_desktop.md`
- `docs/plan/08_ui_design_03_mobile.md`
- `docs/plan/15_release.md`
- `docs/report/current-head-platform-evidence-refresh-2026-05-16.md`
- `docs/report/mainline-gap-scan-after-current-head-platform-evidence-2026-05-16.md`

## Current Evidence

- Docker release smoke: green on current-head evidence run.
- Desktop macOS: package build, startup smoke, installer smoke green.
- Desktop Windows: package build, startup smoke, installer smoke green.
- Android: emulator install/startup smoke green.
- iOS: simulator install/startup smoke green.
- Native process runtime: closed.
- Native authority writes: closed.

## Candidate Gates

| Candidate | External prerequisite | Risk | Decision |
| --- | --- | --- | --- |
| Desktop macOS signing/notarization | Apple Developer account, signing identity, provider short name, notarization credentials | High external dependency | Not first implementation batch |
| Desktop Windows signed installer | Code signing certificate, signing tool availability, certificate storage policy | High external dependency | Not first implementation batch |
| Android signed release | Keystore, key alias/password, release artifact policy | Medium external dependency | Prepare preflight scaffold first |
| Android physical-device smoke | Real attached device or managed target host | Host-dependent | Prepare preflight scaffold first |
| iOS signing/TestFlight/device | Apple Developer account, provisioning profile, device/team setup | Highest external dependency | Defer |
| Native process runtime | Explicit runtime feature and process supervision acceptance | Architecture-sensitive | Keep closed |

## Decision

The next executable batch should be **Platform Signed / Physical-device Preflight Scaffold**.

This batch should add diagnostic, fail-closed gates before any real signing or device release path:

- Desktop signing preflight:
  - detect macOS signing/notarization env shape without requiring secrets in normal mode;
  - detect Windows signing prerequisites without requiring a certificate in normal mode;
  - required mode fails closed when requested secrets/tools are absent.
- Android signed/physical-device preflight:
  - detect keystore/env shape for signed release;
  - detect attached device or required target-host state for physical-device smoke;
  - default mode remains diagnostic-only.
- Release/dev-runbook/acceptance bindings:
  - document that these are prerequisite gates only;
  - keep signed release, store release, physical-device readiness, native process runtime, and native authority writes closed until explicitly opened.

## Non-Goals

- Do not sign macOS, Windows, Android, or iOS artifacts in this batch.
- Do not upload to Play Store, TestFlight, App Store, GitHub Release, or package registries.
- Do not require secrets in normal local or CI runs.
- Do not open child-process runtime.
- Do not introduce native authority write paths.

## Rationale

Direct signing/store/device implementation would be coupled to external accounts and private material. A preflight scaffold converts those dependencies into explicit gates, keeps default CI deterministic, and gives a safe next step toward Desktop/Android release readiness.
