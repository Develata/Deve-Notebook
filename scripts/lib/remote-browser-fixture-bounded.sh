#!/usr/bin/env bash
# shellcheck shell=bash
# Bounded child execution for the Unix RemoteBrowser fixture. On Linux the
# child-subreaper remains the live process-group leader while its Bash control
# launcher publishes payload status and waits for explicit parent release.
REMOTE_FIXTURE_BOUNDED_PAYLOAD_STATUS=""
REMOTE_FIXTURE_BOUNDED_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REMOTE_FIXTURE_BOUNDED_SUBREAPER="$REMOTE_FIXTURE_BOUNDED_LIB_DIR/remote-browser-fixture-subreaper.py"
REMOTE_FIXTURE_SUBREAPER_TREE_LIB="$REMOTE_FIXTURE_BOUNDED_LIB_DIR/remote-browser-fixture-subreaper-tree.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-subreaper-tree.sh
source "$REMOTE_FIXTURE_SUBREAPER_TREE_LIB"
REMOTE_FIXTURE_BOUNDED_CONTROL_LIB="$REMOTE_FIXTURE_BOUNDED_LIB_DIR/remote-browser-fixture-bounded-control.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-bounded-control.sh
source "$REMOTE_FIXTURE_BOUNDED_CONTROL_LIB"

remote_fixture_cleanup_bounded_failure() {
  local label="$1"
  local failure="$2"
  local pid="$3"
  local process_group="$4"
  local process_token="$5"
  local output_limit_bytes="$6"
  local stdout_path="$7"
  local stderr_path="$8"
  local admission_path="$9"
  local status_path="${10}"
  local status_tmp_path="${11}"
  local release_path="${12}"
  local release_fd="${13}"
  local completion_path="${14}"
  local launcher_failure_path="${15}"
  local launcher_identity_path="${16}"
  local cleanup_status=0

  if [[ "$failure" == exceeded* ]]; then
    remote_fixture_limit_output_files "$output_limit_bytes" "$stdout_path" "$stderr_path" \
      || cleanup_status=$?
  fi
  if ((process_group == 1)); then
    remote_fixture_stop_bounded_subreaper_tree \
      "$label" "$pid" "$process_token" 1 || cleanup_status=$?
  else
    remote_fixture_stop_bounded_tree \
      "$label" "$pid" 0 "$process_token" 0 1 || cleanup_status=$?
  fi
  if ((cleanup_status != 0 && process_group == 1)); then
    if remote_fixture_pid_active "$pid" || remote_fixture_job_active "$pid"; then
      local fallback_status=0
      remote_fixture_request_subreaper_self_cleanup \
        "$label" "$pid" "$process_token" || fallback_status=$?
      if ((fallback_status != 0)); then
        remote_fixture_fail \
          "$label subreaper self-cleanup fallback failed with status $fallback_status" \
          || true
      fi
    fi
  fi
  local root_retained=0
  if remote_fixture_pid_active "$pid" || remote_fixture_job_active "$pid"; then
    root_retained=1
    if ((cleanup_status == 0)); then
      cleanup_status=1
      remote_fixture_fail "$label cleanup reported success while the bounded leader remained live" \
        || true
    fi
  else
    wait "$pid" 2>/dev/null || true
  fi
  remote_fixture_close_bounded_fd "$release_fd" || cleanup_status=$?
  if ((root_retained == 0)); then
    remote_fixture_remove_bounded_controls \
      "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
      "$completion_path" "$launcher_failure_path" "${launcher_failure_path}.${pid}.tmp" \
      "$launcher_identity_path" "${launcher_identity_path}.${pid}.tmp" \
      || cleanup_status=$?
  else
    remote_fixture_fail \
      "$label preserved live bounded leader and private recovery controls after cleanup failure" \
      || true
  fi
  if [[ "$failure" == exceeded* ]]; then
    remote_fixture_limit_output_files "$output_limit_bytes" "$stdout_path" "$stderr_path" \
      || cleanup_status=$?
  fi
  remote_fixture_report_bounded_failure "$label" "$failure" "$cleanup_status"
  if ((bounded_signal_status != 0)); then return "$bounded_signal_status"; fi
  return 1
}

remote_fixture_run_bounded_active() {
  local label="$1"
  local timeout_seconds="$2"
  local output_limit_bytes="$3"
  local stdout_path="$4"
  local stderr_path="$5"
  shift 5
  [[ "${1:-}" == "--" ]] || {
    remote_fixture_fail "bounded process command separator is missing"
    return 1
  }
  shift
  [[ "$timeout_seconds" =~ ^[0-9]+$ && "$timeout_seconds" -gt 0 ]] || {
    remote_fixture_fail "$label timeout must be a positive integer"
    return 1
  }
  [[ "$output_limit_bytes" =~ ^[0-9]+$ && "$output_limit_bytes" -ge 2048 ]] || {
    remote_fixture_fail "$label output limit must be at least 2048 bytes"
    return 1
  }
  (($# > 0)) || {
    remote_fixture_fail "$label command is empty"
    return 1
  }
  ((bounded_signal_status == 0)) || return "$bounded_signal_status"

  if ! rm -f -- "$stdout_path" "$stderr_path" \
    || ! : >"$stdout_path" \
    || ! : >"$stderr_path" \
    || ! chmod 0600 "$stdout_path" "$stderr_path"; then
    remote_fixture_fail "$label could not initialize bounded output files"
    return 1
  fi

  local process_group=0
  local use_subreaper=0
  local platform="$REMOTE_FIXTURE_PLATFORM"
  case "$platform" in
    Linux*)
      remote_fixture_require_command setsid || return 1
      use_subreaper=1
      ;;
    MINGW*|MSYS*|CYGWIN*)
      if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" != 1 \
        || "${DEVE_REMOTE_FIXTURE_TEST_ALLOW_UNGROUPED_BOUNDED:-0}" != 1 ]]; then
        remote_fixture_fail \
          "$label Windows bounded fixture must use the PowerShell implementation"
        return 1
      fi
      ;;
    Darwin*)
      remote_fixture_fail \
        "$label macOS bounded fixture requires a verified native process-token adapter"
      return 1
      ;;
    *)
      remote_fixture_fail "$label bounded fixture is unsupported on host $platform"
      return 1
      ;;
  esac
  local output_file_blocks=$((output_limit_bytes / 2048))
  local per_file_hard_limit_bytes=$((output_file_blocks * 1024))
  local control_base="${stdout_path}.bounded.${BASHPID}.${RANDOM}.${RANDOM}"
  local admission_path="${control_base}.admission"
  local status_path="${control_base}.status"
  local status_tmp_path="${control_base}.status.tmp"
  local release_path="${control_base}.release.fifo"
  local completion_path="${control_base}.released"
  local launcher_failure_path="${control_base}.launcher-failed"
  local launcher_identity_path="${control_base}.launcher-identity"
  local release_fd=""
  local bounded_parent_pid="$BASHPID"
  local path
  for path in "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
    "$completion_path" "$launcher_failure_path" "$launcher_identity_path"; do
    [[ ! -e "$path" && ! -L "$path" ]] || {
      remote_fixture_fail "$label bounded-process control path already exists"
      return 1
    }
  done

  if ((use_subreaper == 1)); then
    if [[ ! -f "$REMOTE_FIXTURE_BOUNDED_SUBREAPER" \
      || -L "$REMOTE_FIXTURE_BOUNDED_SUBREAPER" ]]; then
      remote_fixture_fail "$label bounded subreaper helper is missing or unsafe"
      return 1
    fi
    if ! remote_fixture_require_command mkfifo \
      || ! remote_fixture_require_command python3 \
      || ! mkfifo -- "$release_path" \
      || ! chmod 0600 "$release_path"; then
      remote_fixture_remove_bounded_controls \
        "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
        "$completion_path" "$launcher_failure_path" "$launcher_identity_path" || true
      remote_fixture_fail "$label could not initialize bounded-process controls"
      return 1
    fi
    if ! exec {release_fd}<>"$release_path"; then
      remote_fixture_remove_bounded_controls \
        "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
        "$completion_path" "$launcher_failure_path" "$launcher_identity_path" || true
      remote_fixture_fail "$label could not open the bounded-process release channel"
      return 1
    fi
    setsid python3 "$REMOTE_FIXTURE_BOUNDED_SUBREAPER" \
      "$completion_path" "$launcher_failure_path" "$launcher_identity_path" \
      "$bounded_parent_pid" -- bash -c '
      parent_release_fd="$1"
      admission_path="$2"
      status_path="$3"
      status_tmp_path="$4"
      release_path="$5"
      output_file_blocks="$6"
      completion_path="$7"
      shift 7
      [[ "$parent_release_fd" =~ ^[0-9]+$ ]] || exit 125
      eval "exec ${parent_release_fd}>&-"
      [[ "$output_file_blocks" =~ ^[1-9][0-9]*$ ]] || exit 125
      trap ":" INT TERM
      while [[ ! -f "$admission_path" ]]; do sleep 0.01; done
      ulimit -f "$output_file_blocks" || exit 125
      export DEVE_REMOTE_FIXTURE_BOUNDED_ROOT_PID="$PPID"
      "$@" &
      payload_pid="$!"
      payload_status=0
      wait "$payload_pid" || payload_status=$?
      (umask 077; printf "%s\n" "$payload_status" >"$status_tmp_path")
      chmod 0600 "$status_tmp_path"
      mv -f -- "$status_tmp_path" "$status_path"
      while :; do
        release=""
        if IFS= read -r release <"$release_path"; then
          [[ "$release" == release ]] || exit 125
          break
        fi
        [[ -p "$release_path" ]] || exit 125
      done
      (umask 077; set -o noclobber; printf "released\n" >"$completion_path") || exit 125
      exit "$payload_status"
    ' remote-fixture-bounded "$release_fd" "$admission_path" "$status_path" \
      "$status_tmp_path" "$release_path" "$output_file_blocks" "$completion_path" "$@" \
      >"$stdout_path" 2>"$stderr_path" &
    process_group=1
  else
    (ulimit -f "$output_file_blocks" && exec "$@") \
      >"$stdout_path" 2>"$stderr_path" &
  fi
  local pid="$!"
  local process_token=""
  if ((process_group == 1)); then
    if ! process_token="$(remote_fixture_wait_stable_process_token "$label bounded subreaper" "$pid")"; then
      local abort_status=0
      remote_fixture_abort_unadmitted_subreaper "$label bounded subreaper" "$pid" || abort_status=$?
      remote_fixture_close_bounded_fd "$release_fd" || abort_status=$?
      if ((abort_status == 0)); then
        remote_fixture_remove_bounded_controls \
          "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
          "$completion_path" "$launcher_failure_path" "${launcher_failure_path}.${pid}.tmp" \
          "$launcher_identity_path" "${launcher_identity_path}.${pid}.tmp" \
          || abort_status=$?
      else
        remote_fixture_fail \
          "$label bounded subreaper preserved private controls after pre-admission cleanup failure" \
          || true
      fi
      remote_fixture_report_bounded_failure \
        "$label" "exited before group identity binding" "$abort_status"
      return 1
    fi
    if ! remote_fixture_bind_isolated_process_group "$pid" "$process_token"; then
      local bind_cleanup_status=0
      remote_fixture_abort_unadmitted_subreaper "$label bounded subreaper" "$pid" \
        || bind_cleanup_status=$?
      remote_fixture_close_bounded_fd "$release_fd" || bind_cleanup_status=$?
      if ((bind_cleanup_status == 0)); then
        remote_fixture_remove_bounded_controls \
          "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
          "$completion_path" "$launcher_failure_path" "${launcher_failure_path}.${pid}.tmp" \
          "$launcher_identity_path" "${launcher_identity_path}.${pid}.tmp" \
          || bind_cleanup_status=$?
      else
        remote_fixture_fail \
          "$label bounded subreaper preserved private controls after group-binding cleanup failure" \
          || true
      fi
      remote_fixture_report_bounded_failure \
        "$label" "could not bind bounded-process group identity" "$bind_cleanup_status"
      return 1
    fi
    if ! remote_fixture_wait_bounded_launcher_identity \
      "$label" "$pid" "$launcher_identity_path"; then
      remote_fixture_cleanup_bounded_failure "$label" \
        "could not bind bounded launcher child identity" "$pid" 1 "$process_token" \
        "$output_limit_bytes" "$stdout_path" "$stderr_path" "$admission_path" \
        "$status_path" "$status_tmp_path" "$release_path" "$release_fd" \
        "$completion_path" "$launcher_failure_path" "$launcher_identity_path"
      return $?
    fi
    if ((bounded_signal_status != 0)); then
      remote_fixture_cleanup_bounded_failure "$label" \
        "was cancelled by signal status $bounded_signal_status" "$pid" 1 "$process_token" \
        "$output_limit_bytes" "$stdout_path" "$stderr_path" "$admission_path" \
        "$status_path" "$status_tmp_path" "$release_path" "$release_fd" \
        "$completion_path" "$launcher_failure_path" \
        "$launcher_identity_path"
      return $?
    fi
    if ! (umask 077; set -o noclobber; : >"$admission_path"); then
      remote_fixture_cleanup_bounded_failure "$label" \
        "could not publish bounded-process admission" "$pid" 1 "$process_token" \
        "$output_limit_bytes" "$stdout_path" "$stderr_path" "$admission_path" \
        "$status_path" "$status_tmp_path" "$release_path" "$release_fd" \
        "$completion_path" "$launcher_failure_path" \
        "$launcher_identity_path"
      return $?
    fi
  fi

  local started_at="$SECONDS"
  local failure=""
  local output_bytes stdout_bytes stderr_bytes payload_complete=0
  while :; do
    stdout_bytes="$(wc -c <"$stdout_path")"
    stderr_bytes="$(wc -c <"$stderr_path")"
    output_bytes=$((stdout_bytes + stderr_bytes))
    if ((stdout_bytes >= per_file_hard_limit_bytes \
      || stderr_bytes >= per_file_hard_limit_bytes \
      || output_bytes > output_limit_bytes)); then
      failure="exceeded the combined output limit of $output_limit_bytes bytes"
      break
    fi
    if ((bounded_signal_status != 0)); then
      failure="was cancelled by signal status $bounded_signal_status"
      break
    fi
    if ((SECONDS - started_at >= timeout_seconds)); then
      failure="timed out after $timeout_seconds seconds"
      break
    fi
    if ((process_group == 1)) \
      && [[ -e "$launcher_failure_path" || -L "$launcher_failure_path" ]]; then
      if remote_fixture_read_bounded_status "$label launcher" "$launcher_failure_path"; then
        failure="launcher child exited before completion with status $REMOTE_FIXTURE_BOUNDED_PAYLOAD_STATUS"
      else
        failure="published an invalid launcher-failure status"
      fi
      break
    fi
    if ((process_group == 1)) && [[ -e "$status_path" || -L "$status_path" ]]; then
      if remote_fixture_read_bounded_status "$label" "$status_path"; then
        payload_complete=1
      else
        failure="published an invalid payload status"
      fi
      break
    fi
    if ! remote_fixture_pid_active "$pid" && ! remote_fixture_job_active "$pid"; then
      if ((process_group == 1)); then
        failure="exited before publishing payload status"
      else
        payload_complete=1
      fi
      break
    fi
    sleep 0.05
  done

  if [[ -z "$failure" && "$process_group" == 1 && "$payload_complete" == 1 ]]; then
    remote_fixture_capture_bounded_payload_descendants \
      "$label" "$pid" \
      "$REMOTE_FIXTURE_BOUNDED_LAUNCHER_PID|$REMOTE_FIXTURE_BOUNDED_LAUNCHER_TOKEN" \
      || failure="could not prove bounded payload group membership"
    if [[ -z "$failure" && ${#REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]} -ne 0 ]]; then
      failure="retained descendants after payload exit"
    fi
    if [[ -z "$failure" && "$bounded_signal_status" != 0 ]]; then
      failure="was cancelled by signal status $bounded_signal_status"
    fi
  fi

  if [[ -n "$failure" ]]; then
    remote_fixture_cleanup_bounded_failure "$label" "$failure" "$pid" \
      "$process_group" "$process_token" "$output_limit_bytes" "$stdout_path" \
      "$stderr_path" "$admission_path" "$status_path" "$status_tmp_path" \
      "$release_path" "$release_fd" "$completion_path" "$launcher_failure_path" \
      "$launcher_identity_path"
    return $?
  fi

  local status=0
  if ((process_group == 1)); then
    if ! remote_fixture_live_pid_matches_token "$pid" "$process_token"; then
      remote_fixture_cleanup_bounded_failure "$label" \
        "lost bounded subreaper identity before explicit release" "$pid" 1 "$process_token" \
        "$output_limit_bytes" "$stdout_path" "$stderr_path" "$admission_path" \
        "$status_path" "$status_tmp_path" "$release_path" "$release_fd" \
        "$completion_path" "$launcher_failure_path" \
        "$launcher_identity_path"
      return $?
    fi
    if ! printf 'release\n' >&"$release_fd"; then
      remote_fixture_cleanup_bounded_failure "$label" \
        "could not publish bounded-process release" "$pid" 1 "$process_token" \
        "$output_limit_bytes" "$stdout_path" "$stderr_path" "$admission_path" \
        "$status_path" "$status_tmp_path" "$release_path" "$release_fd" \
        "$completion_path" "$launcher_failure_path" \
        "$launcher_identity_path"
      return $?
    fi
    if ! remote_fixture_close_bounded_fd "$release_fd"; then
      remote_fixture_cleanup_bounded_failure "$label" \
        "could not close bounded-process release ownership" "$pid" 1 "$process_token" \
        "$output_limit_bytes" "$stdout_path" "$stderr_path" "$admission_path" \
        "$status_path" "$status_tmp_path" "$release_path" "$release_fd" \
        "$completion_path" "$launcher_failure_path" \
        "$launcher_identity_path"
      return $?
    fi
    release_fd=""
    local release_failure=""
    while remote_fixture_pid_active "$pid" || remote_fixture_job_active "$pid"; do
      if [[ -e "$launcher_failure_path" || -L "$launcher_failure_path" ]]; then
        if remote_fixture_read_bounded_status "$label launcher" "$launcher_failure_path"; then
          release_failure="launcher child failed after release with status $REMOTE_FIXTURE_BOUNDED_PAYLOAD_STATUS"
        else
          release_failure="published an invalid post-release launcher status"
        fi
        break
      fi
      if ((bounded_signal_status != 0)); then
        release_failure="was cancelled by signal status $bounded_signal_status after release"
        break
      fi
      if ((SECONDS - started_at >= timeout_seconds)); then
        release_failure="did not complete release before the $timeout_seconds second deadline"
        break
      fi
      sleep 0.05
    done
    if [[ -n "$release_failure" ]]; then
      remote_fixture_cleanup_bounded_failure "$label" "$release_failure" "$pid" 1 \
        "$process_token" "$output_limit_bytes" "$stdout_path" "$stderr_path" \
        "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
        "$release_fd" "$completion_path" "$launcher_failure_path" \
        "$launcher_identity_path"
      return $?
    fi
    if wait "$pid"; then status=0; else status=$?; fi
    if ((bounded_signal_status != 0)) && remote_fixture_pid_active "$pid"; then
      remote_fixture_cleanup_bounded_failure "$label" \
        "was cancelled by signal status $bounded_signal_status" "$pid" 1 "$process_token" \
        "$output_limit_bytes" "$stdout_path" "$stderr_path" "$admission_path" \
        "$status_path" "$status_tmp_path" "$release_path" "$release_fd" \
        "$completion_path" "$launcher_failure_path" \
        "$launcher_identity_path"
      return $?
    fi
    local release_cleanup_status=0
    remote_fixture_remove_bounded_controls \
      "$admission_path" "$status_path" "$status_tmp_path" "$release_path" \
      "$completion_path" "$launcher_failure_path" "${launcher_failure_path}.${pid}.tmp" \
      "$launcher_identity_path" "${launcher_identity_path}.${pid}.tmp" \
      || release_cleanup_status=$?
    local group_cleanup_status=0
    remote_fixture_stop_bounded_tree "$label" "$pid" 1 "$process_token" \
      || group_cleanup_status=$?
    if ((group_cleanup_status != 0)); then
      remote_fixture_report_bounded_failure \
        "$label" "did not leave an empty process group after release" "$group_cleanup_status"
      return 1
    fi
    if ((release_cleanup_status != 0)); then
      remote_fixture_report_bounded_failure \
        "$label" "could not remove bounded-process controls after release" \
        "$release_cleanup_status"
      return 1
    fi
    if ((status != REMOTE_FIXTURE_BOUNDED_PAYLOAD_STATUS)); then
      remote_fixture_fail "$label launcher status did not match the published payload status"
      return 1
    fi
  else
    if wait "$pid"; then status=0; else status=$?; fi
  fi
  output_bytes=$(( $(wc -c <"$stdout_path") + $(wc -c <"$stderr_path") ))
  if ((output_bytes > output_limit_bytes)); then
    remote_fixture_limit_output_files "$output_limit_bytes" "$stdout_path" "$stderr_path"
    remote_fixture_fail "$label exceeded the combined output limit of $output_limit_bytes bytes"
    return 1
  fi
  if ((bounded_signal_status != 0)); then return "$bounded_signal_status"; fi
  return "$status"
}

remote_fixture_run_bounded() {
  local saved_int saved_term
  saved_int="$(trap -p INT)"
  saved_term="$(trap -p TERM)"
  local bounded_signal_status=0
  trap 'remote_fixture_latch_bounded_signal 130' INT
  trap 'remote_fixture_latch_bounded_signal 143' TERM
  local status=0
  remote_fixture_run_bounded_active "$@" || status=$?
  remote_fixture_restore_bounded_traps "$saved_int" "$saved_term"
  if ((bounded_signal_status != 0)); then return "$bounded_signal_status"; fi
  return "$status"
}
