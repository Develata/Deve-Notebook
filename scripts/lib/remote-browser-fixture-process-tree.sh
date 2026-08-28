#!/usr/bin/env bash
# shellcheck shell=bash

# Exact process-tree ownership and shared-deadline termination for the Unix
# RemoteBrowser fixture. The base lifecycle module supplies PID/token probes.

REMOTE_FIXTURE_PROCESS_TABLE_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-process-table.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-process-table.sh
source "$REMOTE_FIXTURE_PROCESS_TABLE_LIB"

REMOTE_FIXTURE_DESCENDANT_SNAPSHOT=()
REMOTE_FIXTURE_PROCESS_GROUP_ACTIVE=0
REMOTE_FIXTURE_VERIFIED_PROCESS_GROUPS=()

remote_fixture_process_group_is_bound() {
  local identity="$1|$2"
  remote_fixture_identity_is_tracked "$identity" "${REMOTE_FIXTURE_VERIFIED_PROCESS_GROUPS[@]}"
}

remote_fixture_bind_isolated_process_group() {
  local root_pid="$1"
  local expected_token="$2"
  [[ -n "$expected_token" ]] || {
    remote_fixture_fail "cannot bind an isolated process group without a root token"
    return 1
  }
  local actual_token
  actual_token="$(remote_fixture_process_token "$root_pid" 2>/dev/null || true)"
  [[ "$actual_token" == "$expected_token" ]] || {
    remote_fixture_fail "isolated process-group root token changed before binding"
    return 1
  }
  remote_fixture_assert_isolated_process_group "$root_pid" || return 1
  remote_fixture_process_group_is_bound "$root_pid" "$expected_token" \
    || REMOTE_FIXTURE_VERIFIED_PROCESS_GROUPS+=("$root_pid|$expected_token")
}

remote_fixture_forget_isolated_process_group() {
  local identity="$1|$2"
  local -a retained=()
  local entry
  for entry in "${REMOTE_FIXTURE_VERIFIED_PROCESS_GROUPS[@]}"; do
    [[ "$entry" == "$identity" ]] || retained+=("$entry")
  done
  REMOTE_FIXTURE_VERIFIED_PROCESS_GROUPS=("${retained[@]}")
}
remote_fixture_capture_descendant_snapshot() {
  local label="$1"
  local root_pid="$2"
  REMOTE_FIXTURE_DESCENDANT_SNAPSHOT=()
  local entry descendant_output
  if ! descendant_output="$(remote_fixture_tokenized_descendants_deepest "$root_pid")"; then
    remote_fixture_fail "could not enumerate the $label process tree"
    return 1
  fi
  while read -r entry; do
    [[ -n "$entry" ]] || continue
    [[ "$entry" =~ ^[1-9][0-9]*\|[0-9]+$ ]] || {
      remote_fixture_fail "invalid token-bound $label descendant identity"
      return 1
    }
    REMOTE_FIXTURE_DESCENDANT_SNAPSHOT+=("$entry")
  done <<<"$descendant_output"
}

remote_fixture_capture_process_group_snapshot() {
  local label="$1"
  local group_id="$2"
  local root_pid="$3"
  REMOTE_FIXTURE_DESCENDANT_SNAPSHOT=()
  REMOTE_FIXTURE_PROCESS_GROUP_ACTIVE=0
  local entry member member_output
  if ! member_output="$(remote_fixture_tokenized_process_group_members "$group_id")"; then
    remote_fixture_fail "could not enumerate the $label process group"
    return 1
  fi
  while read -r entry; do
    [[ -n "$entry" ]] || continue
    [[ "$entry" =~ ^[1-9][0-9]*\|[0-9]+$ ]] || {
      remote_fixture_fail "invalid token-bound $label process-group identity"
      return 1
    }
    member="${entry%%|*}"
    REMOTE_FIXTURE_PROCESS_GROUP_ACTIVE=1
    [[ "$member" != "$root_pid" ]] || continue
    REMOTE_FIXTURE_DESCENDANT_SNAPSHOT+=("$entry")
  done <<<"$member_output"
}

remote_fixture_owned_process_group_status() {
  local label="$1"
  local group_id="$2"
  remote_fixture_capture_owned_tree_snapshot "$label" "$group_id" 1 || return 2
  ((REMOTE_FIXTURE_PROCESS_GROUP_ACTIVE == 1)) && return 0
  return 1
}

remote_fixture_capture_owned_tree_snapshot() {
  local label="$1"
  local root_pid="$2"
  local process_group="$3"
  if [[ "$process_group" == "1" ]]; then
    remote_fixture_capture_process_group_snapshot "$label" "$root_pid" "$root_pid" || return 1
    local group_active="$REMOTE_FIXTURE_PROCESS_GROUP_ACTIVE"
    local -a group_snapshot=("${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}")
    remote_fixture_capture_descendant_snapshot "$label" "$root_pid" || return 1
    local entry
    for entry in "${group_snapshot[@]}"; do
      remote_fixture_identity_is_tracked "$entry" "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}" \
        || REMOTE_FIXTURE_DESCENDANT_SNAPSHOT+=("$entry")
    done
    REMOTE_FIXTURE_PROCESS_GROUP_ACTIVE="$group_active"
  else
    remote_fixture_capture_descendant_snapshot "$label" "$root_pid"
  fi
}

remote_fixture_signal_owned_identity() {
  local label="$1"
  local pid="$2"
  local expected_token="$3"
  local signal_name="$4"
  local actual_token
  actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
  if [[ -z "$actual_token" ]]; then
    remote_fixture_pid_exists "$pid" || return 0
    remote_fixture_fail "refusing to signal $label PID $pid without a live token proof"
    return 1
  fi
  [[ "$actual_token" == "$expected_token" ]] || return 0
  if ! remote_fixture_pid_active "$pid"; then
    remote_fixture_pid_terminal "$pid" && return 0
    remote_fixture_pid_exists "$pid" || return 0
    remote_fixture_fail "refusing to signal live unreadable $label PID $pid"
    return 1
  fi
  kill -s "$signal_name" "$pid" 2>/dev/null || true
}

remote_fixture_owned_descendant_status() {
  local entry pid token actual_token
  for entry in "$@"; do
    pid="${entry%%|*}"
    token="${entry#*|}"
    remote_fixture_pid_active "$pid" || continue
    actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
    if [[ -z "$actual_token" ]]; then
      remote_fixture_pid_active "$pid" || continue
      return 2
    fi
    [[ "$actual_token" == "$token" ]] && return 0
  done
  return 1
}

remote_fixture_root_identity_status() {
  local pid="$1"
  local expected_token="${2:-}"
  if [[ -n "$expected_token" ]]; then
    local platform="$REMOTE_FIXTURE_PLATFORM"
    if [[ "$platform" == MINGW* || "$platform" == MSYS* || "$platform" == CYGWIN* ]]; then
      local process_row
      if ! process_row="$(ps -W 2>/dev/null | awk -v pid="$pid" \
        'NR > 1 && $1 == pid { print $4 ":" $7 ":" $8; exit }')"; then
        return 2
      fi
      if [[ -z "$process_row" ]]; then
        remote_fixture_job_active "$pid" && return 0
        return 1
      fi
      [[ "$process_row" == "$expected_token" ]] && return 0
      return 2
    fi
    local actual_token
    actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
    if [[ -z "$actual_token" ]]; then
      remote_fixture_pid_active "$pid" && return 2
      remote_fixture_pid_exists "$pid" && return 2
      return 1
    fi
    [[ "$actual_token" == "$expected_token" ]] || return 2
    remote_fixture_pid_active "$pid" && return 0
    remote_fixture_pid_terminal "$pid" && return 1
    remote_fixture_pid_exists "$pid" && return 2
    return 1
  fi
  remote_fixture_job_active "$pid" && return 0
  remote_fixture_pid_active "$pid" && return 2
  remote_fixture_pid_exists "$pid" && return 2
  return 1
}

remote_fixture_signal_owned_root() {
  local label="$1"
  local pid="$2"
  local expected_token="$3"
  local process_group="$4"
  local signal_name="$5"
  local identity_status=0
  remote_fixture_root_identity_status "$pid" "$expected_token" || identity_status=$?
  if ((identity_status == 2)); then
    remote_fixture_fail "refusing to signal $label root PID $pid without job or token proof"
    return 1
  fi
  if [[ "$process_group" == "1" ]]; then
    if ((identity_status == 0)); then
      remote_fixture_bind_isolated_process_group "$pid" "$expected_token" || return 1
    else
      local group_status=0
      remote_fixture_owned_process_group_status "$label" "$pid" || group_status=$?
      ((group_status != 2)) || {
        remote_fixture_fail "could not re-prove the retained $label process group"
        return 1
      }
      ((group_status == 0)) || return 0
      remote_fixture_process_group_is_bound "$pid" "$expected_token" || {
        remote_fixture_fail "refusing to signal an unbound retained $label process group"
        return 1
      }
      # A user-space PID/token/PGID observation is not a pinned kernel handle.
      # Once the exact leader is gone, never send a negative-PGID signal: the
      # numeric PGID may have ended and been reused between observations.
      return 0
    fi
    kill -s "$signal_name" -- "-$pid" 2>/dev/null || true
  else
    ((identity_status != 1)) || return 0
    kill -s "$signal_name" -- "$pid" 2>/dev/null || true
  fi
}

remote_fixture_stop_bounded_tree() {
  local label="$1"
  local pid="$2"
  local process_group="$3"
  local expected_root_token="${4:-}"
  local force_only="${5:-0}"
  local waitable_root="${6:-0}"
  [[ "$force_only" == 0 || "$force_only" == 1 ]] || {
    remote_fixture_fail "invalid force-only process-tree cleanup mode"
    return 1
  }
  [[ "$waitable_root" == 0 || "$waitable_root" == 1 ]] || {
    remote_fixture_fail "invalid waitable-root process-tree cleanup mode"
    return 1
  }
  local root_status=0
  remote_fixture_root_identity_status "$pid" "$expected_root_token" || root_status=$?
  if ((root_status == 2)); then
    remote_fixture_fail "refusing to stop $label root PID $pid without job or token proof"
    return 1
  fi

  local group_status=1
  if [[ "$process_group" == "1" ]]; then
    if ((root_status == 0)); then
      remote_fixture_bind_isolated_process_group "$pid" "$expected_root_token" || return 1
    fi
    group_status=0
    remote_fixture_owned_process_group_status "$label" "$pid" || group_status=$?
    ((group_status != 2)) || return 1
    if ((root_status == 1)) \
      && ! remote_fixture_process_group_is_bound "$pid" "$expected_root_token"; then
      remote_fixture_fail "refusing to stop an unbound retained $label process group"
      return 1
    fi
    if ((root_status == 1 && group_status == 1)); then
      remote_fixture_forget_isolated_process_group "$pid" "$expected_root_token"
      return 0
    fi
    if ((root_status == 1)); then
      remote_fixture_fail \
        "refusing to stop a nonempty retained $label process group without a live leader or pinned kernel handle"
      return 1
    fi
  elif ((root_status == 1)); then
    return 0
  fi

  local -a descendant_identities=()
  local entry descendant_pid descendant_token descendant_status=1
  remote_fixture_capture_owned_tree_snapshot "$label" "$pid" "$process_group" || return 1
  for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
    remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}" \
      || descendant_identities+=("$entry")
  done

  if ((force_only == 0)); then
    # Signal the waitable root first so cooperative workers stop producing new
    # descendants. Descendants are signalled in batches against the same shared
    # deadline; no child receives an independent multi-second grace window.
    remote_fixture_signal_owned_root "$label" "$pid" "$expected_root_token" "$process_group" TERM \
      || return 1
    for entry in "${descendant_identities[@]}"; do
      descendant_pid="${entry%%|*}"
      descendant_token="${entry#*|}"
      remote_fixture_signal_owned_identity "$label descendant" \
        "$descendant_pid" "$descendant_token" TERM || return 1
    done

    local term_deadline=$((SECONDS + 3))
    while ((SECONDS < term_deadline)); do
      root_status=0
      remote_fixture_root_identity_status "$pid" "$expected_root_token" || root_status=$?
      ((root_status != 2)) || {
        remote_fixture_fail "lost $label root identity proof during bounded cleanup"
        return 1
      }
      if [[ "$process_group" == "1" ]]; then
        group_status=0
        remote_fixture_owned_process_group_status "$label" "$pid" || group_status=$?
        ((group_status != 2)) || return 1
        for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
          if ! remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}"; then
            if ((root_status == 1)); then
              remote_fixture_fail \
                "untracked $label group member appeared after the exact leader exited"
              return 1
            fi
            descendant_identities+=("$entry")
            descendant_pid="${entry%%|*}"
            descendant_token="${entry#*|}"
            remote_fixture_signal_owned_identity "$label descendant" \
              "$descendant_pid" "$descendant_token" TERM || return 1
          fi
        done
      fi
      descendant_status=0
      remote_fixture_owned_descendant_status "${descendant_identities[@]}" \
        || descendant_status=$?
      ((descendant_status != 2)) || {
        remote_fixture_fail "lost $label descendant token proof during bounded cleanup"
        return 1
      }
      if ((root_status == 1 && descendant_status == 1 \
        && (process_group == 0 || group_status == 1))); then
        if ((process_group == 1)); then
          remote_fixture_forget_isolated_process_group "$pid" "$expected_root_token"
        fi
        return 0
      fi
      if ((process_group == 0 && root_status == 0)); then
        remote_fixture_capture_owned_tree_snapshot "$label" "$pid" "$process_group" || return 1
        for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
          if ! remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}"; then
            descendant_identities+=("$entry")
            descendant_pid="${entry%%|*}"
            descendant_token="${entry#*|}"
            remote_fixture_signal_owned_identity "$label descendant" \
              "$descendant_pid" "$descendant_token" TERM || return 1
          fi
        done
      fi
      sleep 0.1
    done
  fi

  if ((process_group == 1)); then
    group_status=0
    remote_fixture_owned_process_group_status "$label" "$pid" || group_status=$?
    ((group_status != 2)) || return 1
    for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
      if ! remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}"; then
        if ((root_status == 1)); then
          remote_fixture_fail \
            "untracked $label group member appeared after the exact leader exited"
          return 1
        fi
        descendant_identities+=("$entry")
      fi
    done
  elif ((root_status == 0)); then
    remote_fixture_capture_owned_tree_snapshot "$label" "$pid" "$process_group" || return 1
    for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
      if ! remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}"; then
        descendant_identities+=("$entry")
      fi
    done
  fi
  # Stop the root under running-job or exact-token proof before the final
  # descendant batch so it cannot continue producing children.
  remote_fixture_signal_owned_root "$label" "$pid" "$expected_root_token" "$process_group" KILL \
    || return 1
  root_status=0
  remote_fixture_root_identity_status "$pid" "$expected_root_token" || root_status=$?
  ((root_status != 2)) || {
    remote_fixture_fail "lost $label root identity proof after force-stop"
    return 1
  }
  if ((process_group == 1)); then
    group_status=0
    remote_fixture_owned_process_group_status "$label" "$pid" || group_status=$?
    ((group_status != 2)) || return 1
    for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
      if ! remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}"; then
        descendant_identities+=("$entry")
      fi
    done
  elif ((root_status == 0)); then
    remote_fixture_capture_owned_tree_snapshot "$label" "$pid" "$process_group" || return 1
    for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
      if ! remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}"; then
        descendant_identities+=("$entry")
      fi
    done
  fi
  for entry in "${descendant_identities[@]}"; do
    descendant_pid="${entry%%|*}"
    descendant_token="${entry#*|}"
    remote_fixture_signal_owned_identity "$label descendant" \
      "$descendant_pid" "$descendant_token" KILL || return 1
  done

  local kill_attempt
  for ((kill_attempt = 0; kill_attempt < 20; kill_attempt += 1)); do
    root_status=0
    remote_fixture_root_identity_status "$pid" "$expected_root_token" || root_status=$?
    ((root_status != 2)) || {
      remote_fixture_fail "lost $label root identity proof after force-stop"
      return 1
    }
    if [[ "$process_group" == "1" ]]; then
      group_status=0
      remote_fixture_owned_process_group_status "$label" "$pid" || group_status=$?
      ((group_status != 2)) || return 1
      for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
        if ! remote_fixture_identity_is_tracked "$entry" "${descendant_identities[@]}"; then
          if ((root_status == 1)); then
            remote_fixture_fail \
              "untracked $label group member appeared after the exact leader exited"
            return 1
          fi
          descendant_identities+=("$entry")
          descendant_pid="${entry%%|*}"
          descendant_token="${entry#*|}"
          remote_fixture_signal_owned_identity "$label descendant" \
            "$descendant_pid" "$descendant_token" KILL || return 1
        fi
      done
    fi
    descendant_status=0
    remote_fixture_owned_descendant_status "${descendant_identities[@]}" \
      || descendant_status=$?
    ((descendant_status != 2)) || {
      remote_fixture_fail "lost $label descendant token proof after force-stop"
      return 1
    }
    if ((root_status == 1 && descendant_status == 1 \
      && (process_group == 0 || group_status == 1))); then
      if ((process_group == 1)); then
        remote_fixture_forget_isolated_process_group "$pid" "$expected_root_token"
      fi
      if ((waitable_root == 1)); then
        wait "$pid" 2>/dev/null || true
      fi
      return 0
    fi
    sleep 0.05
  done
  remote_fixture_fail "$label process tree survived shared cleanup deadline"
}

remote_fixture_stop_owned_job() {
  local label="$1"
  local pid="$2"
  local expected_token="${3:-}"
  [[ "$pid" =~ ^[0-9]+$ ]] || return 0

  if ! remote_fixture_job_active "$pid"; then
    if ! remote_fixture_pid_active "$pid"; then
      wait "$pid" 2>/dev/null || true
      return 0
    fi
    local actual_token
    actual_token="$(remote_fixture_process_token "$pid" || true)"
    if [[ -z "$expected_token" || "$actual_token" != "$expected_token" ]]; then
      remote_fixture_fail "refusing to stop reused or unowned $label PID $pid"
      return 1
    fi
  fi

  remote_fixture_stop_bounded_tree "$label" "$pid" 0 "$expected_token" || return 1
  wait "$pid" 2>/dev/null || true
  if remote_fixture_pid_active "$pid" || remote_fixture_job_active "$pid"; then
    remote_fixture_fail "$label job survived verified cleanup"
    return 1
  fi
}
