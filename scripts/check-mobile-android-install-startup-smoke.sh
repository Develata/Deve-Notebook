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
readonly ADB_KILL_AFTER_SECS=5
readonly INSTALL_RETRY_DEADLINE_SECS=180
readonly PACKAGE_SERVICE_READY_ATTEMPTS=10
readonly PACKAGE_SERVICE_READY_INTERVAL_SECS=2

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

install_retry_now() {
  printf '%s\n' "$SECONDS"
}

adb_retry_timed() {
  local deadline="$1"
  local now remaining operation_timeout
  shift

  now="$(install_retry_now)"
  remaining=$((deadline - now))
  (( remaining > 0 )) || return 124
  operation_timeout="$ADB_TIMEOUT_SECS"
  (( operation_timeout <= remaining )) || operation_timeout="$remaining"
  adb_with_timeout "$operation_timeout" "$@"
}

retryable_android_package_install_failure() {
  local status="$1"
  local output="$2"

  (( status == 1 )) || return 1
  printf '%s\n' "$output" | tr -d '\r' | awk '
    /^[[:space:]]*$/ { next }
    $0 == "Performing Streamed Install" { next }
    /^adb: failed to install .+: cmd: Failure calling service package: Broken pipe \(32\)$/ {
      broken_pipe += 1
      next
    }
    { unexpected = 1 }
    END { exit !(broken_pipe == 1 && unexpected == 0) }
  '
}

retryable_android_package_readiness_failure() {
  local status="$1"
  local output="$2"

  (( status == 20 )) || return 1
  printf '%s\n' "$output" | tr -d '\r' | awk '
    /^[[:space:]]*$/ { next }
    $0 == "cmd: Can'\''t find service: package" {
      unavailable += 1
      next
    }
    { unexpected = 1 }
    END { exit !(unavailable == 1 && unexpected == 0) }
  '
}

wait_for_android_package_service() {
  local deadline="$1"
  local attempt now output status

  for ((attempt = 1; attempt <= PACKAGE_SERVICE_READY_ATTEMPTS; attempt += 1)); do
    if output="$(adb_retry_timed "$deadline" shell cmd package list packages 2>&1)"; then
      return 0
    else
      status=$?
    fi
    printf '%s\n' "$output" >&2
    if ! retryable_android_package_readiness_failure "$status" "$output"; then
      return "$status"
    fi
    if (( attempt == PACKAGE_SERVICE_READY_ATTEMPTS )); then
      return "$status"
    fi
    now="$(install_retry_now)"
    (( deadline - now > PACKAGE_SERVICE_READY_INTERVAL_SECS )) || return 124
    echo "mobile-android-install-startup-smoke-check: waiting for package service (attempt $attempt/$PACKAGE_SERVICE_READY_ATTEMPTS)" >&2
    sleep "$PACKAGE_SERVICE_READY_INTERVAL_SECS" || return $?
  done
}

verify_install_retry_contract() {
  local readiness_status

  retryable_android_package_install_failure 1 \
    "adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)" \
    || fail "package-service Broken pipe must remain the only retryable install failure"
  if retryable_android_package_install_failure 1 \
    "adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipeline (32)"; then
    fail "Broken pipeline must not be classified as Broken pipe"
  fi
  if retryable_android_package_install_failure 124 \
    "adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)"; then
    fail "timed-out Android installs must not be retried"
  fi
  if retryable_android_package_install_failure 1 \
    $'adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)\nFailure [INSTALL_FAILED_INVALID_APK]'; then
    fail "mixed Android install failures must not be retried"
  fi
  if retryable_android_package_install_failure 1 \
    "adb: failed to install candidate.apk: Failure [INSTALL_FAILED_INVALID_APK]"; then
    fail "non-transport Android install failures must remain fail-closed"
  fi
  retryable_android_package_readiness_failure 20 \
    "cmd: Can't find service: package" \
    || fail "package-service restart must remain the only retryable readiness failure"
  for readiness_status in 124 130 143; do
    if retryable_android_package_readiness_failure "$readiness_status" \
      "cmd: Can't find service: package"; then
      fail "timeout and interruption statuses must not be retried: $readiness_status"
    fi
  done
  if retryable_android_package_readiness_failure 20 \
    $'cmd: Can'\''t find service: package\nerror: device offline'; then
    fail "mixed Android readiness failures must not be retried"
  fi
}

install_apk() {
  local attempt now output status
  local deadline
  now="$(install_retry_now)"
  deadline=$((now + INSTALL_RETRY_DEADLINE_SECS))

  for attempt in 1 2 3; do
    echo "+ adb_timed install -r $APK_PATH (attempt $attempt/3)"
    if output="$(adb_retry_timed "$deadline" install -r "$APK_PATH" 2>&1)"; then
      printf '%s\n' "$output"
      return 0
    else
      status=$?
    fi
    printf '%s\n' "$output" >&2
    if (( attempt == 3 )) \
        || ! retryable_android_package_install_failure "$status" "$output"; then
      return "$status"
    fi
    echo "mobile-android-install-startup-smoke-check: retrying after package-service Broken pipe" >&2
    now="$(install_retry_now)"
    (( deadline - now > 2 )) || return 124
    sleep 2 || return $?
    adb_retry_timed "$deadline" wait-for-device >/dev/null || return $?
    wait_for_android_package_service "$deadline" || return $?
  done
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

if [[ "${BASH_SOURCE[0]}" != "$0" ]]; then
  return 0
fi

[[ "$ADB_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] \
  || fail "DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS must be a positive integer"

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
trap cleanup EXIT

run install_apk
run adb_timed shell monkey -p "$APP_ID" -c android.intent.category.LAUNCHER 1
sleep "$STARTUP_WAIT_SECS"

pid="$(app_pid)"
[[ -n "$pid" ]] || fail "Android app did not remain running after launch: $APP_ID"

echo "mobile-android-install-startup-smoke-check: app_id=$APP_ID pid=$pid"
echo "mobile-android-install-startup-smoke-check: serial=${ADB_SERIAL:-<default adb target>}"
echo "mobile-android-install-startup-smoke-check: ok"
