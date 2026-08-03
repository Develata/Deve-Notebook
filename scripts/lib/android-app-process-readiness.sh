#!/usr/bin/env bash
# Stable Android app-process identity admission for target-host smokes.
#
# The caller supplies an absolute monotonic deadline, a PID probe callback,
# and a monotonic integer-seconds clock callback. One non-empty pidof sample is
# not admission: Android process bookkeeping can transiently expose, hide, and
# then expose the same just-created process. Two canonical samples of the same
# PID are required, with at most one empty bookkeeping sample between them.
# Once observed, that PID is the immutable launch identity: a different PID or
# two empty samples fail closed as replacement/exit. Probe failures other than
# ordinary pidof absence, invalid PID output, clock failures, and poll-sleep
# failures propagate fail-closed with bounded evidence.

if [[ -n "${ANDROID_APP_PROCESS_READINESS_LOADED:-}" ]]; then
  return 0
fi
ANDROID_APP_PROCESS_READINESS_LOADED=1

readonly ANDROID_APP_PROCESS_STABLE_SAMPLES_REQUIRED=2
readonly ANDROID_APP_PROCESS_POLL_INTERVAL_SECS=1
readonly ANDROID_APP_PROCESS_TRANSPORT_FAILURE_STATUS=70
ANDROID_APP_PROCESS_STABLE_PID=""
ANDROID_APP_PROCESS_CURRENT_PID=""
ANDROID_APP_PROCESS_MISSING_SAMPLES=0
ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="not-probed"

android_app_process_pidof_probe() {
  local adb_fn="$1"
  local app_id="$2"
  local output status

  if output="$("$adb_fn" shell pidof "$app_id" 2>&1)"; then
    status=0
  else
    status=$?
  fi
  output="${output//$'\r'/}"
  if (( status == 0 )); then
    [[ -n "$output" ]] || return 1
    printf '%s\n' "$output" | awk '{ print $1; exit }'
    return 0
  fi
  if (( status == 1 )) && [[ -z "$output" ]]; then
    return 1
  fi
  if [[ -n "$output" ]]; then
    printf 'android-app-process-readiness: pidof probe failed: %.512s\n' "$output" >&2
  fi
  if (( status == 1 )); then
    return "$ANDROID_APP_PROCESS_TRANSPORT_FAILURE_STATUS"
  fi
  return "$status"
}

android_app_process_read_clock() {
  local now_fn="$1"
  local now

  now="$("$now_fn")" || return $?
  [[ "$now" =~ ^[0-9]+$ ]] || return 1
  printf '%s\n' "$now"
}

android_app_process_wait_stable() {
  local deadline="$1"
  local probe_fn="$2"
  local now_fn="$3"
  local candidate_pid=""
  local stable_samples=0
  local missing_after_candidate=0
  local now observed status

  ANDROID_APP_PROCESS_STABLE_PID=""
  ANDROID_APP_PROCESS_CURRENT_PID=""
  ANDROID_APP_PROCESS_MISSING_SAMPLES=0
  ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="not-probed"
  [[ "$deadline" =~ ^[0-9]+$ ]] || {
    ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="deadline=invalid"
    return 1
  }

  while :; do
    now="$(android_app_process_read_clock "$now_fn")" || {
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="clock=invalid"
      return 1
    }
    if (( now >= deadline )); then
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="deadline=expired stable-samples=$stable_samples/$ANDROID_APP_PROCESS_STABLE_SAMPLES_REQUIRED"
      return 124
    fi

    observed=""
    if observed="$("$probe_fn")"; then
      status=0
    else
      status=$?
    fi
    observed="${observed//$'\r'/}"
    if (( status != 0 )) && ! { (( status == 1 )) && [[ -z "$observed" ]]; }; then
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="probe=failed status=$status"
      return "$status"
    fi
    if [[ -n "$observed" && ! "$observed" =~ ^[1-9][0-9]*$ ]]; then
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="probe=invalid-pid"
      return 1
    fi

    if [[ -z "$observed" && -z "$candidate_pid" ]]; then
      :
    elif [[ -z "$observed" ]]; then
      missing_after_candidate=$((missing_after_candidate + 1))
      if (( missing_after_candidate >= 2 )); then
        ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="process=absent-after-candidate pid=$candidate_pid missing-samples=$missing_after_candidate/2"
        return 1
      fi
    elif [[ -z "$candidate_pid" ]]; then
      candidate_pid="$observed"
      stable_samples=1
      missing_after_candidate=0
    elif [[ "$observed" != "$candidate_pid" ]]; then
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="process=replaced initial-pid=$candidate_pid current-pid=$observed"
      return 1
    else
      stable_samples=$((stable_samples + 1))
      missing_after_candidate=0
    fi

    if (( stable_samples >= ANDROID_APP_PROCESS_STABLE_SAMPLES_REQUIRED )); then
      ANDROID_APP_PROCESS_STABLE_PID="$candidate_pid"
      ANDROID_APP_PROCESS_CURRENT_PID="$candidate_pid"
      ANDROID_APP_PROCESS_MISSING_SAMPLES=0
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="stable pid=$candidate_pid samples=$stable_samples"
      return 0
    fi

    if sleep "$ANDROID_APP_PROCESS_POLL_INTERVAL_SECS"; then
      :
    else
      status=$?
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="poll-sleep=failed status=$status"
      return "$status"
    fi
  done
}

android_app_process_observe_anchored() {
  local expected_pid="$1"
  local probe_fn="$2"
  local observed status

  ANDROID_APP_PROCESS_CURRENT_PID=""
  [[ "$expected_pid" =~ ^[1-9][0-9]*$ ]] || {
    ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="anchor=invalid-pid"
    return 1
  }

  observed=""
  if observed="$("$probe_fn")"; then
    status=0
  else
    status=$?
  fi
  observed="${observed//$'\r'/}"
  if (( status != 0 )) && ! { (( status == 1 )) && [[ -z "$observed" ]]; }; then
    ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="probe=failed status=$status"
    return "$status"
  fi
  if [[ -n "$observed" && ! "$observed" =~ ^[1-9][0-9]*$ ]]; then
    ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="probe=invalid-pid"
    return 1
  fi

  if [[ -z "$observed" ]]; then
    ANDROID_APP_PROCESS_MISSING_SAMPLES=$((ANDROID_APP_PROCESS_MISSING_SAMPLES + 1))
    if (( ANDROID_APP_PROCESS_MISSING_SAMPLES >= 2 )); then
      ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="process=absent-after-admission pid=$expected_pid missing-samples=$ANDROID_APP_PROCESS_MISSING_SAMPLES/2"
      return 1
    fi
    ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="process=bookkeeping-gap pid=$expected_pid missing-samples=$ANDROID_APP_PROCESS_MISSING_SAMPLES/2"
    return 0
  fi
  if [[ "$observed" != "$expected_pid" ]]; then
    ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="process=replaced initial-pid=$expected_pid current-pid=$observed"
    return 1
  fi

  ANDROID_APP_PROCESS_CURRENT_PID="$observed"
  ANDROID_APP_PROCESS_MISSING_SAMPLES=0
  ANDROID_APP_PROCESS_READINESS_LAST_EVIDENCE="process=observed pid=$observed"
  return 0
}
