#!/usr/bin/env bash
# shellcheck shell=bash

# Root-last cleanup for the Linux bounded child-subreaper. Generic fixture
# workers remain root-first; this stable root exists only to retain orphans.

REMOTE_FIXTURE_BOUNDED_LAUNCHER_PID=""
REMOTE_FIXTURE_BOUNDED_LAUNCHER_TOKEN=""

remote_fixture_wait_bounded_launcher_identity() {
  local label="$1"
  local root_pid="$2"
  local identity_path="$3"
  local attempt
  for ((attempt = 0; attempt < 100; attempt += 1)); do
    if [[ -e "$identity_path" || -L "$identity_path" ]]; then break; fi
    remote_fixture_pid_active "$root_pid" || break
    sleep 0.01
  done
  [[ -f "$identity_path" && ! -L "$identity_path" ]] || {
    remote_fixture_fail "$label launcher identity was not published safely"
    return 1
  }
  local -a lines=()
  mapfile -t lines <"$identity_path"
  ((${#lines[@]} == 1)) || {
    remote_fixture_fail "$label launcher identity must contain exactly one line"
    return 1
  }
  [[ "${lines[0]}" =~ ^([0-9]+)\|([^|[:space:]]+)$ ]] || {
    remote_fixture_fail "$label launcher identity is invalid"
    return 1
  }
  local launcher_pid="${BASH_REMATCH[1]}"
  local launcher_token="${BASH_REMATCH[2]}"
  local actual_token table observed_parent
  actual_token="$(remote_fixture_process_token "$launcher_pid" 2>/dev/null || true)"
  [[ "$actual_token" == "$launcher_token" ]] || {
    remote_fixture_fail "$label launcher token changed before admission"
    return 1
  }
  table="$(remote_fixture_process_table)" || {
    remote_fixture_fail "$label launcher parent relation could not be inspected"
    return 1
  }
  observed_parent="$(awk -v pid="$launcher_pid" \
    '$1 == pid { print $2; count++ } END { if (count != 1) exit 1 }' <<<"$table")" || {
    remote_fixture_fail "$label launcher parent relation is missing or ambiguous"
    return 1
  }
  [[ "$observed_parent" == "$root_pid" ]] || {
    remote_fixture_fail "$label launcher is not a direct subreaper child"
    return 1
  }
  REMOTE_FIXTURE_BOUNDED_LAUNCHER_PID="$launcher_pid"
  REMOTE_FIXTURE_BOUNDED_LAUNCHER_TOKEN="$launcher_token"
}

remote_fixture_capture_bounded_payload_descendants() {
  local label="$1"
  local root_pid="$2"
  local launcher_identity="$3"
  remote_fixture_capture_owned_tree_snapshot "$label" "$root_pid" 1 || return 1
  local -a payload_descendants=()
  local entry
  for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
    [[ "$entry" == "$launcher_identity" ]] || payload_descendants+=("$entry")
  done
  REMOTE_FIXTURE_DESCENDANT_SNAPSHOT=("${payload_descendants[@]}")
}

remote_fixture_signal_new_subreaper_descendants() {
  local label="$1"
  local signal_name="$2"
  local tracked_name="$3"
  local -n tracked="$tracked_name"
  local entry descendant_pid descendant_token
  for entry in "${REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]}"; do
    if remote_fixture_identity_is_tracked "$entry" "${tracked[@]}"; then
      [[ "$signal_name" == KILL ]] || continue
    else
      tracked+=("$entry")
    fi
    descendant_pid="${entry%%|*}"
    descendant_token="${entry#*|}"
    remote_fixture_signal_owned_identity \
      "$label descendant" "$descendant_pid" "$descendant_token" "$signal_name" \
      || return 1
  done
}

remote_fixture_subreaper_descendants_status() {
  local label="$1"
  local root_pid="$2"
  local signal_name="$3"
  local tracked_name="$4"
  local -n tracked="$tracked_name"
  remote_fixture_capture_owned_tree_snapshot "$label" "$root_pid" 1 || return 2
  remote_fixture_signal_new_subreaper_descendants \
    "$label" "$signal_name" "$tracked_name" || return 2
  local descendant_status=0
  remote_fixture_owned_descendant_status "${tracked[@]}" || descendant_status=$?
  ((descendant_status != 2)) || return 2
  if ((descendant_status == 1 && ${#REMOTE_FIXTURE_DESCENDANT_SNAPSHOT[@]} == 0)); then
    return 1
  fi
  return 0
}

remote_fixture_stop_bounded_subreaper_tree() {
  local label="$1"
  local root_pid="$2"
  local expected_token="$3"
  local waitable_root="${4:-0}"
  [[ "$waitable_root" == 0 || "$waitable_root" == 1 ]] || {
    remote_fixture_fail "invalid subreaper waitable-root mode"
    return 1
  }
  remote_fixture_live_pid_matches_token "$root_pid" "$expected_token" || {
    remote_fixture_fail "refusing root-last cleanup without a live bounded subreaper token"
    return 1
  }
  remote_fixture_bind_isolated_process_group "$root_pid" "$expected_token" || return 1

  local -a tracked_descendants=()
  local status=0 attempt
  remote_fixture_subreaper_descendants_status \
    "$label" "$root_pid" TERM tracked_descendants || status=$?
  ((status != 2)) || return 1
  if ((status == 0)); then
    local term_deadline=$((SECONDS + 3))
    while ((SECONDS < term_deadline)); do
      status=0
      remote_fixture_subreaper_descendants_status \
        "$label" "$root_pid" TERM tracked_descendants || status=$?
      ((status != 2)) || return 1
      ((status != 1)) || break
      sleep 0.1
    done
  fi

  status=0
  remote_fixture_subreaper_descendants_status \
    "$label" "$root_pid" KILL tracked_descendants || status=$?
  ((status != 2)) || return 1
  for ((attempt = 0; attempt < 40 && status != 1; attempt += 1)); do
    sleep 0.05
    status=0
    remote_fixture_subreaper_descendants_status \
      "$label" "$root_pid" KILL tracked_descendants || status=$?
    ((status != 2)) || return 1
  done
  ((status == 1)) || {
    remote_fixture_fail "$label descendant tree survived root-last cleanup deadline"
    return 1
  }

  remote_fixture_signal_owned_identity \
    "$label subreaper" "$root_pid" "$expected_token" KILL || return 1
  for ((attempt = 0; attempt < 40; attempt += 1)); do
    local root_status=0
    remote_fixture_root_identity_status "$root_pid" "$expected_token" || root_status=$?
    ((root_status != 1)) || break
    ((root_status == 0)) || {
      remote_fixture_fail "$label subreaper token changed before root-last reap"
      return 1
    }
    sleep 0.05
  done
  remote_fixture_pid_active "$root_pid" && {
    remote_fixture_fail "$label subreaper survived root-last cleanup"
    return 1
  }
  if ((waitable_root == 1)); then wait "$root_pid" 2>/dev/null || true; fi
  remote_fixture_stop_bounded_tree \
    "$label post-subreaper proof" "$root_pid" 1 "$expected_token" 1 0
}

remote_fixture_request_subreaper_self_cleanup() {
  local label="$1"
  local root_pid="$2"
  local expected_token="$3"
  remote_fixture_signal_owned_identity \
    "$label subreaper fallback" "$root_pid" "$expected_token" USR2 || return 1
  local attempt
  for ((attempt = 0; attempt < 100; attempt += 1)); do
    local root_status=0
    remote_fixture_root_identity_status "$root_pid" "$expected_token" || root_status=$?
    if ((root_status == 1)); then
      wait "$root_pid" 2>/dev/null || true
      remote_fixture_stop_bounded_tree \
        "$label post-self-cleanup proof" "$root_pid" 1 "$expected_token" 1 0
      return $?
    fi
    ((root_status == 0)) || {
      remote_fixture_fail "$label subreaper token changed during self-cleanup fallback"
      return 1
    }
    sleep 0.05
  done
  remote_fixture_fail "$label subreaper self-cleanup fallback exceeded its deadline"
}
