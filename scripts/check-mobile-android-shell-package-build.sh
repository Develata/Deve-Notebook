#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED:-0}"
TARGET="${DEVE_MOBILE_ANDROID_PACKAGE_TARGET:-aarch64}"
BUILD_APK="${DEVE_MOBILE_ANDROID_PACKAGE_APK:-1}"
BUILD_AAB="${DEVE_MOBILE_ANDROID_PACKAGE_AAB:-0}"

# This gate builds only the Android WebView shell; it must not open child-process runtime.

fail() {
  echo "mobile-android-shell-package-build-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

validate_target() {
  case "$TARGET" in
    aarch64|armv7|i686|x86_64) ;;
    *) fail "unsupported Android target: $TARGET" ;;
  esac
}

validate_artifact_kind() {
  if [[ "$BUILD_APK" != "1" && "$BUILD_AAB" != "1" ]]; then
    fail "at least one of DEVE_MOBILE_ANDROID_PACKAGE_APK or DEVE_MOBILE_ANDROID_PACKAGE_AAB must be 1"
  fi
}

assert_android_shell_boundary() {
  [[ ! -e "$ROOT_DIR/apps/mobile/gen/apple" ]] \
    || fail "iOS generated project is not allowed in the Android shell package gate"
  [[ ! -e "$ROOT_DIR/apps/mobile/src-tauri" ]] \
    || fail "legacy src-tauri layout is not allowed for apps/mobile"
  [[ ! -e "$ROOT_DIR/apps/mobile/src/main.rs" ]] \
    || fail "mobile shell must expose the Tauri mobile entrypoint from lib.rs, not src/main.rs"
}

validate_target
validate_artifact_kind
assert_android_shell_boundary

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

if [[ "$REQUIRED" != "1" ]]; then
  DEVE_MOBILE_PACKAGE_TARGETS=android \
    DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=0 \
    run "$ROOT_DIR/scripts/check-mobile-platform-package-preflight.sh"
  echo "mobile-android-shell-package-build-check: build not executed; set DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 on an Android target host"
  echo "mobile-android-shell-package-build-check: ok"
  exit 0
fi

DEVE_MOBILE_PACKAGE_TARGETS=android \
  DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 \
  run "$ROOT_DIR/scripts/check-mobile-platform-package-preflight.sh"

if [[ ! -d "$ROOT_DIR/apps/mobile/gen/android" ]]; then
  (
    cd "$ROOT_DIR/apps/mobile"
    run cargo tauri android init --ci --skip-targets-install
  )
fi

build_args=(cargo tauri android build --ci --features native-packaging --target "$TARGET")
if [[ "$BUILD_APK" == "1" ]]; then
  build_args+=(--apk)
fi
if [[ "$BUILD_AAB" == "1" ]]; then
  build_args+=(--aab)
fi

(
  cd "$ROOT_DIR/apps/mobile"
  run "${build_args[@]}"
)

assert_android_shell_boundary
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

echo "mobile-android-shell-package-build-check: ok"
