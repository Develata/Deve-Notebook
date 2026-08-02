#!/usr/bin/env bash
# shellcheck shell=bash

# Supervises the Unix start worker so runner INT/TERM cannot publish a fixture
# while its caller reports cancellation. The worker announces trap readiness
# before creating any owned resource.

start_fixture() {
  local state_dir=""
  local index
  local -a worker_args=("$@")
  for index in "${!worker_args[@]}"; do
    if [[ "${worker_args[$index]}" == "--state-dir" ]]; then
      state_dir="${worker_args[$((index + 1))]:-}"
    fi
  done

  local parent_pid="$BASHPID"
  local worker_pid=""
  local worker_status=0
  local signal_status=0
  local pending_signal=""
  local worker_ready=0
  local signal_forwarded=0
  trap 'worker_ready=1' USR1
  trap 'signal_status=130; pending_signal=INT' INT
  trap 'signal_status=143; pending_signal=TERM' TERM
  DEVE_REMOTE_FIXTURE_START_PARENT_PID="$parent_pid" start_fixture_worker "$@" &
  worker_pid="$!"
  while remote_fixture_job_active "$worker_pid"; do
    if ((signal_status != 0 && worker_ready == 1 && signal_forwarded == 0)); then
      kill -s "$pending_signal" -- "$worker_pid" 2>/dev/null || true
      signal_forwarded=1
    fi
    sleep 0.1
  done
  if wait "$worker_pid"; then worker_status=0; else worker_status=$?; fi
  trap - USR1 INT TERM

  if ((signal_status != 0)); then
    if ((worker_status == 0)); then
      local cleanup_status=0
      if [[ -z "$state_dir" ]]; then
        cleanup_status=2
      else
        stop_fixture --state-dir "$state_dir" >/dev/null || cleanup_status=$?
      fi
      if ((cleanup_status != 0)); then
        remote_fixture_fail \
          "cancellation rollback failed; signal_status=$signal_status; cleanup_status=$cleanup_status" || true
      fi
    fi
    return "$signal_status"
  fi
  return "$worker_status"
}
