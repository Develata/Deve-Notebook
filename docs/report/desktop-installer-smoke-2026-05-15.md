# Desktop Installer Smoke - 2026-05-15

## Scope

Verify Desktop target-host package build, packaged startup smoke, and installer install/uninstall smoke for macOS and Windows.

This report does not open native process runtime, ledger authority, vault authority, source-control authority, Git authority, or `.notegit` authority to native shells.

## Evidence

### macOS

- Workflow run: `25921302704`
- URL: https://github.com/Develata/Deve-Notebook/actions/runs/25921302704
- Head: `99b19f26e3ea18af4ed0071df7438852c7b574c1`
- Host: macOS `15.7.5` arm64
- Toolchain: Rust `1.92.0`, Tauri CLI `2.11.1`, Node `24.15.0`, npm `11.12.1`, Xcode `16.4`
- Bundles: `app,dmg`
- Results: `desktop_preflight=success`, `process_gate=success`, `package_build=success`, `startup_smoke=success`, `installer_smoke=success`

### Windows

- Workflow run: `25924163007`
- URL: https://github.com/Develata/Deve-Notebook/actions/runs/25924163007
- Head: `c93ef4aaa94857c77fe421a0cd464b9180816320`
- Host: `Windows_NT` / `MINGW64_NT-10.0-26100`
- Toolchain: Rust `1.92.0`, Tauri CLI `2.11.1`, Node `24.15.0`, npm `11.12.1`
- Bundles: `msi,nsis`
- Results: `desktop_preflight=success`, `process_gate=success`, `package_build=success`, `startup_smoke=success`, `installer_smoke=success`

## Fixes In This Batch

- `c93ef4aa` replaced inline native-target-host web build commands with `scripts/build-web-dist-ci.sh`.
- The CI wrapper resolves `npm`/`npm.cmd` and `trunk`/`trunk.exe` explicitly and prints command diagnostics.
- `scripts/smoke-web-release-build.sh` now accepts `DEVE_TRUNK_BIN` while preserving default `trunk` behavior.
- Windows `Build web dist` now passes on GitHub Actions; the previous immediate `exit 127` failure is closed.

## Boundary

- Process runtime gate: closed.
- Native authority writes: closed.
- Desktop installer smoke does not imply Android/iOS install readiness.
- Desktop installer smoke does not authorize `Command::new`, background process supervision, or native child-process runtime.

## Verification

- `scripts/build-web-dist-ci.sh`
- `bash -n scripts/build-web-dist-ci.sh scripts/smoke-web-release-build.sh scripts/check-release-baseline.sh scripts/install-native-target-host-tools.sh`
- `shellcheck scripts/build-web-dist-ci.sh scripts/smoke-web-release-build.sh scripts/install-native-target-host-tools.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-native-process-adapter-gate.sh`
- `scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`
- `scripts/check-native-target-host-evidence.sh target/native-target-host-evidence-download/deve-native-target-host-evidence-macos/desktop-macos.md`
- `scripts/check-native-target-host-evidence.sh target/native-target-host-evidence-download/deve-native-target-host-evidence-windows/native-target-host-evidence/desktop-windows.md`
- `DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 DEVE_NATIVE_TARGET_HOST_RUN_ID=25924163007 DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-windows scripts/collect-native-target-host-evidence.sh`

## Result

Desktop macOS and Windows installer install/uninstall smoke is closed for the current shell-only native packaging boundary.
