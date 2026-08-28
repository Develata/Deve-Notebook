#!/usr/bin/env bash

if ((BASH_VERSINFO[0] < 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] < 1))); then
  for modern_bash in "${DEVE_REMOTE_FIXTURE_MODERN_BASH:-}" \
    /opt/homebrew/bin/bash /usr/local/bin/bash; do
    [[ -n "$modern_bash" && -x "$modern_bash" ]] || continue
    "$modern_bash" -c '((BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1)))' \
      2>/dev/null || continue
    exec "$modern_bash" "$0" "$@"
  done
  printf '%s\n' 'remote-browser-fixture: Bash 5.1+ is required' >&2
  exit 1
fi
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-json.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-json.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-startup-state.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-startup-state.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-supervisor.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-supervisor.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-worker.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-worker.sh"
readonly DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT="$ROOT_DIR/scripts/remote-browser-fixture.sh"
export DEVE_REMOTE_FIXTURE_ENTRY_SCRIPT

usage() {
  cat >&2 <<'EOF'
Usage:
  remote-browser-fixture.sh start --state-dir DIR --expected-head SHA [source options]
  remote-browser-fixture.sh stop --state-dir DIR
  remote-browser-fixture.sh run <start options> -- COMMAND [ARG ...]

Internal executable source:
  --backend-executable FILE --backend-head-file FILE
  --password-hasher FILE [--password-hasher-arg ARG ...]
  [--cloudflared-executable FILE]

Internal Docker source:
  --backend-container-image IMAGE
  --password-hasher FILE [--password-hasher-arg ARG ...]
  [--cloudflared-executable FILE]

External staging override:
  --external-origin HTTPS_ORIGIN --external-head-proof-url URL
  --external-credentials-file FILE

The hasher receives `--password-file PATH` after any configured arguments and
must emit exactly one Argon2id PHC string. Secrets are never passed as argv.
EOF
  exit 2
}

reject_public_test_overrides() {
  local variable
  while read -r variable; do
    [[ "$variable" == DEVE_REMOTE_FIXTURE_TEST_* ]] || continue
    remote_fixture_fail "public fixture start/run rejects synthetic test overrides"
    return 1
  done < <(compgen -A variable DEVE_REMOTE_FIXTURE_TEST_)
}

stop_fixture() {
  local state_dir=""
  while (($#)); do
    case "$1" in
      --state-dir) state_dir="${2:-}"; shift 2 ;;
      *) usage ;;
    esac
  done
  [[ -n "$state_dir" ]] || usage
  state_dir="$(remote_fixture_canonical_dir "$state_dir")"
  local state_file="$state_dir/fixture-state.json"
  local owner_file="$state_dir/.fixture-owner"
  if [[ ! -f "$state_file" || -L "$state_file" || ! -f "$owner_file" || -L "$owner_file" ]]; then
    remote_fixture_fail "owned fixture state is missing or unsafe"
    return 1
  fi

  local fixture_id backend_pid backend_token tunnel_pid tunnel_token container_name credentials_file environment_file
  fixture_id="$(remote_fixture_json_field "$state_file" fixture_id)"
  if [[ -z "$fixture_id" || "$(tr -d '\r\n' <"$owner_file")" != "$fixture_id" ]]; then
    remote_fixture_fail "fixture owner marker does not match state"
    return 1
  fi
  backend_pid="$(remote_fixture_json_field "$state_file" backend_pid)"
  backend_token="$(remote_fixture_json_field "$state_file" backend_process_token)"
  tunnel_pid="$(remote_fixture_json_field "$state_file" tunnel_pid)"
  tunnel_token="$(remote_fixture_json_field "$state_file" tunnel_process_token)"
  container_name="$(remote_fixture_json_field "$state_file" container_name)"
  credentials_file="$(remote_fixture_json_field "$state_file" credentials_file)"
  environment_file="$(remote_fixture_json_field "$state_file" environment_file)"

  local cleanup_failed=0
  local secret_path
  for secret_path in "$state_dir/.username" "$state_dir/.password" "$state_dir/.auth-secret" \
    "$state_dir/.auth-pass" "$state_dir/password-hasher.stderr.log" "$state_dir/.backend.env"; do
    if [[ -e "$secret_path" || -L "$secret_path" ]]; then
      rm -f -- "$secret_path" || cleanup_failed=1
    fi
  done
  if [[ "$(remote_fixture_path_key "$credentials_file")" != "$(remote_fixture_path_key "$state_dir/credentials.json")" || \
    "$(remote_fixture_path_key "$environment_file")" != "$(remote_fixture_path_key "$state_dir/fixture-env.json")" ]]; then
    remote_fixture_fail "refusing to remove fixture files outside fixture state directory" || true
    cleanup_failed=1
  else
    for secret_path in "$credentials_file" "$environment_file"; do
      if [[ -e "$secret_path" || -L "$secret_path" ]]; then
        rm -f -- "$secret_path" || cleanup_failed=1
      fi
    done
  fi

  remote_fixture_stop_pid tunnel "$tunnel_pid" "$tunnel_token" || cleanup_failed=1
  remote_fixture_stop_pid backend "$backend_pid" "$backend_token" || cleanup_failed=1
  if [[ -n "$container_name" ]]; then
    local container_presence
    if remote_fixture_container_presence "$container_name"; then container_presence=0; else container_presence=$?; fi
    if [[ "$container_presence" == "0" ]]; then
      if remote_fixture_verify_container_owner "$container_name" "$fixture_id"; then
        docker rm --force "$container_name" >/dev/null || cleanup_failed=1
      else
        cleanup_failed=1
      fi
    elif [[ "$container_presence" == "2" ]]; then
      cleanup_failed=1
    fi
    if remote_fixture_container_presence "$container_name"; then container_presence=0; else container_presence=$?; fi
    if [[ "$container_presence" != "1" ]]; then
      remote_fixture_fail "owned backend container absence could not be proven"
      cleanup_failed=1
    fi
  fi
  if [[ -n "$backend_pid" ]] && remote_fixture_live_pid_matches_token "$backend_pid" "$backend_token"; then
    remote_fixture_fail "owned backend process survived cleanup"
    cleanup_failed=1
  fi
  if [[ -n "$tunnel_pid" ]] && remote_fixture_live_pid_matches_token "$tunnel_pid" "$tunnel_token"; then
    remote_fixture_fail "owned tunnel process survived cleanup"
    cleanup_failed=1
  fi
  if [[ "$cleanup_failed" == "1" ]]; then
    remote_fixture_fail "one or more fixture resources survived cleanup; ownership state was preserved"
    return 1
  fi
  remote_fixture_remove_startup_state "$state_dir"
  remote_fixture_remove_startup_admission "$state_dir"
  rm -f -- "$state_file" "$owner_file"
  printf '%s\n' "stopped" >"$state_dir/.fixture-stopped"
}

run_fixture() {
  local -a start_args=()
  while (($#)) && [[ "$1" != "--" ]]; do start_args+=("$1"); shift; done
  [[ "${1:-}" == "--" ]] || usage
  shift
  (($#)) || usage
  local state_dir=""
  local index
  for index in "${!start_args[@]}"; do
    if [[ "${start_args[$index]}" == "--state-dir" ]]; then state_dir="${start_args[$((index + 1))]:-}"; fi
  done
  [[ -n "$state_dir" ]] || usage
  start_fixture "${start_args[@]}" >/dev/null
  state_dir="$(remote_fixture_canonical_dir "$state_dir")"
  local cleanup_complete=0
  cleanup_run_fixture() {
    local exit_status="${1:-1}"
    trap - EXIT
    trap ':' INT TERM
    if [[ "$cleanup_complete" == "0" ]]; then
      stop_fixture --state-dir "$state_dir" || exit_status=1
      cleanup_complete=1
    fi
    exit "$exit_status"
  }
  trap 'cleanup_run_fixture $?' EXIT
  trap 'cleanup_run_fixture 130' INT
  trap 'cleanup_run_fixture 143' TERM
  local state_file="$state_dir/fixture-state.json"
  local credentials_file origin username password auth_secret
  credentials_file="$(remote_fixture_json_field "$state_file" credentials_file)"
  origin="$(remote_fixture_json_field "$state_file" https_origin)"
  username="$(remote_fixture_json_field "$credentials_file" username)"
  password="$(remote_fixture_json_field "$credentials_file" password)"
  auth_secret="$(remote_fixture_json_field "$credentials_file" auth_secret)"
  local status=0 cleanup_status=0
  DEVE_REMOTE_FIXTURE_HTTPS_ORIGIN="$origin" \
    DEVE_REMOTE_FIXTURE_USERNAME="$username" \
    DEVE_REMOTE_FIXTURE_PASSWORD="$password" \
    DEVE_REMOTE_FIXTURE_AUTH_SECRET="$auth_secret" \
    DEVE_REMOTE_FIXTURE_STATE_FILE="$state_file" \
    "$@" || status=$?
  stop_fixture --state-dir "$state_dir" || cleanup_status=$?
  cleanup_complete=1
  trap - EXIT INT TERM
  ((status != 0)) && return "$status"
  return "$cleanup_status"
}

case "${1:-}" in
  __start-worker) shift; start_fixture_worker "$@" ;;
  __admit-startup)
    shift
    (($# == 2)) || {
      remote_fixture_fail "internal admission publisher requires state and decision paths"
      exit 2
    }
    remote_fixture_admit_startup_state "$1" "$2"
    ;;
  __test-start)
    shift
    export DEVE_REMOTE_FIXTURE_TEST_MODE=1
    case "$REMOTE_FIXTURE_PLATFORM" in
      MINGW*|MSYS*|CYGWIN*) export DEVE_REMOTE_FIXTURE_TEST_ALLOW_UNGROUPED=1 ;;
    esac
    start_fixture "$@"
    ;;
  start) shift; reject_public_test_overrides; start_fixture "$@" ;;
  stop) shift; stop_fixture "$@" ;;
  run) shift; reject_public_test_overrides; run_fixture "$@" ;;
  *) usage ;;
esac
