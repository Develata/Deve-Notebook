#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
REQUIRED="${DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED:-0}"
TARGET="${DEVE_MOBILE_IOS_PACKAGE_TARGET:-aarch64-sim}"

# This gate builds only the iOS WebView shell; it must not open child-process runtime.

run() {
  echo "+ $*"
  "$@"
}

run_deve_baseline "$ROOT_DIR" "mobile-ios-shell-package-build" "mobile-ios-shell-package-build-check"

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

run_deve_baseline "$ROOT_DIR" "mobile-ios-shell-package-build" "mobile-ios-shell-package-build-check"
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

echo "mobile-ios-shell-package-build-check: ok"
