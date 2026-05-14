# Desktop AppImage Package Verification

Date: 2026-05-14

## Scope

Verified the Linux AppImage target-host package path for the Desktop `native-packaging` shell.

No real backend child-process runtime was opened.

## Result

- Desktop package preflight passed.
- Tauri release binary build passed.
- Linux AppImage bundling passed with host AppImage extraction mode and `librsvg-2.0` pkg-config metadata present.
- Generated artifact: `target/release/bundle/appimage/Deve Notebook_0.0.1_amd64.AppImage`.
- Artifact size observed: `73447928` bytes.
- AppImage runtime metadata responded to `--appimage-help`.

## Host Findings

- This WSL host has `/dev/fuse` and `libfuse3`, but not `libfuse.so.2`.
- Cached `linuxdeploy-x86_64.AppImage` requires FUSE unless run through AppImage extraction mode.
- Tauri invokes `linuxdeploy --appimage-extract-and-run`, so the remaining blocking prerequisite was `pkg-config librsvg-2.0`.
- Non-interactive sudo is unavailable on this host, so `librsvg2-dev` could not be installed system-wide during this run.
- A temporary `PKG_CONFIG_PATH` shim for `librsvg-2.0.pc` was used to verify the package path without modifying the host or repository.

## Verification

- `scripts/check-desktop-platform-package-build.sh`
- `APPIMAGE_EXTRACT_AND_RUN=1 ~/.cache/tauri/linuxdeploy-x86_64.AppImage --version`
- `APPIMAGE_EXTRACT_AND_RUN=1 ~/.cache/tauri/linuxdeploy-plugin-appimage.AppImage --plugin-type`
- `APPIMAGE_EXTRACT_AND_RUN=1 PKG_CONFIG_PATH=/tmp/deve-pkgconfig-... DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=appimage scripts/check-desktop-platform-package-build.sh`
- `APPIMAGE_EXTRACT_AND_RUN=1 target/release/bundle/appimage/Deve\ Notebook_0.0.1_amd64.AppImage --appimage-help`

## Follow-up

- `scripts/check-desktop-platform-package-build.sh` now fails early in required AppImage/all-bundle mode when `pkg-config librsvg-2.0` is missing.
- `docs/dev-runbook.md` now records the Linux AppImage host prerequisite.
- macOS and Windows package/signing readiness remain unverified until run on their target hosts.
