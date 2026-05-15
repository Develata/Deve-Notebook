# Native Desktop Package Artifacts

Date: 2026-05-15

## Scope

This report records target-host Desktop package artifact generation for macOS
and Windows.

No `docs/plan/` contract changed in this batch.

## macOS

- Workflow: `Native Target Host`
- Run: https://github.com/Develata/Deve-Notebook/actions/runs/25910438045
- Head: `a379e614691348a8285aeb4d670550c8085531d7`
- Mode: `target=desktop-macos`, `required_preflight=false`, `run_desktop_package_build=true`
- Result: success
- Command: `DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=app,dmg DEVE_DESKTOP_PACKAGE_NO_SIGN=1 scripts/check-desktop-platform-package-build.sh`

Generated artifacts:

- `target/release/bundle/macos/Deve Notebook.app`
- `target/release/bundle/dmg/Deve Notebook_0.0.1_aarch64.dmg`

Downloaded local evidence:

- `target/native-target-host-package-download/macos-25910438045/macos/Deve Notebook.app/Contents/MacOS/deve_desktop`
- `target/native-target-host-package-download/macos-25910438045/dmg/Deve Notebook_0.0.1_aarch64.dmg`
- `target/native-target-host-evidence-download/macos-25910438045/desktop-macos.md`

## Windows

- Workflow: `Native Target Host`
- Run: https://github.com/Develata/Deve-Notebook/actions/runs/25869574029
- Head: `d7faa1ef948df811cd41671682aefa3d463f84ef`
- Mode: `target=all`, `required_preflight=false`, `run_desktop_package_build=true`
- Job result: `desktop-windows` success
- Run result: failure because the same run contained an older macOS package failure
- Command: `DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-platform-package-build.sh`

Generated artifacts:

- `target/release/bundle/msi/Deve Notebook_0.0.1_x64_en-US.msi`
- `target/release/bundle/nsis/Deve Notebook_0.0.1_x64-setup.exe`

Downloaded local evidence:

- `target/native-target-host-package-download/windows-25869574029/msi/Deve Notebook_0.0.1_x64_en-US.msi`
- `target/native-target-host-package-download/windows-25869574029/nsis/Deve Notebook_0.0.1_x64-setup.exe`
- `target/native-target-host-evidence-download/windows-25869574029/desktop-windows.md`

## Boundary

- macOS artifact generation used `DEVE_DESKTOP_PACKAGE_NO_SIGN=1`; it validates
  package shape, not signed/notarized release readiness.
- Windows artifact generation produced MSI and NSIS installers, but install and
  startup smoke remain unverified.
- Process runtime gate remained closed.
- Native authority writes remained closed.
- Mobile iOS package execution remains closed.

## Fixes

- Added `apps/desktop/icons/icon.icns` and registered it in
  `apps/desktop/tauri.conf.json`.
- Restricted Linux AppImage `librsvg-2.0` prerequisite checks to Linux hosts.
- Added macOS package build inputs for `app,dmg` and unsigned CI smoke mode.

## Verification

- `scripts/check-native-track-boundary.sh`
- `cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture`
- `scripts/check-desktop-platform-package-build.sh`
- GitHub macOS package run `25910438045`: success
- GitHub Windows package job `76020818573`: success
