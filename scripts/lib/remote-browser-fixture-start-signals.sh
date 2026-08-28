#!/usr/bin/env bash
# shellcheck shell=bash

# Signal latching and retained child events for the Unix startup supervisor.
# A completed timer remains waitable, so deadline events cannot be lost between
# a state check and the next wait. A bounded observation tick prevents Bash's
# deferred trap execution from becoming an unbounded lost wake.

remote_fixture_require_wait_any_child() {
  if ((BASH_VERSINFO[0] > 5 \
    || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1))); then
    return 0
  fi
  remote_fixture_fail "Unix fixture startup requires Bash 5.1+ wait -n -p support"
}

remote_fixture_supervisor_delay() {
  local name="$1"
  local default_value="$2"
  local value="$default_value"
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 ]]; then
    case "$name" in
      term) value="${DEVE_REMOTE_FIXTURE_TEST_TERM_DELAY:-$default_value}" ;;
      hard) value="${DEVE_REMOTE_FIXTURE_TEST_HARD_DELAY:-$default_value}" ;;
    esac
  fi
  [[ "$value" =~ ^[0-9]+([.][0-9]+)?$ && "$value" != 0 && "$value" != 0.0 ]] || {
    remote_fixture_fail "fixture supervisor $name delay must be a positive number"
    return 1
  }
  printf '%s\n' "$value"
}

remote_fixture_supervisor_observation_delay() {
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 ]]; then
    printf '0.02\n'
  else
    printf '1\n'
  fi
}

remote_fixture_stop_supervisor_timer() {
  local pid="$1"
  [[ "$pid" =~ ^[0-9]+$ ]] || return 0
  kill -TERM "$pid" 2>/dev/null || true
  while remote_fixture_job_active "$pid"; do
    wait "$pid" 2>/dev/null || true
    remote_fixture_job_active "$pid" || break
    kill -TERM "$pid" 2>/dev/null || true
  done
  wait "$pid" 2>/dev/null || true
}

remote_fixture_start_supervisor_timer() {
  local delay="$1"
  sleep "$delay" &
  REMOTE_FIXTURE_SUPERVISOR_TIMER_PID="$!"
}

remote_fixture_reap_observation_tick() {
  local pid="$1"
  local completed_pid="" retained_status=0
  [[ "$pid" =~ ^[0-9]+$ ]] || return 0
  while :; do
    completed_pid=""
    retained_status=0
    wait -n -p completed_pid "$pid" 2>/dev/null || retained_status=$?
    if [[ "${completed_pid:-}" == "$pid" ]] || ((retained_status == 127)); then
      return 0
    fi
    ((retained_status > 128)) || {
      remote_fixture_fail "fixture observation tick returned without an exact child event"
      return 1
    }
    # A trapped parent signal can interrupt wait without consuming the exact
    # child. Retry wait -n against only that retained child; unlike repeated
    # wait PID, a consumed >128 child is identified by -p exactly once.
  done
}

REMOTE_FIXTURE_ADMISSION_DEADLINE_SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-admission-deadline.py"
REMOTE_FIXTURE_START_ADMISSION_TIMER_TOKEN=""

remote_fixture_start_admission_publisher_timer() {
  local delay="$1"
  local decision_path="$2"
  local deadline_path="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_DEADLINE_PATH"
  [[ -f "$REMOTE_FIXTURE_ADMISSION_DEADLINE_SCRIPT" \
    && ! -L "$REMOTE_FIXTURE_ADMISSION_DEADLINE_SCRIPT" ]] || {
    remote_fixture_fail "fixture admission deadline helper is missing or unsafe"
    return 1
  }
  remote_fixture_require_command python3
  [[ -n "$deadline_path" ]] || {
    remote_fixture_fail "fixture admission deadline marker path is unavailable"
    return 1
  }
  local expected_parent_pid="$BASHPID"
  python3 "$REMOTE_FIXTURE_ADMISSION_DEADLINE_SCRIPT" \
    "$delay" "$decision_path" "$deadline_path" "$expected_parent_pid" &
  REMOTE_FIXTURE_SUPERVISOR_TIMER_PID="$!"
  REMOTE_FIXTURE_START_ADMISSION_TIMER_TOKEN=""
  local attempt
  for ((attempt = 0; attempt < 100; attempt++)); do
    REMOTE_FIXTURE_START_ADMISSION_TIMER_TOKEN="$(
      remote_fixture_process_token "$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID" 2>/dev/null || true
    )"
    [[ -n "$REMOTE_FIXTURE_START_ADMISSION_TIMER_TOKEN" ]] && return 0
    remote_fixture_pid_active "$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID" || break
    sleep 0.01
  done
  remote_fixture_fail "fixture admission deadline child token is unavailable"
  local identity_status=0
  remote_fixture_root_identity_status "$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID" "" \
    || identity_status=$?
  if ((identity_status == 1)); then
    remote_fixture_reap_observation_tick "$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID" || true
    REMOTE_FIXTURE_SUPERVISOR_TIMER_PID=""
  fi
  return 1
}

remote_fixture_stop_admission_publisher_timer() {
  local pid="$1"
  local token="$2"
  [[ "$pid" =~ ^[0-9]+$ ]] || return 0
  local identity_status=0
  remote_fixture_root_identity_status "$pid" "$token" || identity_status=$?
  if ((identity_status == 0)); then
    remote_fixture_signal_owned_identity \
      "fixture admission deadline child" "$pid" "$token" TERM || return 1
  elif ((identity_status == 2)); then
    remote_fixture_fail \
      "refusing to wait a live admission deadline child without token proof"
    return 1
  fi
  # A gone child is safe to exact-reap. A live but unreadable/reused numeric
  # identity returned above without waiting or signalling.
  remote_fixture_reap_observation_tick "$pid"
}

remote_fixture_retire_consumed_admission_timer_identity() {
  local pid_name="$1"
  local token_name="$2"
  [[ "$pid_name" =~ ^[a-z_][a-z0-9_]*$ \
    && "$token_name" =~ ^[a-z_][a-z0-9_]*$ ]] || {
    remote_fixture_fail "invalid admission deadline ownership variable"
    return 1
  }
  # The wait event consumed this exact child. Clear both parts of the
  # capability before any trap or cleanup path can call a signalling helper.
  printf -v "$pid_name" '%s' ""
  printf -v "$token_name" '%s' ""
}

remote_fixture_admission_deadline_status() {
  local deadline_path="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_DEADLINE_PATH"
  [[ -e "$deadline_path" || -L "$deadline_path" ]] || return 1
  [[ -f "$deadline_path" && ! -L "$deadline_path" && ! -s "$deadline_path" ]] || {
    remote_fixture_fail "fixture admission deadline marker is unsafe"
    return 2
  }
  return 0
}

remote_fixture_epoch_microseconds() {
  local target_name="$1"
  remote_fixture_realtime_microseconds "$EPOCHREALTIME" "$target_name"
}

remote_fixture_realtime_microseconds() {
  local timestamp="$1"
  local target_name="$2"
  local seconds="${timestamp%%.*}"
  local fraction="${timestamp#*.}000000"
  fraction="${fraction:0:6}"
  printf -v "$target_name" '%d' "$((10#$seconds * 1000000 + 10#$fraction))"
}

remote_fixture_delay_microseconds() {
  local value="$1"
  local target_name="$2"
  local seconds="${value%%.*}"
  local fraction=0
  if [[ "$value" == *.* ]]; then fraction="${value#*.}"; fi
  fraction="${fraction}000000"
  fraction="${fraction:0:6}"
  printf -v "$target_name" '%d' "$((10#$seconds * 1000000 + 10#$fraction))"
}

remote_fixture_start_remaining_supervisor_timer() {
  local total_delay="$1"
  local started_at_us="$2"
  local now_us total_us remaining_us
  remote_fixture_epoch_microseconds now_us
  remote_fixture_delay_microseconds "$total_delay" total_us
  remaining_us=$((total_us - (now_us - started_at_us)))
  ((remaining_us > 0)) || remaining_us=0
  local remaining_delay
  printf -v remaining_delay '%d.%06d' \
    "$((remaining_us / 1000000))" "$((remaining_us % 1000000))"
  remote_fixture_start_supervisor_timer "$remaining_delay"
}

remote_fixture_start_latched_cancel_timers() {
  [[ -n "$cancel_started_realtime" ]] || {
    remote_fixture_fail "fixture cancellation deadline has no start timestamp"
    return 1
  }
  remote_fixture_realtime_microseconds "$cancel_started_realtime" cancel_started_us
  remote_fixture_start_remaining_supervisor_timer "$term_delay" "$cancel_started_us"
  grace_timer_pid="$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID"
  remote_fixture_start_remaining_supervisor_timer "$hard_delay" "$cancel_started_us"
  hard_timer_pid="$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID"
  timers_started=1
}

REMOTE_FIXTURE_SUPERVISOR_WAIT_PID=""
REMOTE_FIXTURE_SUPERVISOR_WAIT_STATUS=0
REMOTE_FIXTURE_START_ADMISSION_DECISION=""
REMOTE_FIXTURE_START_ADMISSION_OWNER_PID=""

remote_fixture_wait_supervisor_event() {
  local worker_pid="$1"
  local grace_timer_pid="${2:-}"
  local hard_timer_pid="${3:-}"
  local completed_pid="" wait_status=0 observation_delay observation_pid
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
    && "$worker_ready" == 1 \
    && "$test_wait_check_injected" == 0 \
    && -n "${DEVE_REMOTE_FIXTURE_TEST_WAIT_CHECK_SIGNAL:-}" ]]; then
    case "$DEVE_REMOTE_FIXTURE_TEST_WAIT_CHECK_SIGNAL" in
      INT|TERM) kill -s "$DEVE_REMOTE_FIXTURE_TEST_WAIT_CHECK_SIGNAL" "$BASHPID" ;;
      *) remote_fixture_fail "invalid wait-check test signal"; return 1 ;;
    esac
    test_wait_check_injected=1
  fi
  observation_delay="$(remote_fixture_supervisor_observation_delay)" || return 1
  remote_fixture_start_supervisor_timer "$observation_delay"
  observation_pid="$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID"
  local -a waitable_pids=("$worker_pid" "$observation_pid")
  [[ -z "$grace_timer_pid" ]] || waitable_pids+=("$grace_timer_pid")
  [[ -z "$hard_timer_pid" ]] || waitable_pids+=("$hard_timer_pid")
  wait -n -p completed_pid "${waitable_pids[@]}" || wait_status=$?
  # Bash may populate -p with a child even when wait itself returned only
  # because a parent signal trap ran. A >128 wait is never a completion proof;
  # the supervisor rechecks/reaps the exact child in normal context.
  if ((wait_status > 128)); then
    completed_pid=""
  fi
  if [[ "${completed_pid:-}" == "$observation_pid" ]]; then
    completed_pid=""
    wait_status=129
  fi
  # The tick is deliberately allowed to settle (at most one formal second).
  # Never signal its numeric PID after wait -n may already have reaped it.
  remote_fixture_reap_observation_tick "$observation_pid" || return 1
  REMOTE_FIXTURE_SUPERVISOR_WAIT_PID="${completed_pid:-}"
  REMOTE_FIXTURE_SUPERVISOR_WAIT_STATUS="$wait_status"
}

REMOTE_FIXTURE_REAPED_WORKER_STATUS=0
remote_fixture_reap_start_worker_status() {
  local pid="$1"
  local interrupted_status="$2"
  local retained_status="$interrupted_status"
  if remote_fixture_pid_exists "$pid" && ! remote_fixture_pid_terminal "$pid"; then
    remote_fixture_fail "refusing to exact-wait a live fixture start worker"
    return 1
  fi
  # A signal-interrupted wait does not consume the exact child. Re-wait it for
  # the retained status; 127 means an earlier wait already consumed the child.
  if ((interrupted_status > 128)); then
    if wait "$pid"; then retained_status=0; else retained_status=$?; fi
    if ((retained_status == 127)); then retained_status="$interrupted_status"; fi
  fi
  REMOTE_FIXTURE_REAPED_WORKER_STATUS="$retained_status"
}

remote_fixture_latch_start_cancel() {
  local signal_name="$1"
  local requested_status
  case "$signal_name" in
    INT) requested_status=130 ;;
    TERM) requested_status=143 ;;
    *) remote_fixture_fail "unsupported fixture supervisor signal $signal_name"; return 1 ;;
  esac
  if [[ "$supervisor_handoff_to_outer" == 1 ]]; then
    remote_fixture_outer_cancel "$requested_status"
  fi
  if [[ "$signal_name" == TERM ]]; then
    pending_signal=TERM
    signal_status=143
  else
    [[ "$pending_signal" == TERM ]] || pending_signal=INT
    [[ "$signal_status" == 0 ]] && signal_status=130
  fi
  [[ -n "$cancel_started_realtime" ]] || cancel_started_realtime="$EPOCHREALTIME"
  event_pending=1
  local decision_path="${REMOTE_FIXTURE_START_ADMISSION_DECISION:-}"
  if [[ -n "$decision_path" ]]; then
    # This O_EXCL claim is the sole admission/cancel linearization point. The
    # subshell uses Bash builtins only and never exposes secret material.
    (umask 077; set -o noclobber; : >"$decision_path") 2>/dev/null || true
  fi
}

remote_fixture_record_test_hard_event() {
  [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
    && -n "${DEVE_REMOTE_FIXTURE_TEST_HARD_EVENT_MARKER:-}" ]] || return 0
  local observed_us
  remote_fixture_epoch_microseconds observed_us
  printf '%s %s\n' "$cancel_started_us" "$observed_us" \
    >"$DEVE_REMOTE_FIXTURE_TEST_HARD_EVENT_MARKER"
}
