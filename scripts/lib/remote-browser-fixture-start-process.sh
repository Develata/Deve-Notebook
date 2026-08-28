#!/usr/bin/env bash
# shellcheck shell=bash
# Owns start-worker process creation, token checks, signalling, and exact reap.
REMOTE_FIXTURE_SIGNAL_EXEC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-signal-exec.py"

remote_fixture_capture_start_worker_token() {
  local pid="$1"
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
    && "${DEVE_REMOTE_FIXTURE_TEST_FORCE_TOKEN_UNAVAILABLE:-0}" == 1 ]]; then
    return 1
  fi
  remote_fixture_process_token "$pid"
}

REMOTE_FIXTURE_START_WORKER_PID=""
REMOTE_FIXTURE_START_WORKER_PROCESS_GROUP=0
remote_fixture_spawn_start_worker() {
  local parent_pid="$1"
  local parent_token="$2"
  shift 2
  REMOTE_FIXTURE_START_WORKER_PID=""
  REMOTE_FIXTURE_START_WORKER_PROCESS_GROUP=0
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 \
    && "${DEVE_REMOTE_FIXTURE_TEST_ALLOW_UNGROUPED:-0}" == 1 ]]; then
    local entry_script="${DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT:-}"
    [[ -f "$entry_script" && ! -L "$entry_script" ]] || {
      remote_fixture_fail "Unix fixture worker entry script is missing or unsafe"
      return 1
    }
    DEVE_REMOTE_FIXTURE_START_PARENT_PID="$parent_pid" \
      DEVE_REMOTE_FIXTURE_START_PARENT_TOKEN="$parent_token" \
      bash "$entry_script" __start-worker "$@" &
    REMOTE_FIXTURE_START_WORKER_PID="$!"
    return 0
  fi
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 ]]; then
    DEVE_REMOTE_FIXTURE_START_PARENT_PID="$parent_pid" \
      DEVE_REMOTE_FIXTURE_START_PARENT_TOKEN="$parent_token" \
      start_fixture_worker "$@" &
    REMOTE_FIXTURE_START_WORKER_PID="$!"
    return 0
  fi
  local entry_script="${DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT:-}"
  [[ -f "$entry_script" && ! -L "$entry_script" ]] || {
    remote_fixture_fail "Unix fixture worker entry script is missing or unsafe"
    return 1
  }
  case "$REMOTE_FIXTURE_PLATFORM" in
    Darwin*)
      remote_fixture_fail \
        "macOS Unix fixture startup is unavailable until a stable native process-token adapter is verified"
      return 1
      ;;
    MINGW*|MSYS*|CYGWIN*)
      remote_fixture_fail "Windows fixture startup must use remote-browser-fixture.ps1"
      return 1
      ;;
    *)
      remote_fixture_require_command setsid
      remote_fixture_require_command python3
      [[ -f "$REMOTE_FIXTURE_SIGNAL_EXEC" && ! -L "$REMOTE_FIXTURE_SIGNAL_EXEC" ]] || {
        remote_fixture_fail "Unix fixture worker signal exec adapter is missing or unsafe"
        return 1
      }
      DEVE_REMOTE_FIXTURE_START_PARENT_PID="$parent_pid" \
        DEVE_REMOTE_FIXTURE_START_PARENT_TOKEN="$parent_token" \
        setsid python3 "$REMOTE_FIXTURE_SIGNAL_EXEC" \
          bash "$entry_script" __start-worker "$@" &
      ;;
  esac
  REMOTE_FIXTURE_START_WORKER_PID="$!"
  REMOTE_FIXTURE_START_WORKER_PROCESS_GROUP=1
}

remote_fixture_start_worker_active() {
  local pid="$1"
  local expected_token="$2"
  if ! remote_fixture_pid_active "$pid"; then
    # A live PID with temporarily unreadable /proc identity is not an exited
    # direct child. Returning probe-unavailable prevents an exact wait from
    # blocking forever on a worker that still needs cancellation forwarded.
    remote_fixture_pid_exists "$pid" && return 3
    return 1
  fi
  local actual_token=""
  local platform="$REMOTE_FIXTURE_PLATFORM"
  if [[ "$platform" == MINGW* || "$platform" == MSYS* || "$platform" == CYGWIN* ]]; then
    local process_row
    if ! process_row="$(ps -W 2>/dev/null | awk -v pid="$pid" \
      'NR > 1 && $1 == pid { print $4 ":" $7 ":" $8; exit }')"; then
      return 3
    fi
    # MSYS retains a waitable shell PID after its native process entry has
    # gone; an absent native row is the exited-child proof.
    [[ -n "$process_row" ]] || return 1
    actual_token="$process_row"
  else
    actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null)" || return 3
    [[ -n "$actual_token" ]] || return 3
  fi
  [[ "$actual_token" == "$expected_token" ]] || return 2
}

remote_fixture_signal_start_worker() {
  local pid="$1"
  local expected_token="$2"
  local signal_name="$3"
  local active_status=0
  remote_fixture_start_worker_active "$pid" "$expected_token" || active_status=$?
  if ((active_status == 1)); then return 0; fi
  if ((active_status == 2)); then
    remote_fixture_fail "refusing to signal reused or unowned fixture start worker PID $pid"
    return 1
  fi
  if ((active_status == 3)); then
    remote_fixture_fail "fixture start worker process token probe is unavailable"
    return 3
  fi
  kill -s "$signal_name" -- "$pid"
}

remote_fixture_terminate_waitable_start_worker() {
  local pid="$1"
  local expected_token="${2:-}"
  local process_group="${3:-0}"
  local force_only="${4:-0}"
  local tree_status=0 root_status=0
  remote_fixture_stop_bounded_tree \
    "fixture start worker" "$pid" "$process_group" "$expected_token" "$force_only" 1 \
    || tree_status=$?
  ((tree_status == 0)) || {
    remote_fixture_fail "fixture start worker tree could not be proven reaped"
    return 1
  }
  remote_fixture_root_identity_status "$pid" "$expected_token" || root_status=$?
  ((root_status == 1)) || {
    remote_fixture_fail "fixture start worker survived bounded direct-child termination"
    return 1
  }
  wait "$pid" 2>/dev/null || true
  return 0
}
