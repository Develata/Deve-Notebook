#!/usr/bin/env bash
# shellcheck shell=bash

# O_EXCL admission arbitration and the bounded publisher lifecycle. The formal
# Linux path uses a child-subreaper as the exact retained child so parent death
# cannot orphan the Bash publisher or any nested descendant.

remote_fixture_admission_publisher_delay() {
  local value=15
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 ]]; then
    value="${DEVE_REMOTE_FIXTURE_TEST_ADMISSION_PUBLISHER_DELAY:-2}"
  fi
  [[ "$value" =~ ^[0-9]+([.][0-9]+)?$ && "$value" != 0 && "$value" != 0.0 ]] || {
    remote_fixture_fail "fixture admission publisher delay must be a positive number"
    return 1
  }
  printf '%s\n' "$value"
}

remote_fixture_use_formal_admission_publisher() {
  [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" != 1 \
    || "${DEVE_REMOTE_FIXTURE_TEST_FORMAL_PUBLISHER:-0}" == 1 ]]
}

REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID=""
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_TOKEN=""
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP=0
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_SELF_CLEANING=0
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_COMPLETION_PATH=""
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_FAILURE_PATH=""
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_LAUNCHER_PATH=""
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_PATH=""
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_DEADLINE_PATH=""
REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_ADMISSION_PATH=""

remote_fixture_clear_admission_publisher_controls() {
  local path
  for path in \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_COMPLETION_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_FAILURE_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_LAUNCHER_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_DEADLINE_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_ADMISSION_PATH"; do
    [[ -z "$path" ]] || rm -f -- "$path"
  done
}

remote_fixture_abort_admission_publisher_lifecycle() {
  local status="$1"
  trap ':' INT TERM USR1
  remote_fixture_fail \
    "fixture admission publisher cleanup could not be proven; self-cleaning controls were preserved" || true
  # The formal retained child has PDEATHSIG bound to this exact supervisor.
  # Exiting is the only fail-closed transition when shell cleanup lost proof.
  exit "$status"
}

remote_fixture_initialize_admission_publisher_controls() {
  local state_dir="$1"
  local control_prefix="$state_dir/.startup-admission-publisher"
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_COMPLETION_PATH="$control_prefix.released"
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_FAILURE_PATH="$control_prefix.failure"
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_LAUNCHER_PATH="$control_prefix.launcher"
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_PATH="$control_prefix.root"
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_DEADLINE_PATH="$control_prefix.deadline"
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_ADMISSION_PATH="$control_prefix.root-admitted"
  local control_path
  for control_path in \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_COMPLETION_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_FAILURE_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_LAUNCHER_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_DEADLINE_PATH" \
    "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_ADMISSION_PATH"; do
    [[ ! -e "$control_path" && ! -L "$control_path" ]] || {
      remote_fixture_fail "fixture admission publisher control path already exists or is unsafe"
      return 1
    }
  done
  return 0
}

remote_fixture_spawn_admission_publisher() {
  local state_dir="$1"
  local decision_path="$2"
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID=""
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_TOKEN=""
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP=0
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_SELF_CLEANING=0
  if ! remote_fixture_use_formal_admission_publisher; then
    remote_fixture_admit_startup_state "$state_dir" "$decision_path" &
  else
    local entry_script="${DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT:-}"
    [[ -f "$entry_script" && ! -L "$entry_script" ]] || {
      remote_fixture_fail "fixture admission publisher entry script is missing or unsafe"
      return 1
    }
    remote_fixture_require_command setsid
    remote_fixture_require_command python3
    local supervisor_pid="$BASHPID"
    DEVE_REMOTE_FIXTURE_ADMISSION_SUPERVISOR_PID="$supervisor_pid" \
      DEVE_REMOTE_FIXTURE_SUBREAPER_ADMISSION_PATH="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_ADMISSION_PATH" \
      setsid python3 "$REMOTE_FIXTURE_BOUNDED_SUBREAPER" \
        "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_COMPLETION_PATH" \
        "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_FAILURE_PATH" \
        "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_LAUNCHER_PATH" \
        "$supervisor_pid" -- \
        bash "$entry_script" __admit-startup "$state_dir" "$decision_path" &
    REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP=1
    REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_SELF_CLEANING=1
  fi
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID="$!"
  if ((REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_SELF_CLEANING == 1)); then
    (umask 077; set -o noclobber; printf '%s\n' \
      "$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID" \
      >"$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_PATH") 2>/dev/null || return 1
  fi

  local pid="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID"
  local token="" token_attempt
  for ((token_attempt = 0; token_attempt < 100; token_attempt++)); do
    token="$(remote_fixture_capture_start_worker_token "$pid" 2>/dev/null || true)"
    [[ -n "$token" ]] && break
    remote_fixture_pid_active "$pid" || {
      remote_fixture_use_formal_admission_publisher || return 0
      local pretoken_group_status=0
      remote_fixture_owned_process_group_status \
        "fixture admission publisher" "$pid" || pretoken_group_status=$?
      if ((pretoken_group_status == 1)); then
        REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP=0
        remote_fixture_fail \
          "fixture admission publisher exited before identity admission after empty-group proof"
      else
        remote_fixture_fail \
          "fixture admission publisher exited before identity admission with a retained or unprovable process group"
      fi
      return 1
    }
    sleep 0.01
  done
  if [[ -z "$token" ]]; then
    remote_fixture_fail "fixture admission publisher process token is unavailable"
    return 1
  fi
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_TOKEN="$token"
  if ((REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP == 1)); then
    local root_status=0 group_status=0
    if ! remote_fixture_wait_isolated_process_group "$pid" \
      || ! remote_fixture_bind_isolated_process_group "$pid" "$token"; then
      remote_fixture_root_identity_status "$pid" "$token" || root_status=$?
      if ((root_status == 1)); then
        remote_fixture_owned_process_group_status \
          "fixture admission publisher" "$pid" || group_status=$?
        if ((group_status == 1)); then
          # No live capability remains. Permit only non-signalling cleanup of
          # the exact retained event after proving the former group empty.
          REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP=0
          remote_fixture_fail "fixture admission publisher exited before process-group admission"
          return 1
        fi
      fi
      remote_fixture_fail "fixture admission publisher process group could not be bound"
      return 1
    fi
    (umask 077; set -o noclobber; printf 'admitted\n' \
      >"$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_ROOT_ADMISSION_PATH") \
      2>/dev/null || {
        remote_fixture_fail "fixture admission publisher root capability could not be published"
        return 1
      }
  fi
}

remote_fixture_stop_admission_publisher() {
  local pid="$1"
  local token="$2"
  local process_group="$3"
  if [[ -z "$token" ]]; then
    remote_fixture_pid_active "$pid" || {
      if ((process_group == 1)); then
        local group_status=0
        remote_fixture_owned_process_group_status \
          "fixture admission publisher" "$pid" || group_status=$?
        if ((group_status != 1)); then
          remote_fixture_fail \
            "cannot retire a pretoken publisher without an empty former process group"
          return 1
        fi
      fi
      wait -n "$pid" 2>/dev/null || true
      remote_fixture_clear_admission_publisher_controls
      return 0
    }
    remote_fixture_fail "cannot stop a live fixture admission publisher without its process token"
    return 1
  fi
  if ((process_group == 1 \
    && REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_SELF_CLEANING == 1)); then
    remote_fixture_stop_bounded_subreaper_tree \
      "fixture admission publisher" "$pid" "$token" 1 || return 1
    remote_fixture_clear_admission_publisher_controls
    return 0
  fi
  remote_fixture_stop_bounded_tree \
    "fixture admission publisher" "$pid" "$process_group" "$token" 1 1 || return 1
  local root_status=0
  remote_fixture_root_identity_status "$pid" "$token" || root_status=$?
  ((root_status == 1)) || {
    remote_fixture_fail "fixture admission publisher survived bounded cleanup"
    return 1
  }
  remote_fixture_clear_admission_publisher_controls
}

REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_STATUS=0
remote_fixture_read_admission_publisher_status() {
  local status_path="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_FAILURE_PATH"
  [[ -e "$status_path" || -L "$status_path" ]] || return 1
  [[ -f "$status_path" && ! -L "$status_path" ]] || {
    remote_fixture_fail "fixture admission publisher status path is unsafe"
    return 2
  }
  local -a lines=()
  mapfile -t lines <"$status_path" || {
    remote_fixture_fail "fixture admission publisher status could not be read"
    return 2
  }
  ((${#lines[@]} == 1)) && [[ "${lines[0]}" =~ ^[0-9]{1,3}$ ]] \
    && ((10#${lines[0]} <= 255)) || {
    remote_fixture_fail "fixture admission publisher status is invalid"
    return 2
  }
  REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_STATUS="$((10#${lines[0]}))"
}

remote_fixture_finalize_admission_publisher() {
  local pid="$1"
  local token="$2"
  local process_group="$3"
  if ((process_group == 1)); then
    local group_status=0
    remote_fixture_owned_process_group_status \
      "fixture admission publisher" "$pid" || group_status=$?
    if ((group_status != 1)); then
      remote_fixture_fail "fixture admission publisher exited with a retained process group"
      return 1
    fi
    remote_fixture_forget_isolated_process_group "$pid" "$token"
  fi
  remote_fixture_clear_admission_publisher_controls
}

remote_fixture_claim_start_admission_decision() {
  local decision_path="$1"
  (umask 077; set -o noclobber; : >"$decision_path") 2>/dev/null || true
}

remote_fixture_admit_start_worker() {
  local state_dir="$1"
  local admission_status=0 completed_pid="" publisher_pid="" publisher_token=""
  local publisher_process_group=0 publisher_identity_status=0
  local publisher_status_probe=0 publisher_payload_status=0 publisher_timer_status=0
  local deadline_marker_probe=1
  local publisher_timer_completed=0
  local publisher_timer_pid="" publisher_timer_token="" publisher_delay=""
  local observation_pid="" observation_delay=""
  state_dir="$(remote_fixture_existing_state_dir "$state_dir")" || return 1
  REMOTE_FIXTURE_START_ADMISSION_DECISION="$state_dir/.startup-admission-decision"
  if ((signal_status != 0)); then
    REMOTE_FIXTURE_START_ADMISSION_DECISION=""
    return 3
  fi
  publisher_delay="$(remote_fixture_admission_publisher_delay)" || return 1
  observation_delay="$(remote_fixture_supervisor_observation_delay)" || return 1
  remote_fixture_initialize_admission_publisher_controls "$state_dir" || {
    REMOTE_FIXTURE_START_ADMISSION_DECISION=""
    return 1
  }
  REMOTE_FIXTURE_START_ADMISSION_OWNER_PID="$BASHPID"
  # The fixed deadline includes spawn plus every identity/group admission probe.
  remote_fixture_start_admission_publisher_timer \
    "$publisher_delay" "$REMOTE_FIXTURE_START_ADMISSION_DECISION" || {
      REMOTE_FIXTURE_START_ADMISSION_OWNER_PID=""
      REMOTE_FIXTURE_START_ADMISSION_DECISION=""
      remote_fixture_clear_admission_publisher_controls
      return 1
    }
  publisher_timer_pid="$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID"
  publisher_timer_token="$REMOTE_FIXTURE_START_ADMISSION_TIMER_TOKEN"
  remote_fixture_spawn_admission_publisher \
    "$state_dir" "$REMOTE_FIXTURE_START_ADMISSION_DECISION" \
    || publisher_identity_status=$?
  publisher_pid="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PID"
  publisher_token="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_TOKEN"
  publisher_process_group="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_PROCESS_GROUP"
  if ((publisher_identity_status != 0)); then
    remote_fixture_claim_start_admission_decision "$REMOTE_FIXTURE_START_ADMISSION_DECISION"
    local identity_cleanup_status=0
    remote_fixture_stop_admission_publisher_timer \
      "$publisher_timer_pid" "$publisher_timer_token" || identity_cleanup_status=1
    [[ -z "$publisher_pid" ]] || remote_fixture_stop_admission_publisher \
      "$publisher_pid" "$publisher_token" "$publisher_process_group" \
      || identity_cleanup_status=$?
    [[ -n "$publisher_pid" ]] || remote_fixture_clear_admission_publisher_controls
    ((identity_cleanup_status == 0)) \
      || remote_fixture_abort_admission_publisher_lifecycle 1
    REMOTE_FIXTURE_START_ADMISSION_OWNER_PID=""
    REMOTE_FIXTURE_START_ADMISSION_DECISION=""
    return 1
  fi
  while :; do
    if ((signal_status != 0)); then
      admission_status=0
      remote_fixture_stop_admission_publisher_timer \
        "$publisher_timer_pid" "$publisher_timer_token" || admission_status=1
      remote_fixture_stop_admission_publisher \
        "$publisher_pid" "$publisher_token" "$publisher_process_group" \
        || admission_status=1
      if ((admission_status != 0)); then
        remote_fixture_abort_admission_publisher_lifecycle "$signal_status"
      fi
      admission_status=3
      break
    fi
    remote_fixture_start_supervisor_timer "$observation_delay"
    observation_pid="$REMOTE_FIXTURE_SUPERVISOR_TIMER_PID"
    admission_status=0
    completed_pid=""
    publisher_timer_completed=0
    local -a waitable_pids=("$publisher_pid" "$observation_pid")
    [[ -z "$publisher_timer_pid" ]] || waitable_pids+=("$publisher_timer_pid")
    wait -n -p completed_pid "${waitable_pids[@]}" || admission_status=$?
    if [[ -n "$publisher_timer_pid" \
      && "${completed_pid:-}" == "$publisher_timer_pid" ]]; then
      publisher_timer_status="$admission_status"
      remote_fixture_retire_consumed_admission_timer_identity \
        publisher_timer_pid publisher_timer_token || {
        admission_status=125
        break
      }
      publisher_timer_completed=1
    fi
    remote_fixture_reap_observation_tick "$observation_pid" || {
      remote_fixture_claim_start_admission_decision "$REMOTE_FIXTURE_START_ADMISSION_DECISION"
      local observation_cleanup_status=0
      remote_fixture_stop_admission_publisher_timer \
        "$publisher_timer_pid" "$publisher_timer_token" || observation_cleanup_status=1
      remote_fixture_stop_admission_publisher \
        "$publisher_pid" "$publisher_token" "$publisher_process_group" \
        || observation_cleanup_status=$?
      ((observation_cleanup_status == 0)) \
        || remote_fixture_abort_admission_publisher_lifecycle 1
      admission_status=1
      break
    }
    # An independently published deadline marker is already a completed
    # deadline transition. Observe it before cancellation or publisher status
    # so scheduler order cannot turn the same marker into two outcomes.
    deadline_marker_probe=0
    remote_fixture_admission_deadline_status || deadline_marker_probe=$?
    if ((deadline_marker_probe != 1)); then
      if [[ -n "$publisher_timer_pid" ]]; then
        remote_fixture_stop_admission_publisher_timer \
          "$publisher_timer_pid" "$publisher_timer_token" || deadline_marker_probe=2
        publisher_timer_pid=""
        publisher_timer_token=""
      fi
      publisher_timer_completed=1
      ((deadline_marker_probe == 0)) \
        && publisher_timer_status=0 \
        || publisher_timer_status=2
    fi
    if ((publisher_timer_completed == 1)); then
      if ((publisher_timer_status == 0)); then
        remote_fixture_fail "fixture admission publisher exceeded its bounded deadline" || true
      elif ((publisher_timer_status == 1)); then
        remote_fixture_fail \
          "fixture admission publisher decision lacked a status before its bounded deadline" || true
      else
        remote_fixture_fail "fixture admission deadline decision failed closed" || true
      fi
      local deadline_cleanup_status=0
      remote_fixture_stop_admission_publisher \
        "$publisher_pid" "$publisher_token" "$publisher_process_group" \
        || deadline_cleanup_status=$?
      ((deadline_cleanup_status == 0)) \
        || remote_fixture_abort_admission_publisher_lifecycle 1
      admission_status=1
      break
    fi
    # Without a deadline marker, cancellation observed in the same event cycle
    # wins before a completed Bash publisher status can be accepted.
    if ((signal_status != 0)); then
      continue
    fi
    publisher_status_probe=0
    remote_fixture_read_admission_publisher_status || publisher_status_probe=$?
    if ((publisher_status_probe == 0)); then
      local post_status_deadline_probe=0
      remote_fixture_admission_deadline_status || post_status_deadline_probe=$?
      if ((post_status_deadline_probe != 1)); then
        local post_status_cleanup_status=0
        remote_fixture_stop_admission_publisher_timer \
          "$publisher_timer_pid" "$publisher_timer_token" \
          || post_status_cleanup_status=1
        publisher_timer_pid=""
        publisher_timer_token=""
        remote_fixture_stop_admission_publisher \
          "$publisher_pid" "$publisher_token" "$publisher_process_group" \
          || post_status_cleanup_status=$?
        ((post_status_cleanup_status == 0)) \
          || remote_fixture_abort_admission_publisher_lifecycle 1
        if ((post_status_deadline_probe == 0)); then
          remote_fixture_fail \
            "fixture admission publisher status crossed its bounded deadline" || true
        else
          remote_fixture_fail "fixture admission deadline marker became unsafe" || true
        fi
        admission_status=1
        break
      fi
      publisher_payload_status="$REMOTE_FIXTURE_START_ADMISSION_PUBLISHER_STATUS"
      local status_cleanup_status=0
      remote_fixture_stop_admission_publisher_timer \
        "$publisher_timer_pid" "$publisher_timer_token" || status_cleanup_status=1
      remote_fixture_stop_admission_publisher \
        "$publisher_pid" "$publisher_token" "$publisher_process_group" \
        || status_cleanup_status=$?
      ((status_cleanup_status == 0)) \
        || remote_fixture_abort_admission_publisher_lifecycle 1
      admission_status="$publisher_payload_status"
      break
    fi
    if ((publisher_status_probe == 2)); then
      remote_fixture_claim_start_admission_decision "$REMOTE_FIXTURE_START_ADMISSION_DECISION"
      local status_failure_cleanup_status=0
      remote_fixture_stop_admission_publisher_timer \
        "$publisher_timer_pid" "$publisher_timer_token" || status_failure_cleanup_status=1
      remote_fixture_stop_admission_publisher \
        "$publisher_pid" "$publisher_token" "$publisher_process_group" \
        || status_failure_cleanup_status=$?
      ((status_failure_cleanup_status == 0)) \
        || remote_fixture_abort_admission_publisher_lifecycle 1
      admission_status=1
      break
    fi
    if [[ "${completed_pid:-}" == "$publisher_pid" ]]; then
      local finalize_status=0
      remote_fixture_stop_admission_publisher_timer \
        "$publisher_timer_pid" "$publisher_timer_token" || finalize_status=1
      remote_fixture_finalize_admission_publisher \
        "$publisher_pid" "$publisher_token" "$publisher_process_group" \
        || finalize_status=$?
      ((finalize_status == 0)) \
        || remote_fixture_abort_admission_publisher_lifecycle 1
      break
    fi
    if [[ "${completed_pid:-}" == "$observation_pid" ]]; then
      continue
    fi
    if ((admission_status > 128)); then
      # A parent trap interrupted wait without consuming the publisher.
      continue
    fi
    remote_fixture_fail "fixture admission publisher returned without an exact child event"
    admission_status=125
    break
  done
  REMOTE_FIXTURE_START_ADMISSION_OWNER_PID=""
  REMOTE_FIXTURE_START_ADMISSION_DECISION=""
  return "$admission_status"
}
