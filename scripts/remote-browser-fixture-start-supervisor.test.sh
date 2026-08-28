#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-startup-state.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-startup-state.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-supervisor.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-supervisor.sh"
export DEVE_REMOTE_FIXTURE_TEST_MODE=1

fail() {
  printf 'remote-browser-fixture-start-supervisor.test: %s\n' "$*" >&2
  exit 1
}

test_progress() {
  [[ "${DEVE_REMOTE_FIXTURE_TEST_PROGRESS:-0}" != 1 ]] \
    || printf 'remote-browser-fixture-start-supervisor.test: phase=%s\n' "$1" >&2
}

wait_for_test_marker() {
  local marker="$1"
  local supervisor_pid="$2"
  local label="$3"
  for _ in $(seq 1 200); do
    [[ -f "$marker" ]] && return 0
    remote_fixture_pid_active "$supervisor_pid" || break
    sleep 0.01
  done
  fail "$label did not become ready"
}

wait_for_test_process_exit() {
  local pid="$1"
  local label="$2"
  for _ in $(seq 1 200); do
    remote_fixture_pid_active "$pid" || return 0
    sleep 0.02
  done
  fail "$label remained active"
}

assert_test_process_group_empty() {
  local group_id="$1"
  local label="$2"
  local member
  while read -r member; do
    if [[ -n "$member" ]] && remote_fixture_pid_active "$member"; then
      fail "$label retained active process-group member $member"
    fi
  done < <(remote_fixture_process_group_members "$group_id")
}

temporary="$(mktemp -d)"
declare -a test_owned_processes=()
register_test_process() {
  local pid="$1"
  local token=""
  for _ in $(seq 1 20); do
    token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
    [[ -n "$token" ]] && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.01
  done
  [[ -z "$token" ]] || test_owned_processes+=("$pid|$token")
}
cleanup_test_processes() {
  local original_status=$?
  trap - EXIT INT TERM
  local entry pid token active_status
  for entry in "${test_owned_processes[@]}"; do
    pid="${entry%%|*}"
    token="${entry#*|}"
    active_status=0
    remote_fixture_start_worker_active "$pid" "$token" || active_status=$?
    if ((active_status == 0)); then
      if remote_fixture_stop_bounded_tree "supervisor test process" "$pid" 0 >/dev/null 2>&1 \
        && ! remote_fixture_pid_active "$pid"; then
        wait "$pid" 2>/dev/null || true
      else
        printf 'remote-browser-fixture-start-supervisor.test: bounded cleanup failed for PID %s\n' \
          "$pid" >&2
        original_status=1
      fi
    elif ((active_status == 3)); then
      printf 'remote-browser-fixture-start-supervisor.test: token probe unavailable for PID %s\n' \
        "$pid" >&2
      original_status=1
    fi
  done
  rm -rf -- "$temporary"
  exit "$original_status"
}
trap cleanup_test_processes EXIT INT TERM

# Missing token evidence plus a PID that is now absent is a completed root-last
# transition, not a token mismatch. The same evidence on a still-live PID must
# remain fail-closed.
pid_active_definition="$(declare -f remote_fixture_pid_active)"
process_token_definition="$(declare -f remote_fixture_process_token)"
eval "${pid_active_definition/remote_fixture_pid_active/original_remote_fixture_pid_active}"
eval "${process_token_definition/remote_fixture_process_token/original_remote_fixture_process_token}"
identity_race_pid=424242
remote_fixture_pid_active() {
  [[ "$1" != "$identity_race_pid" ]] || return 1
  original_remote_fixture_pid_active "$@"
}
remote_fixture_process_token() {
  [[ "$1" != "$identity_race_pid" ]] || return 1
  original_remote_fixture_process_token "$@"
}
identity_race_status=0
remote_fixture_root_identity_status "$identity_race_pid" expected-token \
  || identity_race_status=$?
[[ "$identity_race_status" == 1 ]] \
  || fail "absent root with missing token evidence was classified as token mismatch"
remote_fixture_pid_active() {
  [[ "$1" != "$identity_race_pid" ]] || return 0
  original_remote_fixture_pid_active "$@"
}
identity_race_status=0
remote_fixture_root_identity_status "$identity_race_pid" expected-token \
  || identity_race_status=$?
[[ "$identity_race_status" == 2 ]] \
  || fail "live root with missing token evidence did not fail closed"
eval "$pid_active_definition"
eval "$process_token_definition"
test_progress identity-race-complete

# A temporary /proc identity read failure while kill(0) still proves the PID
# exists is probe-unavailable, not child exit. The supervisor must never enter
# an exact blocking wait from that observation alone.
pid_active_definition="$(declare -f remote_fixture_pid_active)"
pid_exists_definition="$(declare -f remote_fixture_pid_exists)"
transient_identity_pid=434343
remote_fixture_pid_active() {
  [[ "$1" != "$transient_identity_pid" ]] || return 1
  original_remote_fixture_pid_active "$@"
}
remote_fixture_pid_exists() {
  [[ "$1" != "$transient_identity_pid" ]] || return 0
  kill -0 "$1" 2>/dev/null
}
transient_identity_status=0
remote_fixture_start_worker_active "$transient_identity_pid" expected-token \
  || transient_identity_status=$?
[[ "$transient_identity_status" == 3 ]] \
  || fail "live worker with a transient identity probe failure was classified as exited"
transient_root_status=0
remote_fixture_root_identity_status "$transient_identity_pid" expected-token \
  || transient_root_status=$?
[[ "$transient_root_status" == 2 ]] \
  || fail "live root with a transient identity probe failure was classified as exited"
eval "$pid_active_definition"
eval "$pid_exists_definition"
test_progress transient-identity-complete

# Process membership and token must originate from one tokenized snapshot.
# A later token probe would be vulnerable to composing an old relationship
# with a reused PID's new identity.
tokenized_descendants_definition="$(declare -f remote_fixture_tokenized_descendants_deepest)"
process_token_definition="$(declare -f remote_fixture_process_token)"
tokenized_snapshot_pid=434344
remote_fixture_tokenized_descendants_deepest() {
  printf '%s|777\n' "$tokenized_snapshot_pid"
}
remote_fixture_process_token() {
  [[ "$1" != "$tokenized_snapshot_pid" ]] \
    || fail "tokenized process-tree snapshot performed a later PID token lookup"
  original_remote_fixture_process_token "$@"
}
remote_fixture_capture_descendant_snapshot "tokenized snapshot test" 434300
[[ "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[*]}" == "$tokenized_snapshot_pid|777" ]] \
  || fail "tokenized process-tree snapshot did not preserve its atomic identity"
eval "$tokenized_descendants_definition"
eval "$process_token_definition"

# A live admission timer with unreadable identity must fail closed immediately;
# it cannot enter an exact wait and delay cancellation indefinitely.
root_identity_definition="$(declare -f remote_fixture_root_identity_status)"
observation_reap_definition="$(declare -f remote_fixture_reap_observation_tick)"
timer_probe_waited=0
remote_fixture_root_identity_status() { return 2; }
remote_fixture_reap_observation_tick() { timer_probe_waited=1; return 97; }
timer_probe_status=0
remote_fixture_stop_admission_publisher_timer 434345 expected-token \
  >/dev/null 2>&1 || timer_probe_status=$?
[[ "$timer_probe_status" != 0 && "$timer_probe_waited" == 0 ]] \
  || fail "live admission timer probe failure entered an exact wait"
eval "$root_identity_definition"
eval "$observation_reap_definition"

# A non-interactive Bash parent marks an asynchronously spawned command as
# SIGINT-ignored before exec. The formal adapter must restore the default
# disposition so the worker's typed Bash trap can observe INT.
signal_exec_entry="$temporary/signal-exec-entry.sh"
cat >"$signal_exec_entry" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
trap 'printf observed >"$DEVE_REMOTE_FIXTURE_SIGNAL_EXEC_OBSERVED"; exit 130' INT
printf ready >"$DEVE_REMOTE_FIXTURE_SIGNAL_EXEC_READY"
while :; do sleep 0.1; done
EOF
signal_exec_ready="$temporary/signal-exec-ready"
signal_exec_observed="$temporary/signal-exec-observed"
DEVE_REMOTE_FIXTURE_SIGNAL_EXEC_READY="$signal_exec_ready" \
  DEVE_REMOTE_FIXTURE_SIGNAL_EXEC_OBSERVED="$signal_exec_observed" \
  python3 "$REMOTE_FIXTURE_SIGNAL_EXEC" bash "$signal_exec_entry" &
signal_exec_pid="$!"
register_test_process "$signal_exec_pid"
wait_for_test_marker "$signal_exec_ready" "$signal_exec_pid" "signal exec adapter"
kill -INT "$signal_exec_pid" || fail "signal exec adapter worker exited before INT"
signal_exec_status=0
wait "$signal_exec_pid" || signal_exec_status=$?
[[ "$signal_exec_status" == 130 && -f "$signal_exec_observed" ]] \
  || fail "signal exec adapter did not restore Bash INT delivery"
test_progress signal-exec-complete

# The real startup journal grants cleanup only after complete identity preflight.
owned_state="$temporary/owned-state"
mkdir -p "$owned_state"
owned_fixture_id="feedfacefeedfacefeedfacefeedface"
printf '%s' "$owned_fixture_id" >"$owned_state/.fixture-owner"
remote_fixture_initialize_startup_state "$owned_state" "$owned_fixture_id"
sleep 300 &
owned_backend_pid="$!"
register_test_process "$owned_backend_pid"
owned_backend_token="$(remote_fixture_process_token "$owned_backend_pid")"
REMOTE_FIXTURE_STARTUP_SOURCE_KIND=executable
REMOTE_FIXTURE_STARTUP_BACKEND_PID="$owned_backend_pid"
REMOTE_FIXTURE_STARTUP_BACKEND_TOKEN="$owned_backend_token"
remote_fixture_save_startup_state start-backend
printf secret >"$owned_state/.password"
remote_fixture_recover_startup_state "$owned_state"
wait "$owned_backend_pid" 2>/dev/null || true
remote_fixture_pid_active "$owned_backend_pid" \
  && fail "startup recovery left its exact backend alive"
[[ ! -e "$owned_state/startup-state.json" && ! -e "$owned_state/.fixture-owner" \
  && ! -e "$owned_state/.password" ]] \
  || fail "successful startup recovery retained owned state or secret material"

mismatch_state="$temporary/mismatch-state"
mkdir -p "$mismatch_state"
mismatch_fixture_id="deadbeefdeadbeefdeadbeefdeadbeef"
printf '%s' "$mismatch_fixture_id" >"$mismatch_state/.fixture-owner"
remote_fixture_initialize_startup_state "$mismatch_state" "$mismatch_fixture_id"
sleep 300 &
mismatch_pid="$!"
register_test_process "$mismatch_pid"
REMOTE_FIXTURE_STARTUP_SOURCE_KIND=executable
REMOTE_FIXTURE_STARTUP_BACKEND_PID="$mismatch_pid"
REMOTE_FIXTURE_STARTUP_BACKEND_TOKEN="not-the-live-token"
remote_fixture_save_startup_state start-backend
printf preserve >"$mismatch_state/.password"
if remote_fixture_recover_startup_state "$mismatch_state" 2>"$mismatch_state/recovery.stderr"; then
  fail "startup recovery accepted a mismatched live process token"
fi
remote_fixture_pid_active "$mismatch_pid" \
  || fail "token mismatch stopped an unowned live process"
[[ -e "$mismatch_state/startup-state.json" && -e "$mismatch_state/.fixture-owner" \
  && -e "$mismatch_state/.password" ]] \
  || fail "token mismatch failed to preserve ownership evidence and secret material"
kill -TERM "$mismatch_pid" 2>/dev/null || true
wait "$mismatch_pid" 2>/dev/null || true

malformed_state="$temporary/malformed-state"
mkdir -p "$malformed_state"
malformed_sentinel="DO_NOT_ECHO_THIS_SECRET"
printf '%s' "feedfeedfeedfeedfeedfeedfeedfeed" >"$malformed_state/.fixture-owner"
printf '{"fixture_id":"%s","secret":"%s"' \
  "feedfeedfeedfeedfeedfeedfeedfeed" "$malformed_sentinel" \
  >"$malformed_state/startup-state.json"
printf preserve >"$malformed_state/.password"
if remote_fixture_recover_startup_state "$malformed_state" \
  2>"$malformed_state/recovery.stderr"; then
  fail "startup recovery accepted malformed ownership JSON"
fi
if grep -Fq "$malformed_sentinel" "$malformed_state/recovery.stderr"; then
  fail "malformed startup state leaked its content through diagnostics"
fi
[[ -e "$malformed_state/startup-state.json" && -e "$malformed_state/.fixture-owner" \
  && -e "$malformed_state/.password" ]] \
  || fail "malformed startup state failed to preserve ownership evidence"

symlink_target="$temporary/symlink-target"
symlink_alias="$temporary/symlink-alias"
mkdir -p "$symlink_target"
symlink_fixture_id="facefacefacefacefacefacefaceface"
printf '%s' "$symlink_fixture_id" >"$symlink_target/.fixture-owner"
remote_fixture_initialize_startup_state "$symlink_target" "$symlink_fixture_id"
printf preserve >"$symlink_target/.password"
if ln -s -- "$symlink_target" "$symlink_alias" 2>/dev/null && [[ -L "$symlink_alias" ]]; then
  if remote_fixture_recover_startup_state "$symlink_alias" \
    2>"$symlink_target/symlink-recovery.stderr"; then
    fail "startup recovery accepted a symlink state directory"
  fi
  [[ -e "$symlink_target/startup-state.json" && -e "$symlink_target/.fixture-owner" \
    && -e "$symlink_target/.password" ]] \
    || fail "symlink recovery modified its target ownership state"
else
  printf 'remote-browser-fixture-start-supervisor.test: symlink assertion unavailable on this host\n' >&2
fi

# Readiness may target only the still-direct, token-matching supervisor. A
# stale/reused numeric parent PID must fail before any USR1 is sent.
parent_token_state="$temporary/parent-token-state"
mkdir -p "$parent_token_state"
if DEVE_REMOTE_FIXTURE_TEST_ADMISSION_ATTEMPTS=1 \
  DEVE_REMOTE_FIXTURE_TEST_ADMISSION_DELAY=0.01 \
  remote_fixture_wait_startup_admission \
    "$parent_token_state" "$(printf '1%.0s' {1..32})" "$PPID" "mismatched-parent-token" \
    2>"$parent_token_state/admission.stderr"; then
  fail "startup admission signalled a mismatched supervisor process token"
fi
grep -Fq "startup supervisor process token changed before ownership admission" \
  "$parent_token_state/admission.stderr" \
  || fail "startup admission did not classify a mismatched supervisor token"

parent_relation_state="$temporary/parent-relation-state"
mkdir -p "$parent_relation_state"
if DEVE_REMOTE_FIXTURE_TEST_ADMISSION_ATTEMPTS=1 \
  DEVE_REMOTE_FIXTURE_TEST_ADMISSION_DELAY=0.01 \
  remote_fixture_wait_startup_admission \
    "$parent_relation_state" "$(printf '3%.0s' {1..32})" "$((PPID + 100000))" \
    "not-observed" 2>"$parent_relation_state/admission.stderr"; then
  fail "startup admission accepted a non-parent supervisor PID"
fi
grep -Fq "startup supervisor parent relation changed before ownership admission" \
  "$parent_relation_state/admission.stderr" \
  || fail "startup admission did not classify a changed direct-parent relation"

stale_admission_state="$temporary/stale-admission-state"
mkdir -p "$stale_admission_state"
stale_admission_id="$(printf '2%.0s' {1..32})"
printf '%s\n' "$stale_admission_id" >"$stale_admission_state/.startup-admitted"
if DEVE_REMOTE_FIXTURE_TEST_ADMISSION_ATTEMPTS=1 \
  DEVE_REMOTE_FIXTURE_TEST_ADMISSION_DELAY=0.01 \
  remote_fixture_wait_startup_admission \
    "$stale_admission_state" "$stale_admission_id" "$PPID" "stale-parent-token" \
    2>"$stale_admission_state/admission.stderr"; then
  fail "prepublished admission bypassed final supervisor identity validation"
fi
[[ -f "$stale_admission_state/.startup-admitted" ]] \
  || fail "stale supervisor identity consumed the admission marker"

# The supervisor tests below use a minimal state projection. The production
# startup/final state recovery path was exercised above.
remote_fixture_cancel_owned_state() {
  local state_dir="$1"
  if [[ -f "$state_dir/.test-published" ]]; then
    stop_fixture --state-dir "$state_dir"
  elif [[ -f "$state_dir/.test-startup" ]]; then
    rm -f -- "$state_dir/.test-startup"
    printf recovered >"$state_dir/.test-recovered"
  fi
  return 0
}
remote_fixture_admit_startup_state() {
  local state_dir="$1"
  local decision_file="$2"
  mkdir -p "$state_dir"
  if [[ -n "${DEVE_REMOTE_FIXTURE_TEST_ADMIT_SIGNAL:-}" ]]; then
    kill -s "$DEVE_REMOTE_FIXTURE_TEST_ADMIT_SIGNAL" \
      "${REMOTE_FIXTURE_START_ADMISSION_OWNER_PID:?}"
    sleep 0.05
  fi
  remote_fixture_publish_startup_admission \
    "$state_dir/.test-admitted" admitted "$decision_file"
}

# A completed observation tick has already surrendered its PID ownership to
# wait -n. The event path must reap harmlessly without signalling that number.
(
  worker_ready=0
  test_wait_check_injected=0
  remote_fixture_stop_supervisor_timer() {
    fail "completed observation tick was passed to the signalling stop helper"
  }
  sleep 1 &
  observation_test_worker="$!"
  remote_fixture_wait_supervisor_event "$observation_test_worker"
  [[ -z "$REMOTE_FIXTURE_SUPERVISOR_WAIT_PID" \
    && "$REMOTE_FIXTURE_SUPERVISOR_WAIT_STATUS" == 129 ]] \
    || fail "observation tick did not produce its retained bounded event"
  kill -TERM "$observation_test_worker" 2>/dev/null || true
  wait "$observation_test_worker" 2>/dev/null || true
)

# An exact observation-reap invariant failure is a supervisor failure, not a
# successful empty event that the caller may silently ignore.
(
  worker_ready=0
  test_wait_check_injected=0
  remote_fixture_reap_observation_tick() { return 1; }
  sleep 1 &
  observation_failure_worker="$!"
  if remote_fixture_wait_supervisor_event "$observation_failure_worker"; then
    fail "observation tick reap failure was reported as a successful event"
  fi
  kill -TERM "$observation_failure_worker" 2>/dev/null || true
  wait "$observation_failure_worker" 2>/dev/null || true
)

# Publisher completion statuses above 128 are exact child results, not proof
# that a job-table/process-substitution poll is needed or permitted.
(
  signal_status=0
  publisher_poll_marker="$temporary/publisher-job-poll"
  mkdir -p "$temporary/publisher-exit-143"
  remote_fixture_admit_startup_state() { return 143; }
  remote_fixture_job_active() {
    if [[ "$1" == "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID" ]]; then
      : >"$publisher_poll_marker"
    fi
    return 1
  }
  publisher_status=0
  remote_fixture_admit_start_worker "$temporary/publisher-exit-143" \
    || publisher_status=$?
  [[ "$publisher_status" == 143 ]] \
    || fail "publisher exit 143 was not retained exactly"
  [[ ! -e "$publisher_poll_marker" ]] \
    || fail "publisher exact-status path used a job-table poll"
)

# A parent signal may interrupt the first exact wait while the publisher is
# still live. Retrying the same retained child must preserve its eventual 0.
(
  signal_status=0
  publisher_wait_interrupted=0
  mkdir -p "$temporary/publisher-wait-interrupt"
  trap 'publisher_wait_interrupted=1' TERM
  DEVE_REMOTE_FIXTURE_TEST_ADMIT_SIGNAL=TERM \
    remote_fixture_admit_start_worker "$temporary/publisher-wait-interrupt"
  [[ "$publisher_wait_interrupted" == 1 ]] \
    || fail "publisher wait interruption did not reach the supervisor"
)

test_progress formal-publisher-start
# The formal publisher runs through the actual internal entry in an isolated
# group, publishes the real marker, and retires its PID/token/PGID capability.
formal_publisher_state="$temporary/formal-publisher"
formal_publisher_id="$(printf 'a%.0s' {1..32})"
mkdir -p "$formal_publisher_state"
printf '%s' "$formal_publisher_id" >"$formal_publisher_state/.fixture-owner"
remote_fixture_initialize_startup_state "$formal_publisher_state" "$formal_publisher_id"
(
  unset DEVE_REMOTE_FIXTURE_TEST_MODE
  signal_status=0
  DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$ROOT_DIR/scripts/remote-browser-fixture.sh"
  remote_fixture_admit_start_worker "$formal_publisher_state"
  [[ "$(tr -d '\r\n' <"$formal_publisher_state/.startup-admitted")" \
    == "$formal_publisher_id" ]] \
    || fail "formal admission publisher did not publish the exact fixture id"
  if ((REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP == 1)) \
    && remote_fixture_process_group_is_bound \
      "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID" \
      "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_TOKEN"; then
    fail "formal admission publisher retained a stale process-group capability"
  fi
)
remote_fixture_remove_startup_admission "$formal_publisher_state"
remote_fixture_remove_startup_state "$formal_publisher_state"
rm -f -- "$formal_publisher_state/.fixture-owner"

# Exercise cancellation, deadline, and parent-death cleanup through the same
# setsid + Python child-subreaper path used by formal Linux producers.
formal_stuck_entry="$temporary/formal-stuck-publisher.sh"
cat >"$formal_stuck_entry" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$PPID" >"${DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE:?}"
bash -c 'trap "" INT TERM; while :; do sleep 60; done' &
printf '%s\n' "$!" >"${DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_FILE:?}"
if [[ "${DEVE_REMOTE_FIXTURE_ADMISSION_TEST_SIGNAL_PARENT:-0}" == 1 ]]; then
  kill -TERM "${DEVE_REMOTE_FIXTURE_ADMISSION_SUPERVISOR_PID:?}"
fi
if [[ "${DEVE_REMOTE_FIXTURE_ADMISSION_TEST_EXIT_AFTER_SPAWN:-0}" == 1 ]]; then
  exit 0
fi
trap '' INT TERM
while :; do sleep 60; done
EOF
chmod +x "$formal_stuck_entry"

formal_cancel_state="$temporary/formal-cancel-publisher"
formal_cancel_root_file="$temporary/formal-cancel-root.pid"
formal_cancel_child_file="$temporary/formal-cancel-child.pid"
mkdir -p "$formal_cancel_state"
formal_cancel_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  trap 'remote_fixture_latch_start_cancel TERM' TERM
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=1 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE="$formal_cancel_root_file" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_FILE="$formal_cancel_child_file" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_SIGNAL_PARENT=1 \
    remote_fixture_admit_start_worker "$formal_cancel_state"
) || formal_cancel_status=$?
[[ "$formal_cancel_status" == 3 ]] \
  || fail "formal stuck cancelled publisher returned $formal_cancel_status instead of 3"
formal_cancel_root_pid="$(<"$formal_cancel_root_file")"
formal_cancel_child_pid="$(<"$formal_cancel_child_file")"
wait_for_test_process_exit "$formal_cancel_root_pid" "formal cancelled publisher root"
wait_for_test_process_exit "$formal_cancel_child_pid" "formal cancelled publisher descendant"
assert_test_process_group_empty "$formal_cancel_root_pid" "formal cancelled publisher"

formal_status_state="$temporary/formal-status-publisher"
formal_status_root_file="$temporary/formal-status-root.pid"
formal_status_child_file="$temporary/formal-status-child.pid"
mkdir -p "$formal_status_state"
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=1 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE="$formal_status_root_file" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_FILE="$formal_status_child_file" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_EXIT_AFTER_SPAWN=1 \
    remote_fixture_admit_start_worker "$formal_status_state"
)
formal_status_root_pid="$(<"$formal_status_root_file")"
formal_status_child_pid="$(<"$formal_status_child_file")"
wait_for_test_process_exit "$formal_status_root_pid" "status-settled publisher root"
wait_for_test_process_exit "$formal_status_child_pid" "status-settled publisher descendant"
assert_test_process_group_empty "$formal_status_root_pid" "status-settled publisher"

formal_fast_exit_subreaper="$temporary/formal-fast-exit-subreaper.py"
cat >"$formal_fast_exit_subreaper" <<'EOF'
import os
with open(os.environ["DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE"], "w", encoding="ascii") as root_file:
    root_file.write(f"{os.getpid()}\n")
raise SystemExit(125)
EOF
formal_fast_exit_state="$temporary/formal-fast-exit-publisher"
formal_fast_exit_root_file="$temporary/formal-fast-exit-root.pid"
mkdir -p "$formal_fast_exit_state"
formal_fast_exit_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  REMOTE_FIXTURE_BOUNDED_SUBREAPER="$formal_fast_exit_subreaper"
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=1 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE="$formal_fast_exit_root_file" \
    remote_fixture_admit_start_worker "$formal_fast_exit_state"
) || formal_fast_exit_status=$?
[[ "$formal_fast_exit_status" == 1 ]] \
  || fail "pre-admission exited publisher returned $formal_fast_exit_status instead of failure"
formal_fast_exit_root_pid="$(<"$formal_fast_exit_root_file")"
wait_for_test_process_exit "$formal_fast_exit_root_pid" "pre-admission exited publisher root"
assert_test_process_group_empty "$formal_fast_exit_root_pid" "pre-admission exited publisher"

# A broken subreaper that forks before parent capability admission must not be
# mistaken for an empty pretoken exit. The supervisor preserves controls and
# fails closed; this test then retires the synthetic orphan by exact token.
formal_pretoken_fork_subreaper="$temporary/formal-pretoken-fork-subreaper.py"
formal_pretoken_child_identity="$temporary/formal-pretoken-child.identity"
cat >"$formal_pretoken_fork_subreaper" <<'EOF'
import os
import signal
import time

child_pid = os.fork()
if child_pid == 0:
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    time.sleep(30)
    raise SystemExit(0)
with open(f"/proc/{child_pid}/stat", encoding="ascii") as stat_file:
    token = stat_file.read().rsplit(") ", 1)[1].split()[19]
with open(os.environ["DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_IDENTITY"], "w", encoding="ascii") as identity_file:
    identity_file.write(f"{child_pid}|{token}\n")
raise SystemExit(125)
EOF
formal_pretoken_fork_state="$temporary/formal-pretoken-fork-publisher"
mkdir -p "$formal_pretoken_fork_state"
formal_pretoken_fork_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  REMOTE_FIXTURE_BOUNDED_SUBREAPER="$formal_pretoken_fork_subreaper"
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORCE_TOKEN_UNAVAILABLE=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=1 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_IDENTITY="$formal_pretoken_child_identity" \
    remote_fixture_admit_start_worker "$formal_pretoken_fork_state"
) || formal_pretoken_fork_status=$?
[[ "$formal_pretoken_fork_status" == 1 ]] \
  || fail "pretoken nonempty publisher group returned $formal_pretoken_fork_status instead of failure"
[[ -s "$formal_pretoken_fork_state/.startup-admission-publisher.root" ]] \
  || fail "pretoken nonempty publisher group did not preserve root controls"
IFS='|' read -r formal_pretoken_child_pid formal_pretoken_child_token \
  <"$formal_pretoken_child_identity"
remote_fixture_signal_owned_identity "synthetic pretoken publisher child" \
  "$formal_pretoken_child_pid" "$formal_pretoken_child_token" KILL
wait_for_test_process_exit "$formal_pretoken_child_pid" "synthetic pretoken publisher child"
formal_pretoken_root_pid="$(<"$formal_pretoken_fork_state/.startup-admission-publisher.root")"
assert_test_process_group_empty "$formal_pretoken_root_pid" "synthetic pretoken publisher"
rm -f -- \
  "$formal_pretoken_fork_state/.startup-admission-publisher.root" \
  "$formal_pretoken_fork_state/.startup-admission-publisher.released" \
  "$formal_pretoken_fork_state/.startup-admission-publisher.failure" \
  "$formal_pretoken_fork_state/.startup-admission-publisher.launcher" \
  "$formal_pretoken_fork_state/.startup-admission-publisher.deadline" \
  "$formal_pretoken_fork_state/.startup-admission-publisher.root-admitted"

formal_deadline_state="$temporary/formal-deadline-publisher"
formal_deadline_root_file="$temporary/formal-deadline-root.pid"
formal_deadline_child_file="$temporary/formal-deadline-child.pid"
mkdir -p "$formal_deadline_state"
formal_deadline_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=0.2 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE="$formal_deadline_root_file" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_FILE="$formal_deadline_child_file" \
    remote_fixture_admit_start_worker "$formal_deadline_state"
) || formal_deadline_status=$?
[[ "$formal_deadline_status" == 1 ]] \
  || fail "formal stuck publisher deadline returned $formal_deadline_status instead of 1"
formal_deadline_root_pid="$(<"$formal_deadline_root_file")"
formal_deadline_child_pid="$(<"$formal_deadline_child_file")"
wait_for_test_process_exit "$formal_deadline_root_pid" "formal deadline publisher root"
wait_for_test_process_exit "$formal_deadline_child_pid" "formal deadline publisher descendant"
assert_test_process_group_empty "$formal_deadline_root_pid" "formal deadline publisher"

# The deadline child participates in the same O_EXCL decision. A publisher
# that reaches the decision only after the deadline cannot publish a marker.
formal_late_entry="$temporary/formal-late-publisher.sh"
cat >"$formal_late_entry" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
state_dir="${2:?}"
decision_path="${3:?}"
sleep 0.3
if (umask 077; set -o noclobber; : >"$decision_path") 2>/dev/null; then
  printf 'late\n' >"$state_dir/.late-admission-marker"
  exit 0
fi
exit 3
EOF
chmod +x "$formal_late_entry"
formal_late_state="$temporary/formal-late-publisher"
mkdir -p "$formal_late_state"
formal_late_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=0.1 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_late_entry" \
    remote_fixture_admit_start_worker "$formal_late_state"
) || formal_late_status=$?
[[ "$formal_late_status" == 1 ]] \
  || fail "late publisher returned $formal_late_status instead of deadline failure"
[[ ! -e "$formal_late_state/.late-admission-marker" ]] \
  || fail "late publisher won admission after the atomic deadline"

# A publisher may claim the shared decision before the deadline but publish
# status only afterwards. Delay observation until both children have settled;
# the independent marker, not wait -n argument order, must reject that status.
formal_post_deadline_entry="$temporary/formal-post-deadline-publisher.sh"
cat >"$formal_post_deadline_entry" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
decision_path="${3:?}"
(umask 077; set -o noclobber; : >"$decision_path") 2>/dev/null
sleep 0.2
exit 0
EOF
chmod +x "$formal_post_deadline_entry"
formal_post_deadline_state="$temporary/formal-post-deadline-publisher"
mkdir -p "$formal_post_deadline_state"
formal_post_deadline_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  original_observation_reap="$(declare -f remote_fixture_reap_observation_tick)"
  eval "${original_observation_reap/remote_fixture_reap_observation_tick/original_remote_fixture_reap_observation_tick}"
  delayed_observation=0
  remote_fixture_reap_observation_tick() {
    original_remote_fixture_reap_observation_tick "$@" || return 1
    if ((delayed_observation == 0)); then
      delayed_observation=1
      sleep 0.25
    fi
  }
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=0.1 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_post_deadline_entry" \
    remote_fixture_admit_start_worker "$formal_post_deadline_state"
) || formal_post_deadline_status=$?
[[ "$formal_post_deadline_status" == 1 ]] \
  || fail "post-deadline publisher status returned $formal_post_deadline_status instead of failure"

# The deadline may linearize while the parent is reading an already-published
# status. Recheck the independent marker after that read and before cleanup or
# acceptance, rather than treating the earlier absence check as a lease.
formal_status_window_entry="$temporary/formal-status-window-publisher.sh"
cat >"$formal_status_window_entry" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
decision_path="${3:?}"
(umask 077; set -o noclobber; : >"$decision_path") 2>/dev/null || exit 3
exit 0
EOF
chmod +x "$formal_status_window_entry"
formal_status_window_state="$temporary/formal-status-window-publisher"
mkdir -p "$formal_status_window_state"
formal_status_window_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  original_status_reader="$(declare -f remote_fixture_read_admission_publisher_status)"
  eval "${original_status_reader/remote_fixture_read_admission_publisher_status/original_remote_fixture_read_admission_publisher_status}"
  remote_fixture_read_admission_publisher_status() {
    original_remote_fixture_read_admission_publisher_status || return $?
    (umask 077; set -o noclobber; \
      : >"$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_DEADLINE_PATH") 2>/dev/null
  }
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=2 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_status_window_entry" \
    remote_fixture_admit_start_worker "$formal_status_window_state"
) || formal_status_window_status=$?
[[ "$formal_status_window_status" == 1 ]] \
  || fail "publisher status crossed a deadline marker created during status read"

formal_claimed_stuck_entry="$temporary/formal-claimed-stuck-publisher.sh"
cat >"$formal_claimed_stuck_entry" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
decision_path="${3:?}"
printf '%s\n' "$PPID" >"${DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE:?}"
(umask 077; set -o noclobber; : >"$decision_path") 2>/dev/null
trap '' INT TERM
while :; do sleep 60; done
EOF
chmod +x "$formal_claimed_stuck_entry"
formal_claimed_stuck_state="$temporary/formal-claimed-stuck-publisher"
formal_claimed_stuck_root_file="$temporary/formal-claimed-stuck-root.pid"
mkdir -p "$formal_claimed_stuck_state"
formal_claimed_stuck_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=0.1 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_claimed_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE="$formal_claimed_stuck_root_file" \
    remote_fixture_admit_start_worker "$formal_claimed_stuck_state"
) || formal_claimed_stuck_status=$?
[[ "$formal_claimed_stuck_status" == 1 ]] \
  || fail "decision-only publisher returned $formal_claimed_stuck_status instead of timeout failure"
formal_claimed_stuck_root_pid="$(<"$formal_claimed_stuck_root_file")"
wait_for_test_process_exit "$formal_claimed_stuck_root_pid" "decision-only publisher root"
assert_test_process_group_empty "$formal_claimed_stuck_root_pid" "decision-only publisher"

# `wait -n` does not promise which already-completed child it reports first.
# Prove the safety transition directly: consuming the deadline child retires
# both PID and token before any later trap/cleanup path can signal by number.
consumed_timer_pid=424242
consumed_timer_token=linux-starttime-123
remote_fixture_retire_consumed_admission_timer_identity \
  consumed_timer_pid consumed_timer_token
[[ -z "$consumed_timer_pid" && -z "$consumed_timer_token" ]] \
  || fail "consumed admission deadline ownership was not retired atomically"

# If the parent cannot acquire the root token, the gated subreaper must never
# fork Bash. Exiting the supervisor transfers root cleanup through PDEATHSIG.
formal_token_gate_state="$temporary/formal-token-gate-publisher"
formal_token_gate_root_file="$temporary/formal-token-gate-entry-root.pid"
formal_token_gate_child_file="$temporary/formal-token-gate-entry-child.pid"
mkdir -p "$formal_token_gate_state"
formal_token_gate_status=0
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORCE_TOKEN_UNAVAILABLE=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=2 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE="$formal_token_gate_root_file" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_FILE="$formal_token_gate_child_file" \
    remote_fixture_admit_start_worker "$formal_token_gate_state"
) || formal_token_gate_status=$?
[[ "$formal_token_gate_status" == 1 ]] \
  || fail "publisher token failure returned $formal_token_gate_status instead of 1"
[[ ! -e "$formal_token_gate_root_file" && ! -e "$formal_token_gate_child_file" ]] \
  || fail "subreaper forked the Bash publisher before parent capability admission"
formal_token_gate_root_pid="$(<"$formal_token_gate_state/.startup-admission-publisher.root")"
wait_for_test_process_exit "$formal_token_gate_root_pid" "token-gated publisher root"
assert_test_process_group_empty "$formal_token_gate_root_pid" "token-gated publisher"

# Once admission has occurred, abrupt parent death must still make the live
# subreaper reap its Bash publisher and nested descendant without shell help.
formal_parent_death_state="$temporary/formal-parent-death-publisher"
formal_parent_death_root_file="$temporary/formal-parent-death-root.pid"
formal_parent_death_child_file="$temporary/formal-parent-death-child.pid"
mkdir -p "$formal_parent_death_state"
(
  signal_status=0
  pending_signal=""
  cancel_started_realtime=""
  event_pending=0
  supervisor_handoff_to_outer=0
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER=1 \
    DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY=2 \
    DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$formal_stuck_entry" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_ROOT_FILE="$formal_parent_death_root_file" \
    DEVE_REMOTE_FIXTURE_ADMISSION_TEST_CHILD_FILE="$formal_parent_death_child_file" \
    remote_fixture_admit_start_worker "$formal_parent_death_state"
) &
formal_parent_death_supervisor_pid="$!"
register_test_process "$formal_parent_death_supervisor_pid"
wait_for_test_marker "$formal_parent_death_child_file" \
  "$formal_parent_death_supervisor_pid" "parent-death publisher admission"
formal_parent_death_root_pid="$(<"$formal_parent_death_root_file")"
formal_parent_death_child_pid="$(<"$formal_parent_death_child_file")"
kill -KILL "$formal_parent_death_supervisor_pid"
wait "$formal_parent_death_supervisor_pid" 2>/dev/null || true
wait_for_test_process_exit "$formal_parent_death_root_pid" "parent-death publisher root"
wait_for_test_process_exit "$formal_parent_death_child_pid" "parent-death publisher descendant"
assert_test_process_group_empty "$formal_parent_death_root_pid" "parent-death publisher"
test_progress formal-publisher-complete
test_worker_state_dir() {
  local -a arguments=("$@")
  local index
  for index in "${!arguments[@]}"; do
    if [[ "${arguments[$index]}" == --state-dir ]]; then
      mkdir -p "${arguments[$((index + 1))]:?}"
      printf '%s\n' "${arguments[$((index + 1))]:?}"
      return 0
    fi
  done
  return 1
}
test_wait_for_mock_admission() {
  local state_dir="$1"
  local readiness_notified=0
  for _ in $(seq 1 200); do
    [[ -f "$state_dir/.test-admitted" ]] && return 0
    if ((readiness_notified == 0)); then
      kill -USR1 "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}"
      readiness_notified=1
    fi
    sleep 0.05
  done
  return 1
}
test_wait_until_signal() {
  while :; do sleep 0.25; done
}

# Cancellation during the first parent-token probe must keep its typed signal
# status even though no worker or startup journal exists yet.
for early_signal_case in INT:130 TERM:143; do
  early_signal="${early_signal_case%%:*}"
  early_expected="${early_signal_case#*:}"
  early_status=0
  (
    early_supervisor_pid="$BASHPID"
    remote_fixture_process_token() {
      kill -s "$early_signal" "$early_supervisor_pid"
      return 1
    }
    start_fixture --state-dir "$temporary/early-parent-token-$early_signal" \
      --expected-head "$(printf 'e%.0s' {1..40})"
  ) || early_status=$?
  [[ "$early_status" == "$early_expected" ]] \
    || fail "early parent-token $early_signal returned $early_status instead of $early_expected"
done

# Cancellation at the wait-helper boundary must also be retained. Run this
# against pristine supervisor definitions before later failure-path test doubles.
wait_entry_state="$temporary/wait-entry-cancel"
wait_entry_hard_event="$temporary/wait-entry-hard-event"
wait_entry_status=0
(
  start_fixture_worker() {
    trap '' TERM
    local started_at="$SECONDS"
    while ((SECONDS - started_at < 20)); do :; done
    return 7
  }
  original_wait_definition="$(declare -f remote_fixture_wait_supervisor_event)"
  eval "${original_wait_definition/remote_fixture_wait_supervisor_event/original_remote_fixture_wait_supervisor_event}"
  wait_entry_injected=0
  remote_fixture_wait_supervisor_event() {
    if ((wait_entry_injected == 0)); then
      wait_entry_injected=1
      kill -TERM "$BASHPID"
    fi
    original_remote_fixture_wait_supervisor_event "$@"
  }
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_HARD_EVENT_MARKER="$wait_entry_hard_event" \
    DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=0.05 \
    DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=0.2 \
    start_fixture --state-dir "$wait_entry_state" \
      --expected-head "$(printf '4%.0s' {1..40})"
) || wait_entry_status=$?
[[ "$wait_entry_status" == 143 ]] \
  || fail "wait-entry cancellation returned $wait_entry_status instead of 143"
[[ -s "$wait_entry_hard_event" ]] \
  || fail "wait-entry cancellation lost its retained deadline event"
read -r wait_entry_latched_us wait_entry_hard_us <"$wait_entry_hard_event"
((wait_entry_hard_us - wait_entry_latched_us < 5000000)) \
  || fail "wait-entry cancellation observed its hard deadline too late"

# A worker failure after admission remains the supervisor result; signal-ready
# wakeups must not turn the retained child status into success.
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  test_wait_for_mock_admission "$state_dir"
  return 7
}
stop_fixture() { fail "failed worker unexpectedly published fixture state"; }
worker_failure_status=0
start_fixture --state-dir "$temporary/worker-failure" \
  --expected-head "$(printf '7%.0s' {1..40})" || worker_failure_status=$?
[[ "$worker_failure_status" == 7 ]] \
  || fail "admitted worker failure returned $worker_failure_status instead of 7"

# The supervisor waits only its exact worker/timer children. A caller's
# unrelated direct child must neither wake nor fail fixture startup.
sleep 0.2 &
ambient_child_pid="$!"
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  test_wait_for_mock_admission "$state_dir"
  local started_at="$SECONDS"
  while ((SECONDS - started_at < 2)); do :; done
}
start_fixture --state-dir "$temporary/ambient-child" \
  --expected-head "$(printf 'a%.0s' {1..40})" \
  || fail "ambient caller child corrupted exact supervisor waiting"
wait "$ambient_child_pid"

# A platform token probe can lag or fail. The unadmitted direct child must still
# reach the hard deadline, terminate as an exact waitable child, and be reaped.
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  printf '%s' "$BASHPID" >"$DEVE_REMOTE_FIXTURE_TEST_WORKER_PID"
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 143' TERM
  printf ready >"$DEVE_REMOTE_FIXTURE_TEST_READY"
  test_wait_for_mock_admission "$state_dir"
  test_wait_until_signal token-unavailable
}
stop_fixture() { fail "token-unavailable worker unexpectedly published fixture state"; }
token_state="$temporary/token-unavailable"
mkdir -p "$token_state"
DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
  DEVE_REMOTE_FIXTURE_TEST_FORCE_TOKEN_UNAVAILABLE=1 \
  DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=0.05 \
  DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=0.2 \
  DEVE_REMOTE_FIXTURE_TEST_READY="$token_state/ready" \
  DEVE_REMOTE_FIXTURE_TEST_WORKER_PID="$token_state/worker-pid" \
  DEVE_REMOTE_FIXTURE_TEST_CLEANED="$token_state/cleaned" \
  start_fixture --state-dir "$token_state" --expected-head "$(printf '9%.0s' {1..40})" &
token_supervisor_pid="$!"
register_test_process "$token_supervisor_pid"
wait_for_test_marker "$token_state/ready" "$token_supervisor_pid" "token-unavailable fixture"
kill -TERM "$token_supervisor_pid"
token_status=0
wait "$token_supervisor_pid" || token_status=$?
token_worker_pid="$(<"$token_state/worker-pid")"
[[ "$token_status" == 143 ]] \
  || fail "token-unavailable cancellation returned $token_status instead of 143"
remote_fixture_pid_active "$token_worker_pid" \
  && fail "token-unavailable cancellation returned with a live worker"

start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 143' TERM
  kill -TERM "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}"
  sleep 0.1
  test_wait_for_mock_admission "$state_dir"
  test_wait_until_signal pre-admission
}
stop_fixture() { fail "pre-admission signal unexpectedly published fixture state"; }
pending_status=0
DEVE_REMOTE_FIXTURE_TEST_CLEANED="$temporary/pending-cleaned" \
  start_fixture --state-dir "$temporary/pending" --expected-head "$(printf 'a%.0s' {1..40})" \
  || pending_status=$?
[[ "$pending_status" == 143 && -f "$temporary/pending-cleaned" ]] \
  || fail "pre-admission TERM was not forwarded and reaped"

# If readiness and cancellation are observed in the same supervisor cycle,
# cancellation must win: no admission marker may become visible to the worker.
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 143' TERM
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 130' INT
  printf ready >"$DEVE_REMOTE_FIXTURE_TEST_READY"
  test_wait_for_mock_admission "$state_dir"
  printf admitted >"$DEVE_REMOTE_FIXTURE_TEST_ADMITTED"
  test_wait_until_signal coalesced-cancel
}
stop_fixture() { fail "coalesced cancellation unexpectedly published fixture state"; }
coalesced_state="$temporary/coalesced-cancel"
mkdir -p "$coalesced_state"
coalesced_status=0
DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
  DEVE_REMOTE_FIXTURE_TEST_COALESCED_CANCEL=INT \
  DEVE_REMOTE_FIXTURE_TEST_READY="$coalesced_state/ready" \
  DEVE_REMOTE_FIXTURE_TEST_CLEANED="$coalesced_state/cleaned" \
  DEVE_REMOTE_FIXTURE_TEST_ADMITTED="$coalesced_state/admitted" \
  start_fixture --state-dir "$coalesced_state" --expected-head "$(printf 'b%.0s' {1..40})" \
  || coalesced_status=$?
[[ "$coalesced_status" == 130 && -f "$coalesced_state/cleaned" ]] \
  || fail "coalesced pre-admission INT was not forwarded and reaped"
[[ ! -e "$coalesced_state/admitted" && ! -e "$coalesced_state/.test-admitted" ]] \
  || fail "coalesced cancellation admitted the worker before rollback"

# The same private O_EXCL capability arbitrates a signal delivered inside the
# admission call. Cancellation winning that claim must prevent marker publish.
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 143' TERM
  printf ready >"$DEVE_REMOTE_FIXTURE_TEST_READY"
  test_wait_for_mock_admission "$state_dir"
  printf admitted >"$DEVE_REMOTE_FIXTURE_TEST_ADMITTED"
  test_wait_until_signal admission-race
}
stop_fixture() { fail "admission-race cancellation unexpectedly published fixture state"; }
admission_race_state="$temporary/admission-race"
mkdir -p "$admission_race_state"
admission_race_status=0
DEVE_REMOTE_FIXTURE_TEST_ADMIT_SIGNAL=TERM \
  DEVE_REMOTE_FIXTURE_TEST_READY="$admission_race_state/ready" \
  DEVE_REMOTE_FIXTURE_TEST_CLEANED="$admission_race_state/cleaned" \
  DEVE_REMOTE_FIXTURE_TEST_ADMITTED="$admission_race_state/admitted" \
  start_fixture --state-dir "$admission_race_state" \
    --expected-head "$(printf 'f%.0s' {1..40})" || admission_race_status=$?
[[ "$admission_race_status" == 143 && -f "$admission_race_state/cleaned" ]] \
  || fail "admission-race TERM was not propagated after atomic cancellation"
[[ ! -e "$admission_race_state/admitted" && ! -e "$admission_race_state/.test-admitted" ]] \
  || fail "cancellation inside admission published a worker-visible marker"

# Repeated readiness signals can interrupt wait while the token probe is still
# unavailable. Once admission succeeds and the worker exits 0, the supervisor
# must reap the retained child status instead of returning the last USR1 status.
capture_start_worker_token_definition="$(declare -f remote_fixture_capture_start_worker_token)"
capture_probe_counter="$temporary/readiness-retry-token-probes"
remote_fixture_capture_start_worker_token() {
  local pid="$1"
  local observed_count=0
  [[ ! -f "$capture_probe_counter" ]] || observed_count="$(wc -l <"$capture_probe_counter")"
  printf 'probe\n' >>"$capture_probe_counter"
  ((observed_count >= 4)) || return 1
  remote_fixture_process_token "$pid"
}
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  test_wait_for_mock_admission "$state_dir"
}
stop_fixture() { fail "readiness retry unexpectedly published fixture state"; }
readiness_retry_status=0
start_fixture --state-dir "$temporary/readiness-retry" \
  --expected-head "$(printf 'c%.0s' {1..40})" || readiness_retry_status=$?
eval "$capture_start_worker_token_definition"
[[ "$readiness_retry_status" == 0 ]] \
  || fail "readiness retry returned trap status $readiness_retry_status instead of worker status 0"

start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 143' TERM
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 130' INT
  test_wait_for_mock_admission "$state_dir"
  printf ready >"$DEVE_REMOTE_FIXTURE_TEST_READY"
  test_wait_until_signal "external-$iteration"
}
stop_fixture() { fail "externally cancelled worker unexpectedly published fixture state"; }
# Repeated external TERM is the process-level stress case. Actual async INT
# delivery is proven above through the formal exec adapter; background Bash
# test functions inherit SIGINT-ignore by shell design and are not valid INT
# targets.
test_progress external-cancel-matrix-start
for iteration in $(seq 1 25); do
  external_signal=TERM
  external_expected_status=143
  external_ready="$temporary/external-ready-$iteration"
  external_cleaned="$temporary/external-cleaned-$iteration"
  DEVE_REMOTE_FIXTURE_TEST_READY="$external_ready" \
    DEVE_REMOTE_FIXTURE_TEST_CLEANED="$external_cleaned" \
    start_fixture --state-dir "$temporary/external-$iteration" \
      --expected-head "$(printf 'd%.0s' {1..40})" &
  supervisor_pid="$!"
  register_test_process "$supervisor_pid"
  wait_for_test_marker "$external_ready" "$supervisor_pid" "external signal iteration $iteration"
  kill -s "$external_signal" "$supervisor_pid" \
    || fail "external $external_signal iteration $iteration supervisor exited early"
  external_status=0
  wait "$supervisor_pid" || external_status=$?
  [[ "$external_status" == "$external_expected_status" && -f "$external_cleaned" ]] \
    || fail "external $external_signal iteration $iteration did not cleanly reap its worker"
done
test_progress external-cancel-matrix-complete

# A worker stays live inside cleanup until released. The supervisor must not
# return cancellation before that exact child has exited and been waited.
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  printf '%s' "$BASHPID" >"$DEVE_REMOTE_FIXTURE_TEST_WORKER_PID"
  trap 'printf entered >"$DEVE_REMOTE_FIXTURE_TEST_CLEANUP_ENTERED"; while [[ ! -f "$DEVE_REMOTE_FIXTURE_TEST_RELEASE" ]]; do sleep 0.05; done; exit 143' TERM
  test_wait_for_mock_admission "$state_dir"
  printf ready >"$DEVE_REMOTE_FIXTURE_TEST_READY"
  test_wait_until_signal reap-order
}
reap_state="$temporary/reap"
mkdir -p "$reap_state"
DEVE_REMOTE_FIXTURE_TEST_READY="$reap_state/ready" \
  DEVE_REMOTE_FIXTURE_TEST_WORKER_PID="$reap_state/worker-pid" \
  DEVE_REMOTE_FIXTURE_TEST_CLEANUP_ENTERED="$reap_state/cleanup-entered" \
  DEVE_REMOTE_FIXTURE_TEST_RELEASE="$reap_state/release" \
  start_fixture --state-dir "$reap_state" --expected-head "$(printf 'e%.0s' {1..40})" &
reap_supervisor_pid="$!"
register_test_process "$reap_supervisor_pid"
wait_for_test_marker "$reap_state/ready" "$reap_supervisor_pid" "reap-order fixture"
kill -TERM "$reap_supervisor_pid"
wait_for_test_marker "$reap_state/cleanup-entered" "$reap_supervisor_pid" "reap-order cleanup"
remote_fixture_pid_active "$reap_supervisor_pid" \
  || fail "supervisor returned before blocked worker cleanup completed"
printf release >"$reap_state/release"
reap_status=0
wait "$reap_supervisor_pid" || reap_status=$?
reap_worker_pid="$(<"$reap_state/worker-pid")"
[[ "$reap_status" == 143 ]] || fail "reap-order cancellation returned $reap_status"
remote_fixture_pid_active "$reap_worker_pid" \
  && fail "supervisor returned before reaping the cancelled worker"

# INT is not a one-shot latch: a later TERM must deterministically upgrade the
# supervisor state before forwarding. Process-level INT delivery is covered by
# the formal adapter test above.
signal_status=0
pending_signal=""
cancel_started_realtime=""
event_pending=0
supervisor_handoff_to_outer=0
REMOTE_FIXTURE_START_ADMISSION_DECISION=""
remote_fixture_latch_start_cancel INT
[[ "$signal_status" == 130 && "$pending_signal" == INT && "$event_pending" == 1 ]] \
  || fail "INT cancellation was not latched as status 130"
remote_fixture_latch_start_cancel TERM
[[ "$signal_status" == 143 && "$pending_signal" == TERM && "$event_pending" == 1 ]] \
  || fail "later TERM did not upgrade the latched INT cancellation"
unset signal_status pending_signal cancel_started_realtime event_pending
unset supervisor_handoff_to_outer REMOTE_FIXTURE_START_ADMISSION_DECISION

# A stubborn worker reaches the absolute deadline. Its tree is terminated and
# explicitly reaped before journal-driven recovery runs.
start_fixture_worker() {
  local state_dir
  local test_idle_pid
  state_dir="$(test_worker_state_dir "$@")"
  trap ':' INT TERM
  mkdir -p "$DEVE_REMOTE_FIXTURE_TEST_STATE"
  printf startup >"$DEVE_REMOTE_FIXTURE_TEST_STATE/.test-startup"
  sleep 300 &
  test_idle_pid="$!"
  printf '%s' "$test_idle_pid" >"$DEVE_REMOTE_FIXTURE_TEST_STATE/child-pid"
  test_wait_for_mock_admission "$state_dir"
  printf ready >"$DEVE_REMOTE_FIXTURE_TEST_STATE/ready"
  while kill -0 "$test_idle_pid" 2>/dev/null; do
    wait "$test_idle_pid" 2>/dev/null || true
  done
}
deadline_state="$temporary/deadline"
DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
  DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=0.1 \
  DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=0.4 \
  DEVE_REMOTE_FIXTURE_TEST_STATE="$deadline_state" \
  start_fixture --state-dir "$deadline_state" --expected-head "$(printf '1%.0s' {1..40})" &
deadline_supervisor_pid="$!"
register_test_process "$deadline_supervisor_pid"
wait_for_test_marker "$deadline_state/ready" "$deadline_supervisor_pid" "deadline fixture"
kill -INT "$deadline_supervisor_pid"
deadline_status=0
wait "$deadline_supervisor_pid" || deadline_status=$?
deadline_child_pid="$(<"$deadline_state/child-pid")"
[[ "$deadline_status" == 130 && -f "$deadline_state/.test-recovered" ]] \
  || fail "deadline cancellation did not recover startup ownership"
remote_fixture_pid_active "$deadline_child_pid" \
  && fail "deadline termination left a worker descendant alive"

# A failed tree proof is sticky. If the first hard-deadline attempt kills the
# root but cannot prove its descendants gone, the supervisor must not retry on
# root absence and then authorize startup-state recovery.
terminate_waitable_definition="$(declare -f remote_fixture_terminate_waitable_start_worker)"
termination_attempt_counter="$temporary/tree-failure-attempts"
sticky_cancel_definition="$(declare -f remote_fixture_cancel_owned_state)"
sticky_cancel_counter="$temporary/tree-failure-cancel-calls"
remote_fixture_terminate_waitable_start_worker() {
  local pid="$1"
  printf 'attempt\n' >>"$termination_attempt_counter"
  kill -KILL "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  return 1
}
remote_fixture_cancel_owned_state() {
  printf 'called\n' >>"$sticky_cancel_counter"
  return 1
}
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  trap ':' INT TERM
  mkdir -p "$state_dir"
  printf startup >"$state_dir/.test-startup"
  test_wait_for_mock_admission "$state_dir"
  printf ready >"$state_dir/ready"
  test_wait_until_signal sticky-tree-failure
}
stop_fixture() { fail "failed tree proof unexpectedly authorized fixture stop"; }
sticky_state="$temporary/sticky-tree-failure"
sticky_status=0
(
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=0.05 \
    DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=0.1 \
    start_fixture --state-dir "$sticky_state" --expected-head "$(printf 'e%.0s' {1..40})" \
    || sticky_status=$?
  printf '%s' "$sticky_status" >"$sticky_state/returned-status"
  kill -TERM "$BASHPID"
  sleep 1
) &
sticky_supervisor_pid="$!"
register_test_process "$sticky_supervisor_pid"
wait_for_test_marker "$sticky_state/ready" "$sticky_supervisor_pid" "sticky tree failure fixture"
kill -TERM "$sticky_supervisor_pid"
wait "$sticky_supervisor_pid" || sticky_status=$?
eval "$terminate_waitable_definition"
eval "$sticky_cancel_definition"
[[ "$sticky_status" == 143 ]] || fail "sticky tree failure returned $sticky_status instead of 143"
[[ "$(<"$sticky_state/returned-status")" == 143 ]] \
  || fail "sticky tree failure did not return before the follow-up signal"
[[ "$(wc -l <"$termination_attempt_counter")" == 1 ]] \
  || fail "failed tree cleanup was retried after root disappearance"
[[ -f "$sticky_state/.test-startup" && ! -e "$sticky_state/.test-recovered" ]] \
  || fail "failed tree proof did not preserve startup ownership state"
[[ ! -e "$sticky_cancel_counter" ]] \
  || fail "follow-up signal consumed startup state after an unverified tree failure"

# Admission-state rejection is not worker identity loss: the exact worker must
# be terminated and reaped before USR1 is restored, while invalid state remains
# preserved because journal recovery itself fails closed.
admit_worker_definition="$(declare -f remote_fixture_admit_start_worker)"
cancel_owned_state_definition="$(declare -f remote_fixture_cancel_owned_state)"
remote_fixture_admit_start_worker() { return 1; }
remote_fixture_cancel_owned_state() { return 1; }
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  trap 'exit 143' TERM
  trap 'exit 130' INT
  mkdir -p "$state_dir"
  printf invalid >"$state_dir/.test-startup"
  printf '%s' "$BASHPID" >"$state_dir/worker-pid"
  while :; do
    kill -USR1 "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}" 2>/dev/null || exit 0
    sleep 0.02
  done
}
stop_fixture() { fail "invalid admission state unexpectedly authorized fixture stop"; }
admission_failure_state="$temporary/admission-failure"
admission_failure_status=0
start_fixture --state-dir "$admission_failure_state" \
  --expected-head "$(printf '0%.0s' {1..40})" || admission_failure_status=$?
admission_failure_worker_pid="$(<"$admission_failure_state/worker-pid")"
eval "$admit_worker_definition"
eval "$cancel_owned_state_definition"
[[ "$admission_failure_status" == 1 ]] \
  || fail "admission-state rejection returned $admission_failure_status instead of 1"
remote_fixture_pid_active "$admission_failure_worker_pid" \
  && fail "admission-state rejection returned with a live readiness worker"
[[ -f "$admission_failure_state/.test-startup" ]] \
  || fail "admission-state rejection did not preserve invalid startup state"
printf parent-survived >"$admission_failure_state/parent-survived"

# The outer trap remains rollback-capable after successful publication returns.
start_fixture_worker() {
  local state_dir
  state_dir="$(test_worker_state_dir "$@")"
  mkdir -p "$DEVE_REMOTE_FIXTURE_TEST_STATE"
  test_wait_for_mock_admission "$state_dir"
  printf published >"$DEVE_REMOTE_FIXTURE_TEST_STATE/.test-published"
}
stop_fixture() {
  local state_dir="${2:?}"
  printf stopped >"$state_dir/.test-stopped"
  rm -f -- "$state_dir/.test-published"
}
handoff_state="$temporary/handoff"
handoff_status=0
(
  DEVE_REMOTE_FIXTURE_TEST_STATE="$handoff_state" \
    start_fixture --state-dir "$handoff_state" --expected-head "$(printf '2%.0s' {1..40})"
  kill -TERM "$BASHPID"
  sleep 1
) || handoff_status=$?
[[ "$handoff_status" == 143 && -f "$handoff_state/.test-stopped" ]] \
  || fail "post-publication outer trap did not retain rollback ownership"

# The supervisor-to-outer trap transition is also rollback-capable before both
# outer signal dispositions have been installed.
outer_transition_state="$temporary/outer-transition"
outer_transition_status=0
(
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_OUTER_HANDOFF_SIGNAL=TERM \
    DEVE_REMOTE_FIXTURE_TEST_STATE="$outer_transition_state" \
    start_fixture --state-dir "$outer_transition_state" \
      --expected-head "$(printf 'd%.0s' {1..40})"
) || outer_transition_status=$?
[[ "$outer_transition_status" == 143 && -f "$outer_transition_state/.test-stopped" ]] \
  || fail "supervisor-to-outer trap transition lost cancellation rollback"

# A signal just before the transition flag is set is still latched by the old
# supervisor handler; the final handoff must re-sample and roll it back once.
pre_outer_transition_state="$temporary/pre-outer-transition"
pre_outer_transition_jobs="$temporary/pre-outer-transition.jobs"
pre_outer_transition_status=0
(
  set +e
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_PRE_OUTER_HANDOFF_SIGNAL=TERM \
    DEVE_REMOTE_FIXTURE_TEST_STATE="$pre_outer_transition_state" \
    start_fixture --state-dir "$pre_outer_transition_state" \
      --expected-head "$(printf 'f%.0s' {1..40})"
  transition_status=$?
  set -e
  jobs -pr >"$pre_outer_transition_jobs"
  exit "$transition_status"
) || pre_outer_transition_status=$?
[[ "$pre_outer_transition_status" == 143 \
  && -f "$pre_outer_transition_state/.test-stopped" ]] \
  || fail "pre-outer handoff latch did not perform cancellation rollback"
[[ ! -s "$pre_outer_transition_jobs" ]] \
  || fail "pre-outer handoff left retained supervisor event children"

# A spawn rejection must not leak supervisor traps that close over locals.
# The restored outer handler remains safe if the caller receives TERM later.
spawn_reject_state="$temporary/spawn-reject"
spawn_reject_status=0
(
  unset DEVE_REMOTE_FIXTURE_TEST_MODE DEVE_REMOTE_FIXTURE_TEST_ALLOW_UNGROUPED
  unset DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT
  start_fixture --state-dir "$spawn_reject_state" \
    --expected-head "$(printf '9%.0s' {1..40})" || true
  kill -TERM "$BASHPID"
) || spawn_reject_status=$?
[[ "$spawn_reject_status" == 143 ]] \
  || fail "spawn rejection leaked local supervisor traps: status $spawn_reject_status"

# Cancellation before the first wait must start its deadline immediately even
# when the worker never sends readiness and ignores the forwarded TERM.
pre_wait_state="$temporary/pre-wait-cancel"
pre_wait_start_ms="$(node -e 'process.stdout.write(String(Date.now()))')"
pre_wait_status=0
(
  start_fixture_worker() {
    trap '' TERM
    kill -TERM "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}"
    while :; do :; done
  }
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=0.05 \
    DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=0.2 \
    start_fixture --state-dir "$pre_wait_state" \
      --expected-head "$(printf '7%.0s' {1..40})"
) || pre_wait_status=$?
pre_wait_elapsed_ms=$(( $(node -e 'process.stdout.write(String(Date.now()))') - pre_wait_start_ms ))
[[ "$pre_wait_status" == 143 ]] \
  || fail "pre-wait cancellation returned $pre_wait_status instead of 143"
((pre_wait_elapsed_ms < 4500)) \
  || fail "pre-wait cancellation exceeded its hard deadline: ${pre_wait_elapsed_ms}ms"

# A cancellation latched before the first wait is an immediate worker event;
# it must not wait for the TERM-upgrade timer before forwarding the same TERM.
pre_wait_forward_state="$temporary/pre-wait-forward"
pre_wait_forward_sent="$temporary/pre-wait-forward.sent"
pre_wait_forward_delivered="$temporary/pre-wait-forward.delivered"
pre_wait_forward_start_ms="$(node -e 'process.stdout.write(String(Date.now()))')"
pre_wait_forward_status=0
(
  start_fixture_worker() {
    trap 'printf delivered >"$DEVE_REMOTE_FIXTURE_TEST_DELIVERED"; exit 143' TERM
    kill -TERM "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}"
    printf sent >"$DEVE_REMOTE_FIXTURE_TEST_SENT"
    while :; do :; done
  }
  remote_fixture_spawn_start_worker() {
    local parent_pid="$1"
    local parent_token="$2"
    shift 2
    REMOTE_FIXTURE_START_WORKER_PROCESS_GROUP=0
    DEVE_REMOTE_FIXTURE_START_PARENT_PID="$parent_pid" \
      DEVE_REMOTE_FIXTURE_START_PARENT_TOKEN="$parent_token" \
      start_fixture_worker "$@" &
    REMOTE_FIXTURE_START_WORKER_PID="$!"
    wait_for_test_marker "$DEVE_REMOTE_FIXTURE_TEST_SENT" \
      "$REMOTE_FIXTURE_START_WORKER_PID" "pre-wait forward barrier"
  }
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=5 \
    DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=7 \
    DEVE_REMOTE_FIXTURE_TEST_SENT="$pre_wait_forward_sent" \
    DEVE_REMOTE_FIXTURE_TEST_DELIVERED="$pre_wait_forward_delivered" \
    start_fixture --state-dir "$pre_wait_forward_state" \
      --expected-head "$(printf '5%.0s' {1..40})"
) || pre_wait_forward_status=$?
pre_wait_forward_elapsed_ms=$(( $(node -e 'process.stdout.write(String(Date.now()))') - pre_wait_forward_start_ms ))
[[ "$pre_wait_forward_status" == 143 && -f "$pre_wait_forward_delivered" ]] \
  || fail "pre-wait cancellation was not delivered to the exact worker immediately"
((pre_wait_forward_elapsed_ms < 3500)) \
  || fail "pre-wait cancellation waited for its grace timer: ${pre_wait_forward_elapsed_ms}ms"

# If cancellation lands immediately before wait -n, the latch must bypass the
# wait and reach the worker before the grace timer does.
wait_check_state="$temporary/wait-check-cancel"
wait_check_delivered="$temporary/wait-check-cancel.delivered"
wait_check_start_ms="$(node -e 'process.stdout.write(String(Date.now()))')"
wait_check_status=0
(
  start_fixture_worker() {
    local state_dir
    state_dir="$(test_worker_state_dir "$@")"
    trap 'printf delivered >"$DEVE_REMOTE_FIXTURE_TEST_DELIVERED"; exit 143' TERM
    test_wait_for_mock_admission "$state_dir"
    while :; do :; done
  }
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=5 \
    DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=7 \
    DEVE_REMOTE_FIXTURE_TEST_WAIT_CHECK_SIGNAL=TERM \
    DEVE_REMOTE_FIXTURE_TEST_DELIVERED="$wait_check_delivered" \
    start_fixture --state-dir "$wait_check_state" \
      --expected-head "$(printf '3%.0s' {1..40})"
) || wait_check_status=$?
wait_check_elapsed_ms=$(( $(node -e 'process.stdout.write(String(Date.now()))') - wait_check_start_ms ))
[[ "$wait_check_status" == 143 && -f "$wait_check_delivered" ]] \
  || fail "wait-check cancellation was not delivered to the exact worker"
((wait_check_elapsed_ms < 3500)) \
  || fail "wait-check cancellation waited for its grace timer: ${wait_check_elapsed_ms}ms"

# A one-shot deadline wake can land after the hard check and before the next
# wait. The repeated deadline wake must make that check-to-wait race bounded.
post_hard_state="$temporary/post-hard-check-cancel"
post_hard_start_ms="$(node -e 'process.stdout.write(String(Date.now()))')"
post_hard_status=0
(
  start_fixture_worker() {
    local state_dir
    state_dir="$(test_worker_state_dir "$@")"
    test_wait_for_mock_admission "$state_dir"
    while :; do :; done
  }
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
    DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY=0.02 \
    DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY=0.05 \
    DEVE_REMOTE_FIXTURE_TEST_POST_HARD_CHECK_CANCEL=TERM \
    start_fixture --state-dir "$post_hard_state" \
      --expected-head "$(printf '6%.0s' {1..40})"
) || post_hard_status=$?
post_hard_elapsed_ms=$(( $(node -e 'process.stdout.write(String(Date.now()))') - post_hard_start_ms ))
[[ "$post_hard_status" == 143 ]] \
  || fail "post-hard-check cancellation returned $post_hard_status instead of 143"
((post_hard_elapsed_ms < 4500)) \
  || fail "post-hard-check cancellation lost its repeated wake: ${post_hard_elapsed_ms}ms"

# On a formal Linux process group, cooperative root exit with a surviving
# descendant is not enough to authorize journal recovery. A user-space group
# capability cannot safely signal the retained numeric PGID after leader exit.
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *)
    if command -v setsid >/dev/null 2>&1; then
      cooperative_entry="$temporary/cooperative-group-worker.sh"
      cooperative_child_file="$temporary/cooperative-group-child.pid"
      cat >"$cooperative_entry" <<'SH'
#!/usr/bin/env bash
bash -c 'trap "" TERM; while :; do sleep 60; done; :' &
printf '%s' "$!" >"${DEVE_REMOTE_FIXTURE_TEST_CHILD_FILE:?}"
sleep 1
exit 7
SH
      chmod +x "$cooperative_entry"
      cooperative_status=0
      (
        unset DEVE_REMOTE_FIXTURE_TEST_MODE DEVE_REMOTE_FIXTURE_TEST_ALLOW_UNGROUPED
        export DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$cooperative_entry"
        export DEVE_REMOTE_FIXTURE_TEST_CHILD_FILE="$cooperative_child_file"
        start_fixture --state-dir "$temporary/cooperative-group-state" \
          --expected-head "$(printf '8%.0s' {1..40})"
      ) || cooperative_status=$?
      [[ "$cooperative_status" != 0 ]] \
        || fail "cooperative root exit with a retained group was accepted"
      [[ -s "$cooperative_child_file" ]] \
        || fail "cooperative group worker did not publish its descendant"
      cooperative_child_pid="$(<"$cooperative_child_file")"
      remote_fixture_pid_active "$cooperative_child_pid" \
        || fail "cooperative retained group was signalled after leader exit"
      cooperative_child_token="$(remote_fixture_wait_stable_process_token \
        "cooperative retained-group child" "$cooperative_child_pid")"
      remote_fixture_stop_bounded_tree "cooperative retained-group test child" \
        "$cooperative_child_pid" 0 "$cooperative_child_token"
      remote_fixture_pid_active "$cooperative_child_pid" \
        && fail "cooperative retained-group test cleanup failed"
    fi
    ;;
esac

# A requested production group that fails isolation proof cannot be downgraded
# to an ungrouped worker and then reported tree-empty after its root exits.
unverified_group_child_file="$temporary/unverified-group-child.pid"
unverified_group_status=0
(
  start_fixture_worker() {
    bash -c 'trap "" TERM; while :; do sleep 60; done; :' &
    printf '%s' "$!" >"$DEVE_REMOTE_FIXTURE_TEST_CHILD_FILE"
    return 7
  }
  remote_fixture_spawn_start_worker() {
    local parent_pid="$1"
    local parent_token="$2"
    shift 2
    DEVE_REMOTE_FIXTURE_START_PARENT_PID="$parent_pid" \
      DEVE_REMOTE_FIXTURE_START_PARENT_TOKEN="$parent_token" \
      start_fixture_worker "$@" &
    REMOTE_FIXTURE_START_WORKER_PID="$!"
    REMOTE_FIXTURE_START_WORKER_PROCESS_GROUP=1
  }
  remote_fixture_wait_isolated_process_group() { return 1; }
  DEVE_REMOTE_FIXTURE_TEST_CHILD_FILE="$unverified_group_child_file" \
    start_fixture --state-dir "$temporary/unverified-group-state" \
      --expected-head "$(printf '2%.0s' {1..40})"
) || unverified_group_status=$?
[[ "$unverified_group_status" != 0 && -s "$unverified_group_child_file" ]] \
  || fail "failed group isolation was downgraded to successful ungrouped cleanup"
unverified_group_child_pid="$(<"$unverified_group_child_file")"
remote_fixture_pid_active "$unverified_group_child_pid" \
  || fail "unverified process group was signalled after isolation proof failed"
unverified_group_child_token="$(remote_fixture_wait_stable_process_token \
  "unverified group test child" "$unverified_group_child_pid")"
remote_fixture_stop_bounded_tree "unverified group test child" \
  "$unverified_group_child_pid" 0 "$unverified_group_child_token"

# A group that passed initial isolation but failed capability binding remains
# unverified. A TERM-resistant tree cannot be reclassified as ungrouped.
bind_failure_root_file="$temporary/bind-failure-root.pid"
bind_failure_child_file="$temporary/bind-failure-child.pid"
bind_failure_status=0
(
  start_fixture_worker() {
    trap '' TERM
    printf '%s' "$BASHPID" >"$DEVE_REMOTE_FIXTURE_TEST_ROOT_FILE"
    bash -c 'trap "" TERM; while :; do sleep 60; done; :' &
    printf '%s' "$!" >"$DEVE_REMOTE_FIXTURE_TEST_CHILD_FILE"
    while :; do sleep 60; done
  }
  remote_fixture_spawn_start_worker() {
    local parent_pid="$1"
    local parent_token="$2"
    shift 2
    DEVE_REMOTE_FIXTURE_START_PARENT_PID="$parent_pid" \
      DEVE_REMOTE_FIXTURE_START_PARENT_TOKEN="$parent_token" \
      start_fixture_worker "$@" &
    REMOTE_FIXTURE_START_WORKER_PID="$!"
    REMOTE_FIXTURE_START_WORKER_PROCESS_GROUP=1
  }
  remote_fixture_wait_isolated_process_group() { return 0; }
  remote_fixture_bind_isolated_process_group() { return 1; }
  DEVE_REMOTE_FIXTURE_TEST_ROOT_FILE="$bind_failure_root_file" \
    DEVE_REMOTE_FIXTURE_TEST_CHILD_FILE="$bind_failure_child_file" \
    start_fixture --state-dir "$temporary/bind-failure-state" \
      --expected-head "$(printf 'b%.0s' {1..40})"
) || bind_failure_status=$?
[[ "$bind_failure_status" != 0 && -s "$bind_failure_root_file" \
  && -s "$bind_failure_child_file" ]] \
  || fail "failed group binding was downgraded to successful ungrouped cleanup"
bind_failure_root_pid="$(<"$bind_failure_root_file")"
bind_failure_child_pid="$(<"$bind_failure_child_file")"
remote_fixture_pid_active "$bind_failure_root_pid" \
  || fail "unbound group root was reclassified and reaped"
remote_fixture_pid_active "$bind_failure_child_pid" \
  || fail "unbound group descendant was reclassified and reaped"
bind_failure_child_token="$(remote_fixture_wait_stable_process_token \
  "bind-failure child" "$bind_failure_child_pid")"
remote_fixture_stop_bounded_tree "bind-failure child" \
  "$bind_failure_child_pid" 0 "$bind_failure_child_token"
bind_failure_root_token="$(remote_fixture_wait_stable_process_token \
  "bind-failure root" "$bind_failure_root_pid")"
remote_fixture_stop_bounded_tree "bind-failure root" \
  "$bind_failure_root_pid" 0 "$bind_failure_root_token"

# A token mismatch must never degrade to signalling the naked numeric PID.
sleep 300 &
unowned_pid="$!"
register_test_process "$unowned_pid"
unowned_token="$(remote_fixture_process_token "$unowned_pid")"
if remote_fixture_signal_start_worker "$unowned_pid" "mismatch-$unowned_token" TERM \
  2>"$temporary/token-mismatch.stderr"; then
  fail "worker token mismatch was accepted"
fi
remote_fixture_pid_active "$unowned_pid" || fail "token mismatch killed an unowned process"
kill -TERM "$unowned_pid" 2>/dev/null || true
wait "$unowned_pid" 2>/dev/null || true

# Exercise the real run_fixture handoff. TERM arrives after start_fixture has
# returned but while run_fixture is canonicalizing the state directory; the
# outer lifecycle trap must still perform the published-state rollback.
run_fixture_source="$(sed -n '/^run_fixture() {/,/^case "${1:-}" in/p' \
  "$ROOT_DIR/scripts/remote-browser-fixture.sh" | sed '$d')"
eval "$run_fixture_source"
start_fixture() {
  local state_dir=""
  while (($#)); do
    if [[ "$1" == --state-dir ]]; then state_dir="${2:?}"; shift 2; else shift; fi
  done
  mkdir -p "$state_dir"
  printf published >"$state_dir/.test-published"
  remote_fixture_arm_outer_start_lifecycle "$state_dir"
  printf ready >"$state_dir/run-start-returned"
}
stop_fixture() {
  local state_dir="${2:?}"
  printf stopped >"$state_dir/.test-stopped"
  rm -f -- "$state_dir/.test-published"
}
remote_fixture_canonical_dir() {
  sleep 5
  printf '%s\n' "$1"
}
run_handoff_state="$temporary/run-handoff"
run_handoff_status=0
run_fixture --state-dir "$run_handoff_state" --expected-head "$(printf '3%.0s' {1..40})" \
  -- bash -c 'exit 0' &
run_handoff_pid="$!"
register_test_process "$run_handoff_pid"
wait_for_test_marker "$run_handoff_state/run-start-returned" "$run_handoff_pid" \
  "run_fixture start-to-run handoff"
kill -TERM "$run_handoff_pid"
wait "$run_handoff_pid" || run_handoff_status=$?
[[ "$run_handoff_status" == 143 && -f "$run_handoff_state/.test-stopped" ]] \
  || fail "run_fixture start-to-run handoff lost continuous rollback ownership"

printf 'remote-browser-fixture-start-supervisor.test: ok\n'
