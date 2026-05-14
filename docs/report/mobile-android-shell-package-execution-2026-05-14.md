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
- Android target-host package script now auto-adapts Java/Gradle proxy settings from `HTTPS_PROXY` / `HTTP_PROXY` when `GRADLE_OPTS` does not already define a proxy.
- APK assemble is closed for the Android shell-only path on this target host.

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
- `GRADLE_OPTS='-Dhttp.proxyHost=127.0.0.1 -Dhttp.proxyPort=10808 -Dhttps.proxyHost=127.0.0.1 -Dhttps.proxyPort=10808' ./gradlew --no-daemon --refresh-dependencies --stacktrace --info tasks`

Earlier required Android build reached `target/aarch64-linux-android/release/libdeve_mobile.so` and symlinked it into `jniLibs`, then failed while resolving `org.gradle.kotlin.kotlin-dsl:5.2.0`.

Root cause: `curl` used the shell proxy environment, but Java/Gradle did not inherit that proxy as JVM system properties. With equivalent `GRADLE_OPTS`, Gradle resolved `kotlin-dsl`, compiled `buildSrc`, configured `:app` and `:tauri-android`, and listed Android/Rust assemble tasks successfully.

After adding script-level proxy adaptation, required mode completed `cargo tauri android build --ci --features native-packaging --target aarch64 --apk` and produced:

- `apps/mobile/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`

## Next

- Keep iOS package execution blocked until a macOS target host and separate acceptance path are available.
- Keep child-process runtime blocked until target-host package execution evidence is complete.
