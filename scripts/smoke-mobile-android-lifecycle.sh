#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
ANDROID_INSTALL_RETRY_LOG_PREFIX="mobile-android-lifecycle-smoke"
source "$ROOT_DIR/scripts/lib/android-install-retry.sh"
source "$ROOT_DIR/scripts/lib/android-startup-diagnostics.sh"
source "$ROOT_DIR/scripts/lib/android-app-process-readiness.sh"

REQUIRED="${DEVE_MOBILE_ANDROID_LIFECYCLE_SMOKE_REQUIRED:-0}"
ADB_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS:-60}"
APK_PATH="${DEVE_MOBILE_ANDROID_APK_PATH:-apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk}"
APP_ID="${DEVE_MOBILE_ANDROID_APP_ID:-dev.deve.notebook.mobile}"
SERIAL="${DEVE_MOBILE_ANDROID_SERIAL:-}"
TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS:-180}"
NODE_SCRIPT="${DEVE_MOBILE_ANDROID_LIFECYCLE_NODE_SCRIPT:-$ROOT_DIR/scripts/smoke-mobile-android-lifecycle.mjs}"
EXPECT_WRITABLE="${DEVE_MOBILE_ANDROID_EXPECT_WRITABLE:-1}"
TARGET_FACTS_PATH="${DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH:-}"
EVIDENCE_PATH="${DEVE_MOBILE_ANDROID_LOCAL_EVIDENCE_PATH:-${DEVE_MOBILE_ANDROID_EVIDENCE_PATH:-}}"
TARGET_FACTS_TEMP=0

fail() {
  echo "mobile-android-lifecycle-smoke: $*" >&2
  exit 1
}

adb_bin() {
  android_tool_path adb || fail "adb is required for Android lifecycle smoke"
}

adb_cmd() {
  local args=()
  local limit=30
  [[ -z "$SERIAL" ]] || args=(-s "$SERIAL")
  if [[ -n "${GLOBAL_DEADLINE:-}" ]]; then
    limit=$((GLOBAL_DEADLINE - SECONDS))
    (( limit > 0 )) || fail "global lifecycle smoke deadline exhausted before adb $*"
  fi
  timeout "$limit" "$(adb_bin)" "${args[@]}" "$@"
}

adb_with_timeout() {
  local timeout_secs="$1"
  local args=() remaining
  shift
  [[ -z "$SERIAL" ]] || args=(-s "$SERIAL")
  if [[ -n "${GLOBAL_DEADLINE:-}" ]]; then
    remaining=$((GLOBAL_DEADLINE - SECONDS))
    (( remaining > 0 )) || fail "global lifecycle smoke deadline exhausted before adb $*"
    (( timeout_secs <= remaining )) || timeout_secs="$remaining"
  fi
  timeout "$timeout_secs" "$(adb_bin)" "${args[@]}" "$@"
}

app_pid() {
  android_app_process_pidof_probe adb_cmd "$APP_ID"
}

lifecycle_clock() {
  printf '%s\n' "$SECONDS"
}

adb_cleanup_cmd() {
  local args=()
  [[ -z "$SERIAL" ]] || args=(-s "$SERIAL")
  timeout 10 "$(adb_bin)" "${args[@]}" "$@" >/dev/null 2>&1 || true
}

cleanup() {
  if [[ -n "${FORWARD_PORT:-}" ]]; then
    adb_cleanup_cmd forward --remove "tcp:$FORWARD_PORT"
  fi
  adb_cleanup_cmd shell am force-stop "$APP_ID"
  adb_cleanup_cmd uninstall "$APP_ID"
  if [[ "$TARGET_FACTS_TEMP" == "1" ]]; then
    rm -f "$TARGET_FACTS_PATH"
  fi
}

find_webview_socket() {
  local pid="$1"
  local sockets
  # Quoted remote command: Git Bash (MSYS) path-converts a bare /proc/... arg
  # into a Windows host path on Windows target hosts.
  sockets="$(adb_cmd shell "cat /proc/net/unix" 2>/dev/null | tr -d '\r' | awk '$NF ~ /webview_devtools_remote/ { print $NF }')"
  printf '%s\n' "$sockets" | grep -E "(^|_)${pid}$" | head -n 1 || true
}

android_startup_diag_adb() {
  local timeout_secs="$1"
  local args=()
  shift
  [[ -z "$SERIAL" ]] || args=(-s "$SERIAL")
  timeout "$timeout_secs" "$(adb_bin)" "${args[@]}" "$@"
}

# Bounded evidence for a launched app whose debug WebView socket never
# appears: the abstract-socket inventory separates "WebView never came up"
# from "socket exists under an unexpected name", and the app process snapshot
# plus a capped logcat tail record what the alive app was doing.
report_missing_webview_socket() {
  local pid="$1"
  echo "mobile-android-lifecycle-smoke: webview_devtools sockets visible to adb:" >&2
  adb_with_timeout 20 shell "cat /proc/net/unix" 2>/dev/null | tr -d '\r' \
    | awk '$NF ~ /webview_devtools_remote/ { print "  " $NF }' >&2 || true
  echo "mobile-android-lifecycle-smoke: app process snapshot:" >&2
  adb_with_timeout 20 shell ps -A 2>/dev/null | tr -d '\r' \
    | awk -v app="$APP_ID" 'NR == 1 || index($0, app) { print "  " $0 }' >&2 || true
  echo "mobile-android-lifecycle-smoke: bounded app logcat tail:" >&2
  adb_with_timeout 20 logcat -d -v threadtime 2>/dev/null | tr -d '\r' \
    | grep -E "ActivityManager|AndroidRuntime|chromium|WebView|$APP_ID" \
    | tail -n 200 >&2 || true
  fail "debug WebView socket not found for pid $pid"
}

report_lifecycle_harness_failure() {
  local pid=""
  pid="$(android_startup_diag_adb 10 shell pidof "$APP_ID" 2>/dev/null \
    | tr -d '\r' | awk '{ print $1; exit }' || true)"
  echo "mobile-android-lifecycle-smoke: lifecycle harness failed; app_pid=${pid:-absent}" >&2
  android_startup_diagnostics_collect "$APP_ID"
}

remaining_seconds() {
  local remaining=$((GLOBAL_DEADLINE - SECONDS))
  (( remaining > 0 )) || fail "global lifecycle smoke deadline exhausted"
  printf '%s\n' "$remaining"
}

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-android-lifecycle-smoke: not executed; set DEVE_MOBILE_ANDROID_LIFECYCLE_SMOKE_REQUIRED=1 on an Android emulator host"
  echo "mobile-android-lifecycle-smoke: ok"
  exit 0
fi

[[ -n "$SERIAL" ]] || fail "DEVE_MOBILE_ANDROID_SERIAL is required"
command -v node >/dev/null 2>&1 || fail "node is required"
command -v timeout >/dev/null 2>&1 || fail "timeout is required"
[[ -f "$ROOT_DIR/$APK_PATH" || -f "$APK_PATH" ]] || fail "debug APK not found: $APK_PATH"
[[ -f "$NODE_SCRIPT" ]] || fail "lifecycle harness not found: $NODE_SCRIPT"
[[ -f "$ROOT_DIR/$APK_PATH" ]] && APK_PATH="$ROOT_DIR/$APK_PATH"
GLOBAL_DEADLINE=$((SECONDS + TIMEOUT_SECS))

if [[ -z "$TARGET_FACTS_PATH" ]]; then
  TARGET_FACTS_PATH="${TMPDIR:-/tmp}/deve-android-target-facts-$$.json"
  TARGET_FACTS_TEMP=1
fi

trap cleanup EXIT
adb_cmd start-server >/dev/null
adb_cmd wait-for-device
SDK_RAW="$(adb_cmd shell getprop ro.build.version.sdk | tr -d '\r')"
WEBVIEW_CMD_RAW="$(adb_cmd shell cmd webviewupdate getCurrentWebViewPackage 2>&1 | tr -d '\r' || true)"
WEBVIEW_DUMPSYS_RAW="$(adb_cmd shell dumpsys webviewupdate 2>&1 | tr -d '\r' || true)"
WEBVIEW_RAW="$WEBVIEW_CMD_RAW
$WEBVIEW_DUMPSYS_RAW"
AVD_NAME="$(adb_cmd shell getprop ro.boot.qemu.avd_name 2>/dev/null | tr -d '\r' || true)"
BUILD_FINGERPRINT="$(adb_cmd shell getprop ro.build.fingerprint 2>/dev/null | tr -d '\r' || true)"
MODEL="$(adb_cmd shell getprop ro.product.model 2>/dev/null | tr -d '\r' || true)"
TARGET_FACTS="$(
  DEVE_ANDROID_TARGET_SDK_RAW="$SDK_RAW" \
  DEVE_ANDROID_TARGET_WEBVIEW_RAW="$WEBVIEW_RAW" \
  DEVE_ANDROID_TARGET_AVD_NAME="$AVD_NAME" \
  DEVE_ANDROID_TARGET_BUILD_FINGERPRINT="$BUILD_FINGERPRINT" \
  DEVE_ANDROID_TARGET_MODEL="$MODEL" \
  DEVE_MOBILE_ANDROID_EXPECT_WRITABLE="$EXPECT_WRITABLE" \
  DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH="$TARGET_FACTS_PATH" \
  node "$ROOT_DIR/scripts/inspect-android-target-capability.mjs"
)" || fail "Android target does not satisfy the requested evidence mode"
echo "mobile-android-lifecycle-smoke: target_facts=$TARGET_FACTS"
install_apk
adb_cmd logcat -c
android_startup_diagnostics_prepare "$APP_ID"
adb_cmd shell monkey -p "$APP_ID" -c android.intent.category.LAUNCHER 1 >/dev/null

deadline=$GLOBAL_DEADLINE
if android_app_process_wait_stable "$deadline" app_pid lifecycle_clock; then
  PID="$ANDROID_APP_PROCESS_STABLE_PID"
else
  PID_STATUS=$?
  PID_EVIDENCE="$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE"
  android_startup_diagnostics_collect "$APP_ID" \
    || echo "mobile-android-lifecycle-smoke: startup readiness diagnostics collection failed" >&2
  fail "Android app did not remain running: $APP_ID (process readiness: $PID_EVIDENCE status=$PID_STATUS)"
fi

SOCKET=""
CURRENT_PID="$PID"
while (( SECONDS < deadline )); do
  if android_app_process_observe_anchored "$PID" app_pid; then
    CURRENT_PID="$ANDROID_APP_PROCESS_CURRENT_PID"
  else
    PROCESS_STATUS=$?
    PROCESS_EVIDENCE="$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE"
    android_startup_diagnostics_collect "$APP_ID" \
      || echo "mobile-android-lifecycle-smoke: startup exit diagnostics collection failed" >&2
    case "$PROCESS_EVIDENCE" in
      process=replaced*)
        fail "Android app restarted while waiting for its debug WebView socket (pid $PID; $PROCESS_EVIDENCE)"
        ;;
      process=absent*)
        fail "Android app exited while waiting for its debug WebView socket (initial pid $PID; $PROCESS_EVIDENCE)"
        ;;
      *)
        fail "Android app process probe failed while waiting for its debug WebView socket ($PROCESS_EVIDENCE status=$PROCESS_STATUS)"
        ;;
    esac
  fi
  if [[ -z "$CURRENT_PID" ]]; then
    sleep 1
    continue
  fi
  SOCKET="$(find_webview_socket "$PID")"
  [[ -z "$SOCKET" ]] || break
  sleep 1
done
[[ -n "$SOCKET" ]] || report_missing_webview_socket "$PID"
SOCKET="${SOCKET#@}"
FORWARD_PORT="$(adb_cmd forward tcp:0 "localabstract:$SOCKET" | tr -d '\r')"
[[ "$FORWARD_PORT" =~ ^[0-9]+$ ]] || fail "adb did not allocate a CDP forward port: $FORWARD_PORT"

set +e
DEVE_MOBILE_ANDROID_CDP_ENDPOINT="http://127.0.0.1:$FORWARD_PORT" \
DEVE_MOBILE_ANDROID_ADB_BIN="$(adb_bin)" \
DEVE_MOBILE_ANDROID_SERIAL="$SERIAL" \
DEVE_MOBILE_ANDROID_APP_ID="$APP_ID" \
DEVE_MOBILE_ANDROID_EXPECT_WRITABLE="$EXPECT_WRITABLE" \
DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH="$TARGET_FACTS_PATH" \
DEVE_MOBILE_ANDROID_EVIDENCE_PATH="$EVIDENCE_PATH" \
DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_MS="$(($(remaining_seconds) * 1000))" \
timeout "$(remaining_seconds)" node "$NODE_SCRIPT"
NODE_STATUS=$?
set -e
if (( NODE_STATUS != 0 )); then
  report_lifecycle_harness_failure
  exit "$NODE_STATUS"
fi

for _ in $(seq 1 30); do
  [[ -z "$(app_pid || true)" ]] && break
  sleep 1
done
[[ -z "$(app_pid || true)" ]] || fail "Android app/backend process remained after bounded graceful exit"

LOGCAT="$(adb_cmd logcat -d 2>/dev/null | tr -d '\r')"
printf '%s\n' "$LOGCAT" | grep -F "deve_mobile LocalBackend clean shutdown complete" >/dev/null \
  || fail "clean LocalBackend shutdown marker missing from Android logcat"
if printf '%s\n' "$LOGCAT" | grep -F "deve_mobile LocalBackend exit shutdown failed closed" >/dev/null; then
  fail "Android logcat reports LocalBackend shutdown failure"
fi

echo "mobile-android-lifecycle-smoke: app_id=$APP_ID serial=$SERIAL initial_pid=$PID"
echo "mobile-android-lifecycle-smoke: ok"
