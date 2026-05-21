#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
REQUIRED="${DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED:-0}"
TARGET="${DEVE_MOBILE_ANDROID_PACKAGE_TARGET:-aarch64}"
BUILD_APK="${DEVE_MOBILE_ANDROID_PACKAGE_APK:-1}"
BUILD_AAB="${DEVE_MOBILE_ANDROID_PACKAGE_AAB:-0}"
BUILD_DEBUG="${DEVE_MOBILE_ANDROID_PACKAGE_DEBUG:-0}"

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
  if [[ "$BUILD_DEBUG" == "1" && "$BUILD_AAB" == "1" ]]; then
    fail "debug Android install-smoke builds must produce APK only; AAB is release/store packaging"
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

configure_gradle_proxy_from_env() {
  [[ "${DEVE_MOBILE_ANDROID_GRADLE_PROXY_FROM_ENV:-1}" == "1" ]] || return 0
  [[ "${GRADLE_OPTS:-}" != *".proxyHost="* ]] || return 0

  local proxy_url="${HTTPS_PROXY:-${https_proxy:-${HTTP_PROXY:-${http_proxy:-}}}}"
  [[ -n "$proxy_url" ]] || return 0

  local scheme auth host port
  if [[ "$proxy_url" =~ ^([A-Za-z][A-Za-z0-9+.-]*)://([^/@]+@)?([^/:]+):([0-9]+)(/.*)?$ ]]; then
    scheme="${BASH_REMATCH[1]}"
    auth="${BASH_REMATCH[2]}"
    host="${BASH_REMATCH[3]}"
    port="${BASH_REMATCH[4]}"
  else
    echo "mobile-android-shell-package-build-check: skip Gradle proxy autoconfig; unsupported proxy URL format" >&2
    return 0
  fi

  case "$scheme" in
    http|https) ;;
    *)
      echo "mobile-android-shell-package-build-check: skip Gradle proxy autoconfig; unsupported proxy scheme: $scheme" >&2
      return 0
      ;;
  esac

  if [[ -n "$auth" ]]; then
    echo "mobile-android-shell-package-build-check: skip Gradle proxy autoconfig; authenticated proxy is not encoded into GRADLE_OPTS" >&2
    return 0
  fi

  export GRADLE_OPTS="${GRADLE_OPTS:+$GRADLE_OPTS }-Dhttp.proxyHost=$host -Dhttp.proxyPort=$port -Dhttps.proxyHost=$host -Dhttps.proxyPort=$port"
  echo "mobile-android-shell-package-build-check: Gradle proxy configured from HTTPS_PROXY/HTTP_PROXY"
}

configure_android_build_java() {
  android_prepare_java_home \
    || fail "java >=17 or Android Studio JBR is required for Android shell package build"
}

configure_kotlin_incremental_workaround() {
  case "$(uname -s 2>/dev/null || printf 'unknown')" in
    MINGW*|MSYS*|CYGWIN*) ;;
    *) return 0 ;;
  esac
  [[ "${DEVE_MOBILE_ANDROID_DISABLE_KOTLIN_INCREMENTAL_ON_WINDOWS:-1}" == "1" ]] || return 0
  [[ "${GRADLE_OPTS:-}" != *"kotlin.incremental=false"* ]] || return 0

  export GRADLE_OPTS="${GRADLE_OPTS:+$GRADLE_OPTS }-Dkotlin.incremental=false"
  echo "mobile-android-shell-package-build-check: Kotlin incremental disabled for Windows cross-drive Gradle sources"
}

validate_target
validate_artifact_kind
assert_android_shell_boundary
configure_android_build_java
configure_kotlin_incremental_workaround

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

configure_gradle_proxy_from_env

if [[ ! -d "$ROOT_DIR/apps/mobile/gen/android" ]]; then
  (
    cd "$ROOT_DIR/apps/mobile"
    run cargo tauri android init --ci --skip-targets-install
  )
fi

build_args=(cargo tauri android build --ci --features native-packaging --target "$TARGET")
if [[ "$BUILD_DEBUG" == "1" ]]; then
  build_args+=(--debug)
fi
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
