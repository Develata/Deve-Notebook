# Mobile iOS Simulator Install/Startup Smoke - 2026-05-15

## Scope

Record Mobile iOS simulator install/startup target-host evidence for the shell-only Mobile app.

This report does not declare iOS device release readiness or open native process runtime.

## Run

- GitHub run: `25926372319`
- Workflow: `native-target-host.yml`
- Target: `mobile-ios`
- Commit: `ebf8e1963b7f0632decf1fc30ba7a8396957fd9c`
- Evidence artifact: `deve-native-target-host-evidence-ios`
- Local evidence copy: `target/native-target-host-evidence-download/deve-native-target-host-evidence-ios/mobile-ios.md`

## Evidence

- Host: macOS 15.7.4 arm64.
- Rust: `1.92.0`.
- Tauri CLI: `2.11.1`.
- Node: `24.15.0`.
- npm: `11.12.1`.
- Xcode: `16.4`.

Command results:

- `mobile_ios_preflight=success`
- `process_gate=success`
- `package_build=success`
- `install_startup_smoke=success`
- `package_closed_gate=skipped`

## Result

Mobile iOS simulator shell package build, install, and startup smoke are closed for the current shell-only boundary.

The evidence preserves:

- process runtime gate: closed;
- native authority writes: closed;
- no backend supervision ownership;
- no ledger/vault/source-control/search/Git/`.notegit` native write path.

## Remaining

iOS device signing, notarization-equivalent distribution decisions, and real device install/startup remain separate future gates.

Android emulator/device install/startup evidence remains open.
