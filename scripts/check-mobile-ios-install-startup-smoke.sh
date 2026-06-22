#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
REQUIRED="${DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED:-0}"
APP_PATH="${DEVE_MOBILE_IOS_APP_PATH:-apps/mobile/gen/apple/build/arm64-sim/Deve Notebook.app}"
BUNDLE_ID="${DEVE_MOBILE_IOS_BUNDLE_ID:-dev.deve.notebook.mobile}"
SIMULATOR="${DEVE_MOBILE_IOS_SIMULATOR:-booted}"
BOOT_SIMULATOR="${DEVE_MOBILE_IOS_BOOT_SIMULATOR:-0}"
TERMINATE_AFTER="${DEVE_MOBILE_IOS_INSTALL_SMOKE_TERMINATE:-1}"

# This gate installs and launches only the iOS WebView shell. It must not imply
# release readiness, child-process runtime, backend process ownership, or native
# authority writes.

fail() {
  echo "mobile-ios-install-startup-smoke-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

xcrun_cmd() {
  command -v xcrun >/dev/null 2>&1 || fail "xcrun is required for iOS install/startup smoke"
  xcrun "$@"
}

booted_simulator_exists() {
  xcrun_cmd simctl list devices booted | grep -q "Booted"
}

select_available_simulator() {
  xcrun_cmd simctl list devices available \
    | awk -F '[()]' '/iPhone/ && /Shutdown/ { print $2; exit }'
}

ensure_booted_simulator() {
  if [[ "$SIMULATOR" != "booted" ]]; then
    xcrun_cmd simctl bootstatus "$SIMULATOR" -b >/dev/null
    return 0
  fi

  if booted_simulator_exists; then
    return 0
  fi

  [[ "$BOOT_SIMULATOR" == "1" ]] \
    || fail "a booted iOS simulator is required; boot one or set DEVE_MOBILE_IOS_BOOT_SIMULATOR=1"

  local selected
  selected="$(select_available_simulator)"
  [[ -n "$selected" ]] || fail "no available shutdown iPhone simulator found to boot"

  xcrun_cmd simctl boot "$selected" >/dev/null || true
  xcrun_cmd simctl bootstatus "$selected" -b >/dev/null
  SIMULATOR="$selected"
}

cleanup() {
  [[ "$TERMINATE_AFTER" == "1" ]] || return 0
  xcrun_cmd simctl terminate "$SIMULATOR" "$BUNDLE_ID" >/dev/null 2>&1 || true
}

run_deve_baseline "$ROOT_DIR" "mobile-ios-install-startup-smoke" "mobile-ios-install-startup-smoke-check"
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-ios-install-startup-smoke-check: install/startup not executed; set DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED=1 on a macOS simulator host"
  echo "mobile-ios-install-startup-smoke-check: app path: $APP_PATH"
  echo "mobile-ios-install-startup-smoke-check: ok"
  exit 0
fi

[[ "$(uname -s 2>/dev/null || true)" == "Darwin" ]] \
  || fail "iOS install/startup smoke requires macOS"

[[ -d "$ROOT_DIR/$APP_PATH" || -d "$APP_PATH" ]] \
  || fail "iOS .app bundle is required before install/startup smoke: $APP_PATH"

if [[ -d "$ROOT_DIR/$APP_PATH" ]]; then
  APP_PATH="$ROOT_DIR/$APP_PATH"
fi

ensure_booted_simulator

trap cleanup EXIT

run xcrun_cmd simctl install "$SIMULATOR" "$APP_PATH"
run xcrun_cmd simctl launch "$SIMULATOR" "$BUNDLE_ID"

echo "mobile-ios-install-startup-smoke-check: bundle_id=$BUNDLE_ID simulator=$SIMULATOR"
echo "mobile-ios-install-startup-smoke-check: ok"
