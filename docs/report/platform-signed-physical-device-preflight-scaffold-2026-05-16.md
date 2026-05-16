# Platform Signed / Physical-device Preflight Scaffold - 2026-05-16

本报告记录 Desktop signing 与 Android signed/physical-device preflight scaffold。`docs/plan/` 未修改。

## Scope

- Implements the next task selected in `docs/report/platform-post-gate-scope-decision-2026-05-16.md`.
- Adds diagnostic/required preflight gates only.
- Does not sign artifacts, notarize artifacts, upload stores, install on physical devices, open native process runtime, or add native authority writes.

## Changes

- Added `scripts/check-desktop-signing-preflight.sh`.
- Added `scripts/check-mobile-android-release-preflight.sh`.
- Added both scripts to release workflow diagnostic gates.
- Bound both scripts into `REL-005`.
- Documented required environment shape in `docs/dev-runbook.md`.
- Extended `scripts/check-release-baseline.sh` to guard the new release boundary.

## Desktop Signing Preflight

Default mode:

- Reports missing macOS/Windows signing prerequisites.
- Exits successfully when prerequisites are absent.
- Does not sign or notarize.

Required mode:

- `DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED=1`
- `DEVE_DESKTOP_SIGNING_TARGETS=macos|windows`
- Fails closed when target host tools or signing material are absent.

macOS checks:

- Darwin target host.
- `codesign`.
- `xcrun notarytool`.
- `APPLE_SIGNING_IDENTITY`.
- `APPLE_PROVIDER_SHORT_NAME`.
- Either `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` or `APPLE_API_KEY` / `APPLE_API_KEY_ID` / `APPLE_API_ISSUER`.

Windows checks:

- Windows target host.
- `signtool`.
- `WINDOWS_SIGNING_CERT_PATH` or `WINDOWS_SIGNING_CERT_BASE64`.
- `WINDOWS_SIGNING_CERT_PASSWORD`.

## Android Release / Physical-device Preflight

Default mode:

- Reports missing Android signing and physical-device prerequisites.
- Exits successfully when prerequisites are absent.
- Does not sign, build release artifacts, install, or upload.

Required signing mode:

- `DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED=1`
- Fails closed when keystore or key metadata are absent.

Required physical-device mode:

- `DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED=1`
- Fails closed when no non-emulator `adb` target is attached.
- `DEVE_MOBILE_ANDROID_SERIAL=<adb-serial>` can pin a specific device.

Android signing checks:

- `DEVE_ANDROID_KEYSTORE_PATH` or `DEVE_ANDROID_KEYSTORE_BASE64`.
- `DEVE_ANDROID_KEY_ALIAS`.
- `DEVE_ANDROID_KEYSTORE_PASSWORD`.
- `DEVE_ANDROID_KEY_PASSWORD`.
- `DEVE_MOBILE_ANDROID_RELEASE_ARTIFACT_KIND=apk|aab`.

## Verification

Ran:

- `bash -n scripts/check-desktop-signing-preflight.sh scripts/check-mobile-android-release-preflight.sh`
- `shellcheck scripts/check-desktop-signing-preflight.sh scripts/check-mobile-android-release-preflight.sh`
- `scripts/check-desktop-signing-preflight.sh`
- `scripts/check-mobile-android-release-preflight.sh`
- `DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_SIGNING_TARGETS=macos scripts/check-desktop-signing-preflight.sh` as expected-failure negative test.
- `DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED=1 DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK=0 scripts/check-mobile-android-release-preflight.sh` as expected-failure negative test.
- `DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED=1 DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK=0 scripts/check-mobile-android-release-preflight.sh` as expected-failure negative test.
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `git diff --check`

Results:

- Diagnostic mode passes without private material.
- Required mode fails closed when prerequisites are missing.
- Release/dev-runbook guards pass.
- No native process runtime or authority write path was opened.

## Next

Real signing or physical-device execution now requires an explicit target choice and external material:

- macOS signing/notarization.
- Windows signed installer.
- Android signed APK/AAB.
- Android physical-device smoke.

Without those credentials or target hosts, the safe next step is to return to mainline feature implementation selection.
