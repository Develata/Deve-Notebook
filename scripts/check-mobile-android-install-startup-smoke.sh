#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
REQUIRED="${DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED:-0}"
APK_PATH="${DEVE_MOBILE_ANDROID_APK_PATH:-apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk}"
APP_ID="${DEVE_MOBILE_ANDROID_APP_ID:-dev.deve.notebook.mobile}"
UNINSTALL_AFTER="${DEVE_MOBILE_ANDROID_INSTALL_SMOKE_UNINSTALL:-1}"
STARTUP_WAIT_SECS="${DEVE_MOBILE_ANDROID_STARTUP_WAIT_SECS:-3}"
ADB_SERIAL="${DEVE_MOBILE_ANDROID_SERIAL:-}"
ADB_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS:-60}"

# This gate installs and launches only the Android WebView shell. It must not
# imply release readiness, child-process runtime, backend process ownership, or
# native authority writes.

fail() {
  echo "mobile-android-install-startup-smoke-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

adb_cmd() {
  android_tool_path adb >/dev/null 2>&1 || fail "adb is required for Android install/startup smoke"
  if [[ -n "$ADB_SERIAL" ]]; then
    android_run_tool adb -s "$ADB_SERIAL" "$@"
    return
  fi
  android_run_tool adb "$@"
}

adb_timed() {
  local adb_args=()

  android_tool_path adb >/dev/null 2>&1 || fail "adb is required for Android install/startup smoke"
  command -v timeout >/dev/null 2>&1 \
    || fail "timeout is required for bounded Android install/startup smoke"
  if [[ -n "$ADB_SERIAL" ]]; then
    adb_args=(-s "$ADB_SERIAL")
  fi
  timeout "$ADB_TIMEOUT_SECS" "$(android_tool_path adb)" "${adb_args[@]}" "$@"
}

app_pid() {
  local pid
  pid="$(adb_timed shell pidof "$APP_ID" 2>/dev/null | tr -d '\r' || true)"
  if [[ -n "$pid" ]]; then
    printf '%s\n' "$pid"
    return 0
  fi
  adb_timed shell ps -A 2>/dev/null | tr -d '\r' | awk -v app="$APP_ID" 'index($0, app) { print $2; exit }'
}

cleanup() {
  [[ "$UNINSTALL_AFTER" == "1" ]] || return 0
  adb_timed uninstall "$APP_ID" >/dev/null 2>&1 || true
}

run_deve_baseline "$ROOT_DIR" "mobile-android-install-startup-smoke" "mobile-android-install-startup-smoke-check"
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-android-install-startup-smoke-check: install/startup not executed; set DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED=1 on an Android emulator/device host"
  echo "mobile-android-install-startup-smoke-check: apk path: $APK_PATH"
  echo "mobile-android-install-startup-smoke-check: serial: ${ADB_SERIAL:-<default adb target>}"
  echo "mobile-android-install-startup-smoke-check: ok"
  exit 0
fi

[[ -f "$ROOT_DIR/$APK_PATH" || -f "$APK_PATH" ]] \
  || fail "Android APK is required before install/startup smoke: $APK_PATH"

if [[ -f "$ROOT_DIR/$APK_PATH" ]]; then
  APK_PATH="$ROOT_DIR/$APK_PATH"
fi

run adb_timed start-server
run adb_timed wait-for-device
trap cleanup EXIT

run adb_timed install -r "$APK_PATH"
run adb_timed shell monkey -p "$APP_ID" -c android.intent.category.LAUNCHER 1
sleep "$STARTUP_WAIT_SECS"

pid="$(app_pid)"
[[ -n "$pid" ]] || fail "Android app did not remain running after launch: $APP_ID"

echo "mobile-android-install-startup-smoke-check: app_id=$APP_ID pid=$pid"
echo "mobile-android-install-startup-smoke-check: serial=${ADB_SERIAL:-<default adb target>}"
echo "mobile-android-install-startup-smoke-check: ok"
