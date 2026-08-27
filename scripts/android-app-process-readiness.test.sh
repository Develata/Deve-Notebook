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
process_budgets="$temporary/process-budgets"
probe_stderr="$temporary/probe-stderr"
mock_adb_args="$temporary/mock-adb-args"
hard_timeout_args="$temporary/hard-timeout-args"
real_timeout_stderr="$temporary/real-timeout-stderr"
real_child_kill_stderr="$temporary/real-child-kill-stderr"
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
  local remaining_secs="${1:-}"
  local count
  if [[ -n "$remaining_secs" ]]; then
    [[ "$remaining_secs" =~ ^[1-9][0-9]*$ ]] \
      || test_fail "process probe received an invalid remaining budget: $remaining_secs"
    printf '%s\n' "$remaining_secs" >>"$process_budgets"
  fi
  count="$(process_probe_count)"
  case "$PROCESS_MODE:$count" in
    launch-race:1 | launch-race:3 | stable:* | sleep-fail:*) printf '4659\n' ;;
    late-success:*)
      printf '%s\n' "$(($(cat "$process_clock") + remaining_secs))" >"$process_clock"
      printf '4659\n'
      ;;
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
  : >"$process_budgets"
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
[[ "$(paste -sd, "$process_budgets")" == "10,9,8" ]] \
  || test_fail "startup probes did not receive the shrinking absolute-deadline budget"

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

reset_process_case
PROCESS_MODE="late-success"
set +e
android_app_process_wait_stable 3 process_probe process_now
status=$?
set -e
[[ "$status" == "124" ]] || test_fail "a PID sampled at the deadline must not be admitted, got $status"
[[ -z "$ANDROID_APP_PROCESS_STABLE_PID" \
  && "$ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE" == "deadline=expired stable-samples=0/2" ]] \
  || test_fail "post-probe deadline expiry did not fail before PID admission"
[[ ! -s "$process_sleeps" ]] \
  || test_fail "post-probe deadline expiry performed an out-of-budget poll sleep"

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
  printf '%s\n' "$*" >>"$mock_adb_args"
  case "$MOCK_ADB_MODE" in
    match) printf '4659 4660\r\n' ;;
    absent) return 1 ;;
    transport) echo 'error: device offline' >&2; return 1 ;;
    timeout) return 124 ;;
    *) return 9 ;;
  esac
}

MOCK_ADB_MODE="match"
: >"$mock_adb_args"
[[ "$(android_app_process_pidof_probe mock_adb dev.deve.notebook.mobile)" == "4659" ]] \
  || test_fail "pidof probe did not canonicalize the first PID"
[[ "$(tail -n 1 "$mock_adb_args")" == "shell pidof dev.deve.notebook.mobile" ]] \
  || test_fail "pidof probe unexpectedly changed the legacy adb callback signature"
[[ "$(android_app_process_pidof_probe mock_adb dev.deve.notebook.mobile 7)" == "4659" ]] \
  || test_fail "deadline-aware pidof probe did not preserve the canonical PID"
[[ "$(tail -n 1 "$mock_adb_args")" == "7 shell pidof dev.deve.notebook.mobile" ]] \
  || test_fail "pidof probe did not pass the remaining deadline to the adb callback"
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

# Entry-point regression: the minified release startup proof must consume the
# shared stable-PID state machine, not regress to one delayed process snapshot.
(
  source "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
  reset_process_case
  PROCESS_MODE="launch-race"
  STARTUP_WAIT_SECS=10
  app_process_probe() { process_probe; }
  app_process_readiness_now() { process_now; }
  require_app_running_after_launch
  [[ "$APP_RUNNING_PID" == "4659" ]] \
    || test_fail "release startup proof did not admit the stable PID after one bookkeeping gap"
  [[ "$(cat "$process_counter")" == "3" ]] \
    || test_fail "release startup proof bypassed shared stable-PID admission"
)

# The release entry point must impose the remaining startup budget itself;
# the broader ADB command timeout is not allowed to extend this proof.
(
  source "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
  android_tool_path() { printf '/mock/adb\n'; }
  MOCK_TIMEOUT_EXPIRED=1
  timeout() {
    printf '%s\n' "$*" >"$hard_timeout_args"
    if [[ "$MOCK_TIMEOUT_EXPIRED" == "1" ]]; then
      printf "timeout: sending signal KILL to command '/mock/adb'\n" >&2
    fi
    return 137
  }
  set +e
  adb_with_startup_deadline 7 shell pidof dev.deve.notebook.mobile
  status=$?
  set -e
  [[ "$status" == "124" ]] \
    || test_fail "hard startup deadline did not normalize SIGKILL timeout to status 124"
  [[ "$(cat "$hard_timeout_args")" == "--verbose --signal=KILL 7s /mock/adb shell pidof dev.deve.notebook.mobile" ]] \
    || test_fail "release startup probe did not enforce its exact remaining hard deadline"

  MOCK_TIMEOUT_EXPIRED=0
  set +e
  adb_with_startup_deadline 7 shell pidof dev.deve.notebook.mobile
  status=$?
  set -e
  [[ "$status" == "137" ]] \
    || test_fail "child-originated SIGKILL was incorrectly classified as deadline expiry"
)

# Exercise the installed GNU timeout rather than only the wiring mock. A real
# deadline expiry and a child-originated SIGKILL must remain distinguishable.
(
  source "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
  bash_path="$(command -v bash)"
  android_tool_path() { printf '%s\n' "$bash_path"; }
  set +e
  adb_with_startup_deadline 1 -c 'sleep 5' 2>"$real_timeout_stderr"
  timeout_status=$?
  adb_with_startup_deadline 1 -c 'kill -KILL $$' 2>"$real_child_kill_stderr"
  child_kill_status=$?
  set -e
  [[ "$timeout_status" == "124" ]] \
    || test_fail "real GNU hard timeout was not normalized to deadline status 124"
  [[ "$child_kill_status" == "137" ]] \
    || test_fail "real child SIGKILL was incorrectly normalized to deadline expiry"
  if grep -Fq "timeout: sending signal KILL to command" "$real_timeout_stderr"; then
    test_fail "GNU timeout implementation marker leaked through the startup probe boundary"
  fi
)

echo "android-app-process-readiness.test: ok"
