#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/check-mobile-android-install-startup-smoke.sh
source "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"

temporary="$(mktemp -d)"
cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT

operations="$temporary/operations"
stderr_file="$temporary/stderr"
MOCK_APP_PID=""
MOCK_DIAG_MODE="success"
SECRET_SENTINEL="test-secret-sentinel-5a3f9c"

test_fail() {
  printf 'android-startup-diagnostics.test: %s\n' "$*" >&2
  exit 1
}

app_pid() {
  printf '%s\n' "$MOCK_APP_PID"
}

emit_bytes() {
  local count="$1"
  local filler
  filler="$(printf 'ActivityManager: filler %.0s' $(seq 1 $((count / 24 + 1))))"
  printf '%s' "${filler:0:$count}"
}

android_startup_diag_adb() {
  local timeout_secs="$1"
  shift
  printf 'timeout %s\n' "$timeout_secs" >>"$operations"
  printf 'cmd %s\n' "$*" >>"$operations"
  case "$*" in
    "logcat -b main,system,crash -c")
      [[ "$MOCK_DIAG_MODE" != "clear-fail"* ]] || return 1
      return 0
      ;;
    "shell date +%m-%d %H:%M:%S.000")
      case "$MOCK_DIAG_MODE" in
        clear-fail-marker) printf '07-27 10:11:12.000\n' ;;
        clear-fail-garbage) printf 'garbage; rm -rf /\n' ;;
        *) return 1 ;;
      esac
      return 0
      ;;
    "shell dumpsys activity exit-info dev.deve.notebook.mobile")
      case "$MOCK_DIAG_MODE" in
        all-fail) printf 'adb: device offline\n'; return 7 ;;
        exit-info-unsupported) printf "Can't find service: activity_exit_info\n"; return 255 ;;
        oversized) emit_bytes 8192 ;;
        *) printf 'ApplicationExitInfo reason=CRASH pid=1234\n' ;;
      esac
      return 0
      ;;
    "logcat -b crash -d -v threadtime" | "logcat -b crash -d -v threadtime -T "*)
      case "$MOCK_DIAG_MODE" in
        all-fail) printf 'adb: device offline\n'; return 7 ;;
        oversized) emit_bytes 8192 ;;
        *) printf 'FATAL EXCEPTION: main dev.deve.notebook.mobile\n' ;;
      esac
      return 0
      ;;
    "logcat -b main,system -d -v threadtime" | "logcat -b main,system -d -v threadtime -T "*)
      case "$MOCK_DIAG_MODE" in
        all-fail) printf 'adb: device offline\n'; return 7 ;;
        oversized) emit_bytes 8192 ;;
        *)
          printf 'ActivityManager: Process dev.deve.notebook.mobile has died\n'
          printf 'deve_mobile initial native session handoff failed closed: android_native_cookie_callback_timeout\n'
          printf 'deve_mobile initial native session handoff failed closed: android_initial_webview_admission_timeout\n'
          printf 'deve_mobile native session cookie handoff failed closed: android_native_cookie_not_retained\n'
          printf 'DeveMobile: deve_mobile presentation checkpoint: android_system_gesture_insets_unavailable\n'
          printf 'DeveMobile: deve_mobile ui back checkpoint: android_ui_back_root_backgrounded\n'
          printf 'RustStdoutStderr: deve_mobile RemoteBrowser recovered to fresh LocalBackend runtime recovery_id=7\n'
          printf '%s DeveMobile: deve_mobile presentation checkpoint: android_system_gesture_insets_unavailable\n' "$SECRET_SENTINEL"
          printf '%s RustStdoutStderr: deve_mobile RemoteBrowser recovered to fresh LocalBackend runtime recovery_id=7\n' "$SECRET_SENTINEL"
          printf '%s DeveMobile: deve_mobile initial native session handoff failed closed: android_native_cookie_callback_timeout\n' "$SECRET_SENTINEL"
          printf 'deve_mobile initial native session handoff failed closed: %s\n' "$SECRET_SENTINEL"
          printf 'deve_mobile native session cookie handoff failed closed: %s\n' "$SECRET_SENTINEL"
          printf 'dev.deve.notebook.mobile deve_mobile native session cookie handoff failed closed: %s\n' "$SECRET_SENTINEL"
          printf 'dev.deve.notebook.mobile deve_mobile presentation checkpoint: %s\n' "$SECRET_SENTINEL"
          printf 'dev.deve.notebook.mobile deve_mobile RemoteBrowser recovered to fresh LocalBackend runtime recovery_id=7 %s\n' "$SECRET_SENTINEL"
          ;;
      esac
      return 0
      ;;
    "shell dumpsys activity processes dev.deve.notebook.mobile")
      case "$MOCK_DIAG_MODE" in
        all-fail) printf 'adb: device offline\n'; return 7 ;;
        oversized) emit_bytes 8192 ;;
        *) printf 'no process record for dev.deve.notebook.mobile\n' ;;
      esac
      return 0
      ;;
    *)
      printf 'unexpected diagnostic command\n' >&2
      return 97
      ;;
  esac
}

assert_all_diag_calls_time_bounded() {
  local bad
  bad="$(awk '$1 == "timeout" && $2 !~ /^[1-9][0-9]*$/ { print; exit }' "$operations")"
  [[ -z "$bad" ]] || test_fail "diagnostic command without a positive timeout: $bad"
}

run_missing_pid_case() {
  local mode="$1"
  local status=0
  : >"$operations"
  MOCK_APP_PID=""
  MOCK_DIAG_MODE="$mode"
  set +e
  (require_app_running_after_launch) 2>"$stderr_file"
  status=$?
  set -e
  [[ "$status" == "1" ]] || test_fail "$mode: expected status 1, got $status"
  grep -Fq "Android app did not remain running after launch: dev.deve.notebook.mobile" "$stderr_file" \
    || test_fail "$mode: primary process-exit failure was replaced or lost"
  [[ "$(tail -n 1 "$stderr_file")" == *"Android app did not remain running after launch"* ]] \
    || test_fail "$mode: primary failure must be the final reported line"
  assert_all_diag_calls_time_bounded
}

# Success path: diagnostics must not run and must not print anything.
: >"$operations"
MOCK_APP_PID="4321"
MOCK_DIAG_MODE="success"
(require_app_running_after_launch) 2>"$stderr_file" \
  || test_fail "running app must pass the startup check"
[[ ! -s "$operations" ]] || test_fail "success path invoked diagnostic adb commands"
if grep -q "android-startup-diagnostics:" "$stderr_file"; then
  test_fail "success path emitted failure diagnostics"
fi

# Missing pid: diagnostics run, are bounded, and never mask the primary failure.
run_missing_pid_case success
grep -Fq -- "--- activity exit-info" "$stderr_file" || test_fail "exit-info section missing"
grep -Fq -- "--- crash buffer logcat" "$stderr_file" || test_fail "crash buffer section missing"
grep -Fq -- "--- recent runtime logcat" "$stderr_file" || test_fail "runtime logcat section missing"
grep -Fq -- "--- app process state" "$stderr_file" || test_fail "process state section missing"
grep -Fq "ApplicationExitInfo reason=CRASH" "$stderr_file" || test_fail "exit-info evidence missing"
grep -Fq "deve_mobile initial native session handoff failed closed: android_native_cookie_callback_timeout" "$stderr_file" \
  || test_fail "fixed native session handoff category missing from bounded diagnostics"
grep -Fq "deve_mobile initial native session handoff failed closed: android_initial_webview_admission_timeout" "$stderr_file" \
  || test_fail "fixed WebView admission category missing from bounded diagnostics"
grep -Fq "deve_mobile native session cookie handoff failed closed: android_native_cookie_not_retained" "$stderr_file" \
  || test_fail "fixed platform cookie category missing from bounded diagnostics"
grep -Fq "deve_mobile presentation checkpoint: android_system_gesture_insets_unavailable" "$stderr_file" \
  || test_fail "fixed Android presentation category missing from bounded diagnostics"
grep -Fq "deve_mobile ui back checkpoint: android_ui_back_root_backgrounded" "$stderr_file" \
  || test_fail "fixed Android UI Back category missing from bounded diagnostics"
grep -Fq "deve_mobile RemoteBrowser recovered to fresh LocalBackend runtime recovery_id=7" "$stderr_file" \
  || test_fail "fixed Android native recovery completion missing from bounded diagnostics"
if grep -Fq "$SECRET_SENTINEL" "$stderr_file"; then
  test_fail "unknown native session handoff suffix leaked through bounded diagnostics"
fi
expected_calls="$(grep -c '^cmd ' "$operations")"
[[ "$expected_calls" == "4" ]] || test_fail "expected 4 diagnostic commands, got $expected_calls"

# Unsupported exit-info stays nonfatal for the remaining diagnostics.
run_missing_pid_case exit-info-unsupported
grep -Fq "Can't find service: activity_exit_info" "$stderr_file" \
  || test_fail "unsupported exit-info output was dropped"
grep -Fq -- "--- app process state" "$stderr_file" \
  || test_fail "unsupported exit-info stopped later diagnostic sections"

# Total diagnostic failure never replaces the primary failure.
run_missing_pid_case all-fail
grep -Fq "section command failed (nonfatal)" "$stderr_file" \
  || test_fail "failed diagnostic sections must be reported as nonfatal"

# Combined output is capped by the explicit byte budget.
ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES=4096
run_missing_pid_case oversized
grep -Fq "output budget exhausted before section" "$stderr_file" \
  || test_fail "budget exhaustion was not reported"
reported_bytes="$(sed -n 's/^android-startup-diagnostics: done (bytes=\([0-9]*\) budget=4096)$/\1/p' "$stderr_file")"
[[ -n "$reported_bytes" ]] || test_fail "capped run did not report its byte usage"
(( reported_bytes <= 4096 )) || test_fail "diagnostic output exceeded its budget: $reported_bytes"
ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES=131072

# Secret-like environment values never enter diagnostic output.
export AUTH_SECRET="$SECRET_SENTINEL"
export AUTH_PASS="$SECRET_SENTINEL"
export DEVE_REMOTE_FIXTURE_PASSWORD="$SECRET_SENTINEL"
run_missing_pid_case success
if grep -Fq "$SECRET_SENTINEL" "$stderr_file"; then
  test_fail "diagnostics printed a secret-like environment value"
fi
unset AUTH_SECRET AUTH_PASS DEVE_REMOTE_FIXTURE_PASSWORD

# Prepare: the non-destructive device-time marker is the default isolation.
: >"$operations"
MOCK_DIAG_MODE="clear-fail-marker"
android_startup_diagnostics_prepare "$APP_ID" 2>"$stderr_file" \
  || test_fail "marker prepare must never fail the gate"
[[ "$ANDROID_STARTUP_DIAG_LOGCAT_MARKER" == "07-27 10:11:12.000" ]] \
  || test_fail "device-time marker was not recorded"
if grep -q '^cmd logcat -b main,system,crash -c$' "$operations"; then
  test_fail "a usable device-time marker must not clear ambient logcat buffers"
fi
run_missing_pid_case clear-fail-marker
grep -q -- '-T 07-27 10:11:12.000$' "$operations" \
  || test_fail "marker was not applied to post-exit logcat reads"

# Prepare: marker unavailable falls back to clearing the logcat buffers.
: >"$operations"
MOCK_DIAG_MODE="success"
android_startup_diagnostics_prepare "$APP_ID" 2>"$stderr_file" \
  || test_fail "clear fallback must never fail the gate"
[[ -z "$ANDROID_STARTUP_DIAG_LOGCAT_MARKER" ]] || test_fail "failed date probe must not set a marker"
grep -q '^cmd logcat -b main,system,crash -c$' "$operations" \
  || test_fail "marker-unavailable prepare did not clear logcat buffers"
grep -Fq "cleared logcat buffers instead" "$stderr_file" \
  || test_fail "clear fallback must be reported"

# Prepare: malformed device time is rejected instead of reaching logcat -T.
: >"$operations"
MOCK_DIAG_MODE="clear-fail-garbage"
android_startup_diagnostics_prepare "$APP_ID" 2>"$stderr_file" \
  || test_fail "garbage marker handling must never fail the gate"
[[ -z "$ANDROID_STARTUP_DIAG_LOGCAT_MARKER" ]] || test_fail "malformed device time must be rejected"
grep -Fq "diagnostic setup failed (nonfatal)" "$stderr_file" \
  || test_fail "failed diagnostic setup must be reported"

# Invalid budget/timeout overrides are clamped back to bounded defaults.
ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS="not-a-number"
ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES="99999999999"
run_missing_pid_case success
grep -Fq "invalid command timeout; using 20s" "$stderr_file" \
  || test_fail "invalid timeout override was not reported"
grep -Fq "invalid output budget; using 131072 bytes" "$stderr_file" \
  || test_fail "invalid budget override was not reported"
grep -Fq "budget=131072" "$stderr_file" \
  || test_fail "invalid budget override was not clamped to the default"
bad_timeout="$(awk '$1 == "timeout" && $2 != "20" { print; exit }' "$operations")"
[[ -z "$bad_timeout" ]] \
  || test_fail "invalid timeout override was not clamped to the default: $bad_timeout"
ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS=20
ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES=131072

# The verified pid is exported to the primary success output path.
: >"$operations"
MOCK_APP_PID="4321"
MOCK_DIAG_MODE="success"
APP_RUNNING_PID=""
require_app_running_after_launch 2>"$stderr_file" \
  || test_fail "running app must pass the startup check in the parent shell"
[[ "$APP_RUNNING_PID" == "4321" ]] \
  || test_fail "APP_RUNNING_PID was not propagated to the caller"

echo "android-startup-diagnostics.test: ok"
