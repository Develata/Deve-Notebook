#!/usr/bin/env bash
# shellcheck shell=bash

# Status-propagating process-table and PGID inspection for the Unix fixture.
# This module is read-only and never owns signal or cleanup policy.

remote_fixture_msys_descendants() {
  remote_fixture_descendants_from_process_table "$1"
}

remote_fixture_posix_descendants() {
  remote_fixture_descendants_from_process_table "$1"
}

remote_fixture_process_table() {
  local table
  case "$REMOTE_FIXTURE_PLATFORM" in
    MINGW*|MSYS*|CYGWIN*)
      table="$(ps -W 2>/dev/null)" || return 1
      awk 'NR > 1 { print $1, $2, $3 }' <<<"$table"
      ;;
    *)
      command -v ps >/dev/null 2>&1 || return 1
      ps -eo pid=,ppid=,pgid= 2>/dev/null
      ;;
  esac
}

remote_fixture_descendants_from_process_table() {
  local parent_pid="$1"
  local table
  table="$(remote_fixture_process_table)" || return 1
  node -e '
const fs = require("fs");
const root = process.argv[1];
const children = new Map();
for (const line of fs.readFileSync(0, "utf8").split(/\r?\n/)) {
  const [pid, ppid] = line.trim().split(/\s+/);
  if (!pid || !ppid) continue;
  if (!children.has(ppid)) children.set(ppid, []);
  children.get(ppid).push(pid);
}
const visiting = new Set();
function emit(parent) {
  if (visiting.has(parent)) process.exit(2);
  visiting.add(parent);
  for (const child of children.get(parent) || []) { emit(child); process.stdout.write(`${child}\n`); }
  visiting.delete(parent);
}
emit(root);
' "$parent_pid" <<<"$table"
}

remote_fixture_process_group_members() {
  local group_id="$1"
  local table
  table="$(remote_fixture_process_table)" || return 1
  awk -v group_id="$group_id" '$3 == group_id { print $1 }' <<<"$table"
}

remote_fixture_tokenized_process_table() {
  [[ "$REMOTE_FIXTURE_PLATFORM" == Linux* ]] || return 1
  local process_path pid process_stat process_tail state ppid pgid token
  local -a process_fields
  for process_path in /proc/[0-9]*/stat; do
    [[ -e "$process_path" ]] || continue
    pid="${process_path#/proc/}"
    pid="${pid%/stat}"
    if ! remote_fixture_read_linux_process_stat "$pid"; then
      remote_fixture_pid_exists "$pid" && return 1
      continue
    fi
    process_stat="$REMOTE_FIXTURE_LINUX_PROCESS_STAT"
    [[ "${process_stat%% *}" == "$pid" ]] || return 1
    process_tail="${process_stat##*) }"
    read -r -a process_fields <<<"$process_tail"
    ((${#process_fields[@]} > 19)) || return 1
    state="${process_fields[0]}"
    [[ "$state" != Z && "$state" != X ]] || continue
    ppid="${process_fields[1]}"
    pgid="${process_fields[2]}"
    token="${process_fields[19]}"
    [[ "$ppid" =~ ^[0-9]+$ && "$pgid" =~ ^[0-9]+$ && "$token" =~ ^[0-9]+$ ]] \
      || return 1
    printf '%s %s %s %s\n' "$pid" "$ppid" "$pgid" "$token"
  done
}

remote_fixture_tokenized_descendants_deepest() {
  local parent_pid="$1"
  local table
  table="$(remote_fixture_tokenized_process_table)" || return 1
  node -e '
const fs = require("fs");
const root = process.argv[1];
const children = new Map();
for (const line of fs.readFileSync(0, "utf8").split(/\r?\n/)) {
  const [pid, ppid, _pgid, token] = line.trim().split(/\s+/);
  if (!pid || !ppid || !token) continue;
  if (!children.has(ppid)) children.set(ppid, []);
  children.get(ppid).push({ pid, token });
}
const visiting = new Set();
function emit(parent) {
  if (visiting.has(parent)) process.exit(2);
  visiting.add(parent);
  for (const child of children.get(parent) || []) {
    emit(child.pid);
    process.stdout.write(`${child.pid}|${child.token}\n`);
  }
  visiting.delete(parent);
}
emit(root);
' "$parent_pid" <<<"$table"
}

remote_fixture_tokenized_process_group_members() {
  local group_id="$1"
  local table
  table="$(remote_fixture_tokenized_process_table)" || return 1
  awk -v group_id="$group_id" '$3 == group_id { print $1 "|" $4 }' <<<"$table"
}

remote_fixture_assert_isolated_process_group() {
  local root_pid="$1"
  local table root_group supervisor_group
  local supervisor_pid="$BASHPID"
  table="$(remote_fixture_process_table)" || {
    remote_fixture_fail "could not inspect the fixture worker process group"
    return 1
  }
  root_group="$(awk -v pid="$root_pid" \
    '$1 == pid { print $3; count++ } END { if (count != 1) exit 1 }' <<<"$table")" || {
    remote_fixture_fail "fixture worker process-group identity is missing or ambiguous"
    return 1
  }
  [[ "$root_group" == "$root_pid" ]] || {
    remote_fixture_fail "fixture worker is not the leader of its isolated process group"
    return 1
  }
  supervisor_group="$(awk -v pid="$supervisor_pid" \
    '$1 == pid { print $3; count++ } END { if (count != 1) exit 1 }' <<<"$table")" || {
    remote_fixture_fail "fixture supervisor process-group identity is missing or ambiguous"
    return 1
  }
  [[ "$supervisor_group" != "$root_group" ]] || {
    remote_fixture_fail "fixture worker process group overlaps the supervisor"
    return 1
  }
}

remote_fixture_wait_isolated_process_group() {
  local root_pid="$1"
  local attempt
  for attempt in $(seq 1 100); do
    remote_fixture_assert_isolated_process_group "$root_pid" 2>/dev/null && return 0
    remote_fixture_job_active "$root_pid" || break
    sleep 0.01
  done
  remote_fixture_assert_isolated_process_group "$root_pid"
}

remote_fixture_identity_is_tracked() {
  local needle="$1"
  shift
  local entry
  for entry in "$@"; do [[ "$entry" == "$needle" ]] && return 0; done
  return 1
}

remote_fixture_descendants_deepest() {
  local parent_pid="$1"
  case "$REMOTE_FIXTURE_PLATFORM" in
    MINGW*|MSYS*|CYGWIN*) remote_fixture_msys_descendants "$parent_pid" ;;
    *) remote_fixture_posix_descendants "$parent_pid" ;;
  esac
}
