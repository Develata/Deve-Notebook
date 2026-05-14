# Mobile Android Shell Package Execution

Date: 2026-05-14

## Scope

- Implemented the Android shell-only package execution gate opened by `08_ui_design_03_mobile#mobile-android-shell-package-execution-gate`.
- Added a feature-gated Tauri mobile WebView shell entrypoint in `apps/mobile`.
- Added an explicit Android target-host script: `scripts/check-mobile-android-shell-package-build.sh`.
- Generated the Android shell project under `apps/mobile/gen/android`.

## Boundary

- Default `deve_mobile` build remains no-Tauri.
- Android package execution remains behind `apps/mobile/native-packaging`.
- The mobile Tauri entrypoint starts only the WebView shell.
- No backend child-process runtime was opened.
- No native ledger, vault, source-control, search, `.git`, or `.notegit` write authority was opened.
- iOS package execution remains closed.
- APK assemble is not yet closed: Rust Android `.so` generation succeeds, but Gradle dependency resolution currently stops at `org.gradle.kotlin.kotlin-dsl:5.2.0`.

## Verification

- `cargo check --locked -p deve_mobile --no-default-features`
- `cargo check --locked -p deve_mobile --features native-packaging`
- `cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-mobile-platform-package-preflight.sh`
- `scripts/check-mobile-android-shell-package-build.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 scripts/check-mobile-android-shell-package-build.sh`

The required Android build reached `target/aarch64-linux-android/release/libdeve_mobile.so` and symlinked it into `jniLibs`. APK assemble then failed in Gradle while resolving `org.gradle.kotlin.kotlin-dsl:5.2.0` from the Gradle Plugin Portal. Direct `curl` to the plugin POM succeeds, so the remaining blocker is Gradle dependency resolution/cache on this target host, not the Rust mobile shell entrypoint.

## Next

- Close Gradle dependency resolution for the generated Android project, then rerun `DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 scripts/check-mobile-android-shell-package-build.sh`.
- Keep iOS package execution blocked until a macOS target host and separate acceptance path are available.
- Keep child-process runtime blocked until target-host package execution evidence is complete.
