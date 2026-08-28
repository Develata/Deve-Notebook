#!/usr/bin/env bash
# shellcheck shell=bash

# Private control-file, descriptor, signal-latch, and output normalization
# helpers for the bounded RemoteBrowser payload runner.

remote_fixture_limit_output_files() {
  local combined_limit_bytes="$1"
  shift
  remote_fixture_require_command truncate
  local per_file_limit=$((combined_limit_bytes / $#))
  local path
  for path in "$@"; do
    [[ -f "$path" ]] || continue
    if (($(wc -c <"$path") > per_file_limit)); then
      truncate -s "$per_file_limit" -- "$path"
    fi
  done
}

remote_fixture_latch_bounded_signal() {
  local status="$1"
  if [[ "$status" == 143 || "${bounded_signal_status:-0}" == 0 ]]; then
    bounded_signal_status="$status"
  fi
}

remote_fixture_restore_bounded_traps() {
  local saved_int="$1"
  local saved_term="$2"
  if [[ -n "$saved_int" ]]; then eval "$saved_int"; else trap - INT; fi
  if [[ -n "$saved_term" ]]; then eval "$saved_term"; else trap - TERM; fi
}

remote_fixture_close_bounded_fd() {
  local fd="${1:-}"
  [[ "$fd" =~ ^[0-9]+$ ]] || return 0
  eval "exec ${fd}>&-"
}

remote_fixture_remove_bounded_controls() {
  rm -f -- "$@"
}

remote_fixture_abort_unadmitted_subreaper() {
  local label="$1"
  local pid="$2"
  local attempt members
  if remote_fixture_job_active "$pid"; then
    kill -USR2 -- "$pid" 2>/dev/null || {
      remote_fixture_fail "$label could not request subreaper self-cleanup"
      return 1
    }
  elif remote_fixture_pid_active "$pid"; then
    remote_fixture_fail "$label is live without retained-child ownership"
    return 1
  fi
  for ((attempt = 0; attempt < 100; attempt += 1)); do
    if ! remote_fixture_pid_active "$pid" && ! remote_fixture_job_active "$pid"; then
      wait "$pid" 2>/dev/null || true
      if ! members="$(remote_fixture_process_group_members "$pid" 2>/dev/null)"; then
        remote_fixture_fail "$label pre-admission process-group probe failed"
        return 1
      fi
      if [[ -n "$members" ]]; then
        remote_fixture_fail "$label left a retained pre-admission process group"
        return 1
      fi
      return 0
    fi
    sleep 0.05
  done
  remote_fixture_fail "$label survived pre-admission subreaper self-cleanup"
}

remote_fixture_read_bounded_status() {
  local label="$1"
  local status_path="$2"
  [[ -f "$status_path" && ! -L "$status_path" ]] || {
    remote_fixture_fail "$label payload status is unsafe"
    return 1
  }
  local -a lines=()
  mapfile -t lines <"$status_path"
  ((${#lines[@]} == 1)) || {
    remote_fixture_fail "$label payload status must contain exactly one line"
    return 1
  }
  [[ "${lines[0]}" =~ ^([0-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-5])$ ]] || {
    remote_fixture_fail "$label payload status is invalid"
    return 1
  }
  REMOTE_FIXTURE_BOUNDED_PAYLOAD_STATUS="${lines[0]}"
}

remote_fixture_report_bounded_failure() {
  local label="$1"
  local failure="$2"
  local cleanup_status="$3"
  remote_fixture_fail "$label $failure" || true
  if ((cleanup_status != 0)); then
    remote_fixture_fail "$label cleanup failed with status $cleanup_status" || true
  fi
}
