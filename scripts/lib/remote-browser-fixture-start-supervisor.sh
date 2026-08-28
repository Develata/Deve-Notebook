#!/usr/bin/env bash
# shellcheck shell=bash
# Token-bound, event-driven Unix startup supervisor. It owns cancellation while
# the worker is active, then restores an outer rollback trap before returning.
REMOTE_FIXTURE_START_SIGNAL_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-start-signals.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-signals.sh
source "$REMOTE_FIXTURE_START_SIGNAL_LIB"
REMOTE_FIXTURE_START_PROCESS_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-start-process.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-process.sh
source "$REMOTE_FIXTURE_START_PROCESS_LIB"
REMOTE_FIXTURE_START_PUBLISHER_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-start-publisher.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-publisher.sh
source "$REMOTE_FIXTURE_START_PUBLISHER_LIB"
REMOTE_FIXTURE_LIFECYCLE_STATE_DIR=""

remote_fixture_outer_cancel() {
  local signal_status="$1"
  trap ':' INT TERM
  local cleanup_status=0
  if [[ -n "$REMOTE_FIXTURE_LIFECYCLE_STATE_DIR" ]]; then
    remote_fixture_cancel_owned_state "$REMOTE_FIXTURE_LIFECYCLE_STATE_DIR" \
      >/dev/null || cleanup_status=$?
  fi
  if ((cleanup_status != 0)); then
    remote_fixture_fail \
      "outer cancellation rollback failed; signal_status=$signal_status; cleanup_status=$cleanup_status" || true
  fi
  exit "$signal_status"
}
remote_fixture_unverified_start_cancel() {
  local signal_status="$1"
  trap ':' INT TERM
  REMOTE_FIXTURE_LIFECYCLE_STATE_DIR=""
  remote_fixture_fail \
    "cancellation arrived after an unverified worker-tree failure; startup ownership was preserved" || true
  exit "$signal_status"
}
remote_fixture_arm_outer_start_lifecycle() {
  REMOTE_FIXTURE_LIFECYCLE_STATE_DIR="$1"
  trap 'remote_fixture_outer_cancel 130' INT
  trap 'remote_fixture_outer_cancel 143' TERM
}
start_fixture() {
  local state_dir=""
  local index
  local -a worker_args=("$@")
  for index in "${!worker_args[@]}"; do
    if [[ "${worker_args[$index]}" == "--state-dir" ]]; then
      state_dir="${worker_args[$((index + 1))]:-}"
    fi
  done
  remote_fixture_require_wait_any_child || return 1
  remote_fixture_arm_outer_start_lifecycle "$state_dir"
  local parent_pid="$BASHPID"
  local parent_token=""
  local worker_pid="" worker_token="" worker_status=0 worker_process_group=0
  local signal_status=0 pending_signal="" forwarded_signal="" worker_ready=0 readiness_event=0
  local event_pending=0
  local cancel_started_realtime="" cancel_started_us=0
  local grace_expired=0 hard_expired=0 identity_failed=0 worker_reaped=0 worker_tree_reaped=0
  local termination_forbidden=0
  local worker_group_unverified=0
  local token_probe_unavailable=0
  local test_coalesced_cancel_injected=0
  local test_post_hard_check_injected=0
  local test_wait_check_injected=0
  local supervisor_handoff_to_outer=0
  local grace_timer_pid="" hard_timer_pid="" timers_started=0
  local term_delay hard_delay
  term_delay="$(remote_fixture_supervisor_delay term 2)" || return 1
  hard_delay="$(remote_fixture_supervisor_delay hard 45)" || return 1
  trap 'readiness_event=1; event_pending=1' USR1
  trap 'remote_fixture_latch_start_cancel INT' INT
  trap 'remote_fixture_latch_start_cancel TERM' TERM
  parent_token="$(remote_fixture_process_token "$parent_pid" 2>/dev/null || true)"
  if [[ -z "$parent_token" ]]; then
    trap - USR1
    remote_fixture_arm_outer_start_lifecycle "$state_dir"
    if ((signal_status != 0)); then remote_fixture_outer_cancel "$signal_status"; fi
    remote_fixture_fail "fixture startup supervisor process token is unavailable"
    return 1
  fi
  if ! remote_fixture_spawn_start_worker "$parent_pid" "$parent_token" "$@"; then
    remote_fixture_stop_supervisor_timer "$grace_timer_pid"
    remote_fixture_stop_supervisor_timer "$hard_timer_pid"
    trap - USR1
    remote_fixture_arm_outer_start_lifecycle "$state_dir"
    if ((signal_status != 0)); then remote_fixture_outer_cancel "$signal_status"; fi
    return 1
  fi
  worker_pid="$REMOTE_FIXTURE_START_WORKER_PID"
  worker_process_group="$REMOTE_FIXTURE_START_WORKER_PROCESS_GROUP"
  worker_token="$(remote_fixture_capture_start_worker_token "$worker_pid" 2>/dev/null || true)"
  if ((worker_process_group == 1)) \
    && ! remote_fixture_wait_isolated_process_group "$worker_pid"; then
    identity_failed=1
    termination_forbidden=1
    worker_group_unverified=1
    worker_process_group=0
  fi
  if ((worker_process_group == 1)) && [[ -z "$worker_token" ]]; then
    local token_attempt
    for token_attempt in $(seq 1 100); do
      worker_token="$(remote_fixture_capture_start_worker_token "$worker_pid" 2>/dev/null || true)"
      [[ -n "$worker_token" ]] && break
      remote_fixture_pid_active "$worker_pid" || break
      sleep 0.01
    done
  fi
  if ((worker_process_group == 1)) && [[ -n "$worker_token" ]] \
    && ! remote_fixture_bind_isolated_process_group "$worker_pid" "$worker_token"; then
    identity_failed=1
    termination_forbidden=1
    worker_group_unverified=1
    worker_process_group=0
    remote_fixture_signal_start_worker "$worker_pid" "$worker_token" TERM || true
  fi
  ((identity_failed == 0)) || event_pending=1

  while :; do
    local wait_status=0 active_status=0 completed_pid="" reap_hint=129
    if ((event_pending == 0)); then
      if remote_fixture_wait_supervisor_event \
        "$worker_pid" "$grace_timer_pid" "$hard_timer_pid"; then
        completed_pid="$REMOTE_FIXTURE_SUPERVISOR_WAIT_PID"
        wait_status="$REMOTE_FIXTURE_SUPERVISOR_WAIT_STATUS"
      else
        completed_pid=""
        wait_status=125
        identity_failed=1
      fi
    else
      event_pending=0
      wait_status=129
    fi
    ((wait_status <= 128)) || reap_hint="$wait_status"
    if [[ "$completed_pid" == "$worker_pid" ]]; then
      worker_status="$wait_status"
      worker_reaped=1
      ((worker_process_group == 0 && worker_group_unverified == 0)) && worker_tree_reaped=1
      break
    elif [[ -n "$grace_timer_pid" && "$completed_pid" == "$grace_timer_pid" ]]; then
      grace_expired=1
      grace_timer_pid=""
    elif [[ -n "$hard_timer_pid" && "$completed_pid" == "$hard_timer_pid" ]]; then
      hard_expired=1
      hard_timer_pid=""
    elif [[ -n "$completed_pid" ]]; then
      remote_fixture_fail "fixture supervisor consumed an unknown child event"
      identity_failed=1
    fi
    if [[ -z "$worker_token" ]]; then
      if ! remote_fixture_pid_active "$worker_pid"; then
        if remote_fixture_pid_exists "$worker_pid"; then
          token_probe_unavailable=1
        else
          remote_fixture_reap_start_worker_status "$worker_pid" "$reap_hint" || {
            identity_failed=1
            termination_forbidden=1
            break
          }
          worker_status="$REMOTE_FIXTURE_REAPED_WORKER_STATUS"
          worker_reaped=1
          ((worker_process_group == 0 && worker_group_unverified == 0)) && worker_tree_reaped=1
          break
        fi
      fi
      worker_token="$(remote_fixture_capture_start_worker_token "$worker_pid" 2>/dev/null || true)"
      [[ -n "$worker_token" ]] || token_probe_unavailable=1
    fi
    if [[ -n "$worker_token" ]]; then
      if ((worker_process_group == 1)) \
        && ! remote_fixture_process_group_is_bound "$worker_pid" "$worker_token" \
        && ! remote_fixture_bind_isolated_process_group "$worker_pid" "$worker_token"; then
        identity_failed=1
        termination_forbidden=1
        break
      fi
      remote_fixture_start_worker_active "$worker_pid" "$worker_token" || active_status=$?
      if ((active_status == 1)); then
        remote_fixture_reap_start_worker_status "$worker_pid" "$reap_hint" || {
          identity_failed=1
          termination_forbidden=1
          break
        }
        worker_status="$REMOTE_FIXTURE_REAPED_WORKER_STATUS"
        worker_reaped=1
        ((worker_process_group == 0 && worker_group_unverified == 0)) && worker_tree_reaped=1
        break
      elif ((active_status == 2)); then
        identity_failed=1
        termination_forbidden=1
        worker_status="$wait_status"
        break
      elif ((active_status == 3)); then
        token_probe_unavailable=1
      fi
    fi

    # wait(1) returned because one of our traps ran; the exact worker is still
    # alive. Process the accumulated event without polling its process table.
    if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
      && "$test_coalesced_cancel_injected" == 0 \
      && "$readiness_event" == 1 \
      && "$worker_ready" == 0 \
      && -n "${DEVE_REMOTE_FIXTURE_TEST_COALESCED_CANCEL:-}" ]]; then
      case "$DEVE_REMOTE_FIXTURE_TEST_COALESCED_CANCEL" in
        INT|TERM) kill -s "$DEVE_REMOTE_FIXTURE_TEST_COALESCED_CANCEL" "$BASHPID" ;;
        *) remote_fixture_fail "invalid coalesced-cancel test signal"; identity_failed=1 ;;
      esac
      test_coalesced_cancel_injected=1
    fi
    # Cancellation wins when readiness and a parent signal coalesce in the
    # same wait cycle: never publish admission to a cancelled worker.
    if ((identity_failed == 0 && signal_status == 0 && readiness_event == 1 && worker_ready == 0 && token_probe_unavailable == 0)) \
      && [[ -n "$worker_token" ]]; then
      local admission_status=0
      remote_fixture_admit_start_worker "$state_dir" || admission_status=$?
      if ((admission_status == 0)); then
        worker_ready=1
        readiness_event=0
      elif ((admission_status == 3 && signal_status != 0)); then
        readiness_event=0
      else
        identity_failed=1
      fi
    fi
    if ((signal_status != 0 && timers_started == 0)); then
      remote_fixture_start_latched_cancel_timers || identity_failed=1
    fi
    local signal_result=0
    if ((token_probe_unavailable == 0)); then
      if [[ "$pending_signal" == TERM && "$forwarded_signal" != TERM ]]; then
        remote_fixture_signal_start_worker "$worker_pid" "$worker_token" TERM \
          || signal_result=$?
        if ((signal_result == 0)); then forwarded_signal=TERM
        elif ((signal_result == 3)); then token_probe_unavailable=1
        else identity_failed=1; termination_forbidden=1
        fi
      elif [[ "$pending_signal" == INT && -z "$forwarded_signal" ]]; then
        remote_fixture_signal_start_worker "$worker_pid" "$worker_token" INT \
          || signal_result=$?
        if ((signal_result == 0)); then forwarded_signal=INT
        elif ((signal_result == 3)); then token_probe_unavailable=1
        else identity_failed=1; termination_forbidden=1
        fi
      fi
    fi
    if ((grace_expired == 1 && token_probe_unavailable == 0)) && [[ "$forwarded_signal" != TERM ]]; then
      signal_result=0
      remote_fixture_signal_start_worker "$worker_pid" "$worker_token" TERM \
        || signal_result=$?
      if ((signal_result == 0)); then forwarded_signal=TERM
      elif ((signal_result == 3)); then token_probe_unavailable=1
      else identity_failed=1; termination_forbidden=1
      fi
    fi
    if ((hard_expired == 1)); then
      remote_fixture_record_test_hard_event
      trap '' USR1
      remote_fixture_stop_supervisor_timer "$grace_timer_pid"
      grace_timer_pid=""
      remote_fixture_stop_supervisor_timer "$hard_timer_pid"
      hard_timer_pid=""
      if remote_fixture_terminate_waitable_start_worker \
        "$worker_pid" "$worker_token" "$worker_process_group" 1; then
        worker_reaped=1
        worker_tree_reaped=1
      else
        identity_failed=1
        termination_forbidden=1
      fi
      worker_status=137
      break
    fi
    if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
      && "${DEVE_REMOTE_FIXTURE_TEST_POST_HARD_CHECK_CANCEL:-}" == TERM \
      && "$test_post_hard_check_injected" == 0 ]]; then
      remote_fixture_latch_start_cancel TERM
      while remote_fixture_job_active "$hard_timer_pid"; do sleep 0.01; done
      test_post_hard_check_injected=1
    fi
    token_probe_unavailable=0
    ((identity_failed == 0)) || break
  done

  remote_fixture_stop_supervisor_timer "$grace_timer_pid"
  remote_fixture_stop_supervisor_timer "$hard_timer_pid"
  local cleanup_required=0
  if ((signal_status != 0 || identity_failed != 0 || worker_status != 0)); then
    cleanup_required=1
  fi
  # A reaped group leader is not a tree-empty proof. On cancellation/failure,
  # consume the capability bound before admission and prove the retained PGID
  # empty before startup-state recovery is allowed.
  if ((cleanup_required == 1 && worker_reaped == 1 && worker_process_group == 1)); then
    if remote_fixture_stop_bounded_tree \
      "fixture start worker" "$worker_pid" 1 "$worker_token"; then
      worker_tree_reaped=1
    else
      identity_failed=1
      termination_forbidden=1
      worker_tree_reaped=0
    fi
  fi
  # A failed tree proof is terminal for this cancellation attempt. Never retry
  # based only on a now-gone root and then authorize journal recovery while an
  # unverified descendant may still exist.
  if ((worker_reaped == 0 && termination_forbidden == 0)); then
    trap ':' USR1
    if remote_fixture_terminate_waitable_start_worker \
      "$worker_pid" "$worker_token" "$worker_process_group"; then
      worker_reaped=1
      worker_tree_reaped=1
    else
      identity_failed=1
      termination_forbidden=1
    fi
  fi

  local cleanup_status=0 cleanup_attempted=0
  if ((cleanup_required == 1 || identity_failed != 0)); then
    cleanup_attempted=1
    trap ':' USR1
    if ((worker_tree_reaped == 1)); then
      remote_fixture_cancel_owned_state "$state_dir" >/dev/null || cleanup_status=$?
    else
      cleanup_status=1
    fi
    if ((cleanup_status != 0)); then
      remote_fixture_fail \
        "cancellation rollback failed; signal_status=$signal_status; cleanup_status=$cleanup_status" || true
    fi
  fi

  if ((cleanup_required == 0 || worker_tree_reaped == 1)); then
    if ((cleanup_required == 0 && worker_process_group == 1)); then
      remote_fixture_forget_isolated_process_group "$worker_pid" "$worker_token"
    fi
    # Restore rollback ownership only after the worker tree has been proven
    # gone. A latch captured before the handoff flag is re-sampled once; later
    # signals execute outer rollback immediately.
    if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
      && -n "${DEVE_REMOTE_FIXTURE_TEST_PRE_OUTER_HANDOFF_SIGNAL:-}" ]]; then
      case "$DEVE_REMOTE_FIXTURE_TEST_PRE_OUTER_HANDOFF_SIGNAL" in
        INT|TERM) kill -s "$DEVE_REMOTE_FIXTURE_TEST_PRE_OUTER_HANDOFF_SIGNAL" "$BASHPID" ;;
        *) remote_fixture_fail "invalid pre-outer-handoff test signal"; return 1 ;;
      esac
    fi
    supervisor_handoff_to_outer=1
    if ((signal_status != 0 && cleanup_attempted == 0)); then
      cleanup_attempted=1
      cleanup_status=0
      remote_fixture_cancel_owned_state "$state_dir" >/dev/null || cleanup_status=$?
      if ((cleanup_status != 0)); then
        remote_fixture_fail \
          "handoff cancellation rollback failed; signal_status=$signal_status; cleanup_status=$cleanup_status" || true
      fi
    fi
    trap - USR1
    if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
      && -n "${DEVE_REMOTE_FIXTURE_TEST_OUTER_HANDOFF_SIGNAL:-}" ]]; then
      case "$DEVE_REMOTE_FIXTURE_TEST_OUTER_HANDOFF_SIGNAL" in
        INT|TERM) kill -s "$DEVE_REMOTE_FIXTURE_TEST_OUTER_HANDOFF_SIGNAL" "$BASHPID" ;;
        *) remote_fixture_fail "invalid outer-handoff test signal"; return 1 ;;
      esac
    fi
    remote_fixture_arm_outer_start_lifecycle "$state_dir"
  else
    # Never hand an unverified live tree to journal recovery. Keep readiness
    # harmless and install a terminal handler that preserves the state.
    REMOTE_FIXTURE_LIFECYCLE_STATE_DIR=""
    trap ':' USR1
    trap 'remote_fixture_unverified_start_cancel 130' INT
    trap 'remote_fixture_unverified_start_cancel 143' TERM
  fi
  if ((signal_status != 0)); then return "$signal_status"; fi
  if ((identity_failed != 0)); then
    remote_fixture_fail "fixture start worker identity changed before it was reaped"
    return 1
  fi
  return "$worker_status"
}
