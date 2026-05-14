# Desktop Target-Host Package Build Verification

Date: 2026-05-14

## Scope

- Active queue item: Desktop target-host package build verification.
- Source of truth: `docs/plan/08_ui_design_02_desktop.md` and `docs/plan/14_tech_stack.md`.
- This batch does not modify `docs/plan/`.

## Result

- Installed `tauri-cli 2.11.1` on the local host.
- Verified `cargo tauri --version` reports `tauri-cli 2.11.1`.
- Verified Linux `deb,rpm` package build through:

```bash
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=deb,rpm scripts/check-desktop-platform-package-build.sh
```

## Artifacts

- `target/release/deve_desktop`
- `target/release/bundle/deb/Deve Notebook_0.0.1_amd64.deb`
- `target/release/bundle/rpm/Deve Notebook-0.0.1-1.x86_64.rpm`

## Manifest Note

- `cargo tauri build` materialized explicit empty `features = []` on the optional
  `tauri` and `tauri-build` dependency declarations.
- This does not change the dependency graph; it makes the native-packaging gate
  depend on semantic manifest checks instead of exact line text.

## Residual

- Full default Linux bundle set attempted `deb`, `rpm`, and `appimage`.
- `deb` and `rpm` were created before AppImage bundling failed at `linuxdeploy`.
- AppImage remains a target-host-specific packaging residual; it is not a Desktop runtime code defect.
- macOS and Windows package/signing readiness remain unverified on this Linux host.
- Tauri emitted a `__TAURI_BUNDLE_TYPE` updater metadata warning while creating
  `deb` and `rpm`; updater artifacts are disabled by `tauri.conf.json`, so this
  remains a release-chain observation, not a current shell package blocker.

## Boundary

- No child-process runtime was opened.
- No native authority write path was added.
- Desktop package build verifies shell packaging only; it does not certify backend process supervision or offline native app readiness.

## Verification

Run:

```bash
cargo tauri --version
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=deb,rpm scripts/check-desktop-platform-package-build.sh
scripts/check-release-baseline.sh
scripts/check-dev-runbook-baseline.sh
scripts/check-native-packaging-gate.sh
scripts/plan-coverage.sh --summary-missing-plan-ref
cargo fmt --check
git diff --check
```
