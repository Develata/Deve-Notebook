#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-app-process-readiness.sh"

temporary="$(mktemp -d)"
cleanup() { rm -rf "$temporary"; }
trap cleanup EXIT

test_fail() {
  echo "android-app-process-readiness.test: $*" >&2
  exit 1
}

process_clock="$temporary/process-clock"
process_counter="$temporary/process-counter"
process_sleeps="$temporary/process-sleeps"
probe_stderr="$temporary/probe-stderr"
PROCESS_MODE=""

process_now() {
  case "$PROCESS_MODE" in
    clock-fail) return 42 ;;
    clock-invalid) printf 'not-a-clock\n'; return 0 ;;
  esac
  cat "$process_clock"
}

process_probe_count() {
  local count
  count="$(cat "$process_counter")"
  count=$((count + 1))
  printf '%s\n' "$count" >"$process_counter"
  printf '%s\n' "$count"
}

process_probe() {
  local count
  count="$(process_probe_count)"
  case "$PROCESS_MODE:$count" in
    launch-race:1 | launch-race:3 | stable:* | sleep-fail:*) printf '4659\n' ;;
    launch-race:2 | absent:* | disappeared:2 | disappeared:3 | observe-gap:1 | observe-absent:*) return 1 ;;
    pid-switch:1 | disappeared:1) printf '4659\n' ;;
    pid-switch:2 | observe-replaced:*) printf '4660\n' ;;
    observe-gap:2) printf '4659\n' ;;
    probe-timeout:*) return 124 ;;
    probe-transport:*) return "$ANDROID_APP_PROCESS_TRANSPORT_FAILURE_STATUS" ;;
    invalid:*) printf '4659 4660\n' ;;
    *) test_fail "unexpected process probe mode/count: $PROCESS_MODE/$count" ;;
  esac
}

sleep() {
  printf '%s\n' "$1" >>"$process_sleeps"
  [[ "$PROCESS_MODE" != "sleep-fail" ]] || return 41
  local clock
  clock="$(cat "$process_clock")"
  printf '%s\n' "$((clock + $1))" >"$process_clock"
}

reset_process_case() {
  printf '0\n' >"$process_clock"
  printf '0\n' >"$process_counter"
  : >"$process_sleeps"
  : >"$probe_stderr"
  ANDROID_APP_PROCESS_STABLE_PID=""
  ANDROID_APP_PROCESS_CURRENT_PID=""
  ANDROID_APP_PROCESS_MISSING_SAMPLES=0
  ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="not-probed"
}

reset_process_case
PROCESS_MODE="launch-race"
android_app_process_wait_stable 10 process_probe process_now \
  || test_fail "transient empty pid sample must not reject a stable launch"
[[ "$ANDROID_APP_PROCESS_STABLE_PID" == "4659" ]] \
  || test_fail "launch race admitted the wrong stable PID"
[[ "$(cat "$process_counter")" == "3" ]] \
  || test_fail "launch race did not re-admit the anchored PID after one empty sample"

reset_process_case
PROCESS_MODE="pid-switch"
set +e
android_app_process_wait_stable 10 process_probe process_now
status=$?
set -e
[[ "$status" == "1" ]] || test_fail "startup PID replacement must fail, got $status"
[[ "$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE" == "process=replaced initial-pid=4659 current-pid=4660" ]] \
  || test_fail "startup PID replacement evidence was not preserved"

reset_process_case
PROCESS_MODE="disappeared"
set +e
android_app_process_wait_stable 10 process_probe process_now
status=$?
set -e
[[ "$status" == "1" ]] || test_fail "two missing samples after PID admission must fail, got $status"
[[ "$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE" == "process=absent-after-candidate pid=4659 missing-samples=2/2" ]] \
  || test_fail "post-candidate disappearance evidence was not preserved"

reset_process_case
PROCESS_MODE="absent"
set +e
android_app_process_wait_stable 3 process_probe process_now
status=$?
set -e
[[ "$status" == "124" ]] || test_fail "absent process must expire with status 124, got $status"

for failure_mode in probe-timeout invalid clock-fail clock-invalid sleep-fail; do
  reset_process_case
  PROCESS_MODE="$failure_mode"
  set +e
  android_app_process_wait_stable 10 process_probe process_now
  status=$?
  set -e
  [[ "$status" != "0" ]] || test_fail "$failure_mode must fail closed"
done

reset_process_case
set +e
android_app_process_wait_stable invalid process_probe process_now
status=$?
set -e
[[ "$status" == "1" && "$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE" == "deadline=invalid" ]] \
  || test_fail "invalid deadline was not rejected"

reset_process_case
PROCESS_MODE="observe-gap"
android_app_process_observe_anchored 4659 process_probe \
  || test_fail "one post-admission bookkeeping gap must be tolerated"
[[ -z "$ANDROID_APP_PROCESS_CURRENT_PID" && "$ANDROID_APP_PROCESS_MISSING_SAMPLES" == "1" ]] \
  || test_fail "one bookkeeping gap did not preserve the anchored identity"
android_app_process_observe_anchored 4659 process_probe \
  || test_fail "the anchored PID must recover after one bookkeeping gap"
[[ "$ANDROID_APP_PROCESS_CURRENT_PID" == "4659" && "$ANDROID_APP_PROCESS_MISSING_SAMPLES" == "0" ]] \
  || test_fail "matching post-gap PID did not reset missing samples"

reset_process_case
PROCESS_MODE="observe-replaced"
set +e
android_app_process_observe_anchored 4659 process_probe
status=$?
set -e
[[ "$status" == "1" && "$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE" == "process=replaced initial-pid=4659 current-pid=4660" ]] \
  || test_fail "post-admission PID replacement was not rejected"

reset_process_case
PROCESS_MODE="observe-absent"
android_app_process_observe_anchored 4659 process_probe \
  || test_fail "first post-admission empty sample must remain provisional"
set +e
android_app_process_observe_anchored 4659 process_probe
status=$?
set -e
[[ "$status" == "1" && "$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE" == "process=absent-after-admission pid=4659 missing-samples=2/2" ]] \
  || test_fail "continued post-admission absence was not rejected"

for failure_mode in probe-timeout probe-transport; do
  reset_process_case
  PROCESS_MODE="$failure_mode"
  expected_status=124
  [[ "$failure_mode" != "probe-transport" ]] \
    || expected_status="$ANDROID_APP_PROCESS_TRANSPORT_FAILURE_STATUS"
  set +e
  android_app_process_observe_anchored 4659 process_probe
  status=$?
  set -e
  [[ "$status" == "$expected_status" \
    && "$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE" == "probe=failed status=$status" ]] \
    || test_fail "post-admission $failure_mode status was not preserved"
done

MOCK_ADB_MODE=""
mock_adb() {
  case "$MOCK_ADB_MODE" in
    match) printf '4659 4660\r\n' ;;
    absent) return 1 ;;
    transport) echo 'error: device offline' >&2; return 1 ;;
    timeout) return 124 ;;
    *) return 9 ;;
  esac
}

MOCK_ADB_MODE="match"
[[ "$(android_app_process_pidof_probe mock_adb dev.deve.notebook.mobile)" == "4659" ]] \
  || test_fail "pidof probe did not canonicalize the first PID"
MOCK_ADB_MODE="absent"
set +e
android_app_process_pidof_probe mock_adb dev.deve.notebook.mobile
status=$?
set -e
[[ "$status" == "1" ]] || test_fail "ordinary pidof absence must stay status 1"
MOCK_ADB_MODE="transport"
set +e
android_app_process_pidof_probe mock_adb dev.deve.notebook.mobile 2>"$probe_stderr"
status=$?
set -e
[[ "$status" == "$ANDROID_APP_PROCESS_TRANSPORT_FAILURE_STATUS" ]] \
  || test_fail "status-1 transport failure was not disambiguated"
grep -Fq 'error: device offline' "$probe_stderr" \
  || test_fail "transport failure evidence was not preserved"
MOCK_ADB_MODE="timeout"
set +e
android_app_process_pidof_probe mock_adb dev.deve.notebook.mobile
status=$?
set -e
[[ "$status" == "124" ]] || test_fail "pidof timeout status was not preserved"

echo "android-app-process-readiness.test: ok"
