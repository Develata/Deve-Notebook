#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
ANDROID_INSTALL_RETRY_LOG_PREFIX="mobile-android-remote-browser-smoke"
source "$ROOT_DIR/scripts/lib/android-install-retry.sh"
source "$ROOT_DIR/scripts/lib/android-startup-diagnostics.sh"
source "$ROOT_DIR/scripts/lib/android-app-process-readiness.sh"

REQUIRED="${DEVE_MOBILE_ANDROID_REMOTE_SMOKE_REQUIRED:-0}"
ADB_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS:-60}"
APK_PATH="${DEVE_MOBILE_ANDROID_APK_PATH:-apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk}"
APP_ID="${DEVE_MOBILE_ANDROID_APP_ID:-dev.deve.notebook.mobile}"
SERIAL="${DEVE_MOBILE_ANDROID_SERIAL:-}"
REMOTE_ORIGIN="${DEVE_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN:-}"
USERNAME="${DEVE_MOBILE_ANDROID_REMOTE_USERNAME:-}"
PASSWORD="${DEVE_MOBILE_ANDROID_REMOTE_PASSWORD:-}"
TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_REMOTE_TIMEOUT_SECS:-240}"
TARGET_FACTS_PATH="${DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH:-}"
EVIDENCE_PATH="${DEVE_MOBILE_ANDROID_REMOTE_EVIDENCE_PATH:-${DEVE_MOBILE_ANDROID_EVIDENCE_PATH:-}}"
TARGET_FACTS_TEMP=0

fail() { echo "mobile-android-remote-browser-smoke: $*" >&2; exit 1; }

adb_bin() { android_tool_path adb || fail "adb is required"; }

adb_cmd() {
  local limit=$((GLOBAL_DEADLINE - SECONDS))
  (( limit > 0 )) || fail "global deadline exhausted before adb $*"
  timeout "$limit" "$(adb_bin)" -s "$SERIAL" "$@"
}

adb_with_timeout() {
  local timeout_secs="$1" remaining
  shift
  remaining=$((GLOBAL_DEADLINE - SECONDS))
  (( remaining > 0 )) || fail "global deadline exhausted before adb $*"
  (( timeout_secs <= remaining )) || timeout_secs="$remaining"
  timeout "$timeout_secs" "$(adb_bin)" -s "$SERIAL" "$@"
}

adb_cleanup_cmd() {
  timeout 10 "$(adb_bin)" -s "$SERIAL" "$@" >/dev/null 2>&1 || true
}

app_pid() { android_app_process_pidof_probe adb_cmd "$APP_ID"; }

remote_browser_clock() { printf '%s\n' "$SECONDS"; }

android_startup_diag_adb() { adb_with_timeout "$@"; }

cleanup() {
  [[ -z "${FORWARD_PORT:-}" ]] || adb_cleanup_cmd forward --remove "tcp:$FORWARD_PORT"
  adb_cleanup_cmd shell am force-stop "$APP_ID"
  adb_cleanup_cmd uninstall "$APP_ID"
  [[ "$TARGET_FACTS_TEMP" != "1" ]] || rm -f "$TARGET_FACTS_PATH"
}

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-android-remote-browser-smoke: not executed; set DEVE_MOBILE_ANDROID_REMOTE_SMOKE_REQUIRED=1"
  echo "mobile-android-remote-browser-smoke: ok"
  exit 0
fi

[[ -n "$SERIAL" && -n "$REMOTE_ORIGIN" && -n "$USERNAME" && -n "$PASSWORD" ]] \
  || fail "serial, remote HTTPS origin, username, and password are required"
node -e 'const u=new URL(process.argv[1]);if(u.protocol!=="https:"||u.origin!==process.argv[1])process.exit(2)' \
  "$REMOTE_ORIGIN" || fail "remote target must be an exact HTTPS origin"
[[ -f "$ROOT_DIR/$APK_PATH" ]] && APK_PATH="$ROOT_DIR/$APK_PATH"
[[ -f "$APK_PATH" ]] || fail "debug APK not found: $APK_PATH"
GLOBAL_DEADLINE=$((SECONDS + TIMEOUT_SECS))
if [[ -z "$TARGET_FACTS_PATH" ]]; then
  TARGET_FACTS_PATH="${TMPDIR:-/tmp}/deve-android-remote-target-facts-$$.json"
  TARGET_FACTS_TEMP=1
fi
trap cleanup EXIT

adb_cmd start-server >/dev/null
adb_cmd wait-for-device
SDK_RAW="$(adb_cmd shell getprop ro.build.version.sdk | tr -d '\r')"
WEBVIEW_RAW="$(adb_cmd shell cmd webviewupdate getCurrentWebViewPackage 2>&1 | tr -d '\r' || true)
$(adb_cmd shell dumpsys webviewupdate 2>&1 | tr -d '\r' || true)"
DEVE_ANDROID_TARGET_SDK_RAW="$SDK_RAW" \
DEVE_ANDROID_TARGET_WEBVIEW_RAW="$WEBVIEW_RAW" \
DEVE_ANDROID_TARGET_AVD_NAME="$(adb_cmd shell getprop ro.boot.qemu.avd_name 2>/dev/null | tr -d '\r' || true)" \
DEVE_ANDROID_TARGET_BUILD_FINGERPRINT="$(adb_cmd shell getprop ro.build.fingerprint 2>/dev/null | tr -d '\r' || true)" \
DEVE_ANDROID_TARGET_MODEL="$(adb_cmd shell getprop ro.product.model 2>/dev/null | tr -d '\r' || true)" \
DEVE_MOBILE_ANDROID_EXPECT_WRITABLE=1 \
DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH="$TARGET_FACTS_PATH" \
node "$ROOT_DIR/scripts/inspect-android-target-capability.mjs" >/dev/null

install_apk
PREFERENCE_JSON="$(node -e 'process.stdout.write(JSON.stringify({mode:"remote",remote_url:process.argv[1]}))' "$REMOTE_ORIGIN")"
PREFERENCE_BASE64="$(printf '%s' "$PREFERENCE_JSON" | base64 | tr -d '\r\n')"
# Single quoted remote command: adb shell flattens multiple arguments without
# re-quoting, so a bare sh -c payload runs its pipe/redirect in the device
# outer shell (read-only /) instead of inside run-as. run-as starts in the
# package data dir (Tauri app_data_dir -> Context.dataDir), so the relative
# path stays correct for any device user. The read-back guards against silent
# injection failure on targets that do not propagate remote exit codes.
[[ "$APP_ID" =~ ^[A-Za-z0-9._]+$ ]] || fail "APP_ID must be a plain package id"
adb_cmd shell "run-as $APP_ID sh -c 'echo $PREFERENCE_BASE64 | base64 -d > native-backend.json'" \
  || fail "RemoteBrowser preference injection failed"
adb_cmd shell "run-as $APP_ID cat native-backend.json" | grep -qF '"remote"' \
  || fail "RemoteBrowser preference did not land in the app data dir"
adb_cmd logcat -c
android_startup_diagnostics_prepare "$APP_ID"
adb_cmd shell monkey -p "$APP_ID" -c android.intent.category.LAUNCHER 1 >/dev/null

if android_app_process_wait_stable "$GLOBAL_DEADLINE" app_pid remote_browser_clock; then
  PID="$ANDROID_APP_PROCESS_STABLE_PID"
else
  PID_STATUS=$?
  PID_EVIDENCE="$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE"
  android_startup_diagnostics_collect "$APP_ID" \
    || echo "mobile-android-remote-browser-smoke: startup readiness diagnostics collection failed" >&2
  fail "Android RemoteBrowser app did not remain running (process readiness: $PID_EVIDENCE status=$PID_STATUS)"
fi
SOCKET=""
while (( SECONDS < GLOBAL_DEADLINE )); do
  if android_app_process_observe_anchored "$PID" app_pid; then
    CURRENT_PID="$ANDROID_APP_PROCESS_CURRENT_PID"
  else
    PROCESS_STATUS=$?
    PROCESS_EVIDENCE="$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE"
    android_startup_diagnostics_collect "$APP_ID" \
      || echo "mobile-android-remote-browser-smoke: startup exit diagnostics collection failed" >&2
    fail "Android RemoteBrowser app process failed while waiting for its WebView socket ($PROCESS_EVIDENCE status=$PROCESS_STATUS)"
  fi
  if [[ -z "$CURRENT_PID" ]]; then
    sleep 1
    continue
  fi
  # Quoted remote command: Git Bash (MSYS) path-converts a bare /proc/... arg
  # into a Windows host path on Windows target hosts.
  SOCKET="$(adb_cmd shell "cat /proc/net/unix" 2>/dev/null | tr -d '\r' | awk -v pid="$PID" '$NF ~ /webview_devtools_remote/ && $NF ~ pid"$" {print $NF; exit}')"
  [[ -z "$SOCKET" ]] || break
  sleep 1
done
[[ -n "$SOCKET" ]] || fail "RemoteBrowser WebView CDP socket unavailable"
FORWARD_PORT="$(adb_cmd forward tcp:0 "localabstract:${SOCKET#@}" | tr -d '\r')"
[[ "$FORWARD_PORT" =~ ^[0-9]+$ ]] || fail "adb did not allocate a CDP port"

if DEVE_MOBILE_ANDROID_CDP_ENDPOINT="http://127.0.0.1:$FORWARD_PORT" \
DEVE_MOBILE_ANDROID_ADB_BIN="$(adb_bin)" \
DEVE_MOBILE_ANDROID_SERIAL="$SERIAL" \
DEVE_MOBILE_ANDROID_APP_ID="$APP_ID" \
DEVE_MOBILE_ANDROID_EXPECTED_APP_PID="$PID" \
DEVE_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN="$REMOTE_ORIGIN" \
DEVE_MOBILE_ANDROID_REMOTE_USERNAME="$USERNAME" \
DEVE_MOBILE_ANDROID_REMOTE_PASSWORD="$PASSWORD" \
DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH="$TARGET_FACTS_PATH" \
DEVE_MOBILE_ANDROID_EVIDENCE_PATH="$EVIDENCE_PATH" \
DEVE_MOBILE_ANDROID_REMOTE_TIMEOUT_MS="$(((GLOBAL_DEADLINE - SECONDS) * 1000))" \
timeout "$((GLOBAL_DEADLINE - SECONDS))" \
  node "$ROOT_DIR/scripts/smoke-mobile-android-remote-browser.mjs"; then
  :
else
  JOURNEY_STATUS=$?
  android_startup_diagnostics_collect "$APP_ID" \
    || echo "mobile-android-remote-browser-smoke: journey diagnostics collection failed" >&2
  echo "mobile-android-remote-browser-smoke: journey failed with status $JOURNEY_STATUS" >&2
  exit "$JOURNEY_STATUS"
fi

LOGCAT="$(adb_cmd logcat -d 2>/dev/null | tr -d '\r')"
printf '%s\n' "$LOGCAT" \
  | grep -F "deve_mobile RemoteBrowser recovered to fresh LocalBackend runtime" >/dev/null \
  || fail "native recovery did not establish a fresh LocalBackend runtime"
echo "mobile-android-remote-browser-smoke: app_id=$APP_ID serial=$SERIAL pid=$PID"
echo "mobile-android-remote-browser-smoke: ok"
