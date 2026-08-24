#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
source "$ROOT_DIR/scripts/lib/android-startup-diagnostics.sh"
source "$ROOT_DIR/scripts/lib/android-package-session.sh"
REQUIRED="${DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED:-0}"
APK_PATH="${DEVE_MOBILE_ANDROID_APK_PATH:-apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk}"
APP_ID="${DEVE_MOBILE_ANDROID_APP_ID:-dev.deve.notebook.mobile}"
UNINSTALL_AFTER="${DEVE_MOBILE_ANDROID_INSTALL_SMOKE_UNINSTALL:-1}"
STARTUP_WAIT_SECS="${DEVE_MOBILE_ANDROID_STARTUP_WAIT_SECS:-3}"
ADB_SERIAL="${DEVE_MOBILE_ANDROID_SERIAL:-}"
ADB_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS:-60}"
PROCESS_RETIREMENT_WAIT_SECS="${DEVE_MOBILE_ANDROID_PROCESS_RETIREMENT_WAIT_SECS:-10}"
readonly ADB_KILL_AFTER_SECS=5
ANDROID_INSTALL_RETRY_LOG_PREFIX="mobile-android-install-startup-smoke-check"
source "$ROOT_DIR/scripts/lib/android-install-retry.sh"

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

adb_with_timeout() {
  local timeout_secs="$1"
  local adb_args=()
  shift

  android_tool_path adb >/dev/null 2>&1 || fail "adb is required for Android install/startup smoke"
  command -v timeout >/dev/null 2>&1 \
    || fail "timeout is required for bounded Android install/startup smoke"
  if [[ -n "$ADB_SERIAL" ]]; then
    adb_args=(-s "$ADB_SERIAL")
  fi
  timeout --kill-after="${ADB_KILL_AFTER_SECS}s" "${timeout_secs}s" \
    "$(android_tool_path adb)" "${adb_args[@]}" "$@"
}

adb_timed() {
  adb_with_timeout "$ADB_TIMEOUT_SECS" "$@"
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

android_startup_diag_adb() {
  adb_with_timeout "$@"
}

# Keeps the primary process-exit failure authoritative: diagnostics run only
# when the app process is missing, and a diagnostics failure never replaces it.
require_app_running_after_launch() {
  APP_RUNNING_PID="$(app_pid)"
  if [[ -z "$APP_RUNNING_PID" ]]; then
    android_startup_diagnostics_collect "$APP_ID" \
      || echo "mobile-android-install-startup-smoke-check: startup exit diagnostics collection failed" >&2
    fail "Android app did not remain running after launch: $APP_ID"
  fi
}

process_retirement_now() {
  printf '%s\n' "$SECONDS"
}

process_retirement_delay() {
  sleep "$1"
}

wait_for_app_process_retirement() {
  local started_at
  local deadline
  local now
  local remaining
  local process_listing=""
  local missing_samples=0

  started_at="$(process_retirement_now)" \
    || { echo "mobile-android-install-startup-smoke-check: Android process retirement clock failed" >&2; return 1; }
  deadline=$((started_at + PROCESS_RETIREMENT_WAIT_SECS))
  while true; do
    now="$(process_retirement_now)" \
      || { echo "mobile-android-install-startup-smoke-check: Android process retirement clock failed" >&2; return 1; }
    remaining=$((deadline - now))
    (( remaining > 0 )) || break
    process_listing="$(adb_with_timeout "$remaining" shell ps -A 2>/dev/null | tr -d '\r')" \
      || { echo "mobile-android-install-startup-smoke-check: Android process retirement probe failed: $APP_ID" >&2; return 1; }
    if printf '%s\n' "$process_listing" \
      | awk -v app="$APP_ID" '$NF == app { found = 1 } END { exit !found }'; then
      missing_samples=0
    else
      missing_samples=$((missing_samples + 1))
      if (( missing_samples >= 2 )); then
        return 0
      fi
    fi
    now="$(process_retirement_now)" \
      || { echo "mobile-android-install-startup-smoke-check: Android process retirement clock failed" >&2; return 1; }
    remaining=$((deadline - now))
    (( remaining > 0 )) || break
    process_retirement_delay 1 \
      || { echo "mobile-android-install-startup-smoke-check: Android process retirement delay failed" >&2; return 1; }
  done
  echo "mobile-android-install-startup-smoke-check: Android app process remained after bounded cleanup: $APP_ID" >&2
  return 1
}

cleanup() {
  local package_listing=""
  local launcher_resolution=""

  [[ "$UNINSTALL_AFTER" == "1" ]] || return 0
  android_package_session_cleanup 0 adb_timed "$APP_ID" >/dev/null || return 1

  package_listing="$(adb_timed shell pm list packages "$APP_ID" 2>/dev/null | tr -d '\r')" \
    || { echo "mobile-android-install-startup-smoke-check: Android package retirement probe failed: $APP_ID" >&2; return 1; }
  if printf '%s\n' "$package_listing" | grep -Fxq "package:$APP_ID"; then
    echo "mobile-android-install-startup-smoke-check: Android package remained installed after cleanup: $APP_ID" >&2
    return 1
  fi

  launcher_resolution="$(adb_timed shell cmd package resolve-activity --brief \
    -a android.intent.action.MAIN -c android.intent.category.LAUNCHER "$APP_ID" 2>/dev/null | tr -d '\r')" \
    || { echo "mobile-android-install-startup-smoke-check: Android launcher retirement probe failed: $APP_ID" >&2; return 1; }
  if [[ -n "$launcher_resolution" && "$launcher_resolution" != "No activity found" ]]; then
    echo "mobile-android-install-startup-smoke-check: Android launcher remained resolvable after cleanup: $APP_ID" >&2
    return 1
  fi

  wait_for_app_process_retirement
}

cleanup_on_exit() {
  local status=$?
  local cleanup_status=0
  trap - EXIT
  cleanup || cleanup_status=$?
  if (( status != 0 )); then
    exit "$status"
  fi
  exit "$cleanup_status"
}

if [[ "${BASH_SOURCE[0]}" != "$0" ]]; then
  return 0
fi

[[ "$ADB_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] \
  || fail "DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS must be a positive integer"
[[ "$PROCESS_RETIREMENT_WAIT_SECS" =~ ^[1-9][0-9]*$ ]] \
  || fail "DEVE_MOBILE_ANDROID_PROCESS_RETIREMENT_WAIT_SECS must be a positive integer"

run_deve_baseline "$ROOT_DIR" "mobile-android-install-startup-smoke" "mobile-android-install-startup-smoke-check"
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"
verify_install_retry_contract

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
trap cleanup_on_exit EXIT

run install_apk
android_startup_diagnostics_prepare "$APP_ID"
run adb_timed shell monkey -p "$APP_ID" -c android.intent.category.LAUNCHER 1
sleep "$STARTUP_WAIT_SECS"

require_app_running_after_launch

echo "mobile-android-install-startup-smoke-check: app_id=$APP_ID pid=$APP_RUNNING_PID"
echo "mobile-android-install-startup-smoke-check: serial=${ADB_SERIAL:-<default adb target>}"
echo "mobile-android-install-startup-smoke-check: ok"
