#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED:-0}"
TARGET="${DEVE_MOBILE_IOS_PACKAGE_TARGET:-aarch64-sim}"

# This gate builds only the iOS WebView shell; it must not open child-process runtime.

fail() {
  echo "mobile-ios-shell-package-build-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

validate_target() {
  case "$TARGET" in
    aarch64|aarch64-sim|x86_64) ;;
    *) fail "unsupported iOS target: $TARGET" ;;
  esac
}

assert_ios_shell_boundary() {
  [[ ! -e "$ROOT_DIR/apps/mobile/src-tauri" ]] \
    || fail "legacy src-tauri layout is not allowed for apps/mobile"
  [[ ! -e "$ROOT_DIR/apps/mobile/src/main.rs" ]] \
    || fail "mobile shell must expose the Tauri mobile entrypoint from lib.rs, not src/main.rs"
}

validate_target
assert_ios_shell_boundary

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

if [[ "$REQUIRED" != "1" ]]; then
  DEVE_MOBILE_PACKAGE_TARGETS=ios \
    DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=0 \
    run "$ROOT_DIR/scripts/check-mobile-platform-package-preflight.sh"
  echo "mobile-ios-shell-package-build-check: build not executed; set DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED=1 on a macOS target host"
  echo "mobile-ios-shell-package-build-check: ok"
  exit 0
fi

DEVE_MOBILE_PACKAGE_TARGETS=ios \
  DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 \
  run "$ROOT_DIR/scripts/check-mobile-platform-package-preflight.sh"

if [[ ! -d "$ROOT_DIR/apps/mobile/gen/apple" ]]; then
  (
    cd "$ROOT_DIR/apps/mobile"
    run cargo tauri ios init --ci --skip-targets-install
  )
fi

(
  cd "$ROOT_DIR/apps/mobile"
  run cargo tauri ios build --ci --features native-packaging --target "$TARGET"
)

assert_ios_shell_boundary
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

echo "mobile-ios-shell-package-build-check: ok"
