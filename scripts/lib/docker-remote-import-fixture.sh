#!/usr/bin/env bash
# shellcheck shell=bash

# B6-only lifecycle helpers. Durable Remote Import state remains owned by the
# candidate backend; this file owns only ephemeral Docker/tunnel resources.

DEVE_REMOTE_IMPORT_DOCKER_BIN="${DEVE_REMOTE_IMPORT_DOCKER_BIN:-${DEVE_DOCKER_BIN:-docker}}"
export DEVE_REMOTE_IMPORT_DOCKER_BIN
readonly DEVE_REMOTE_IMPORT_ABSENCE_SCRIPT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P
)/docker-remote-import-absence.sh"
readonly DEVE_REMOTE_IMPORT_DOCKER_CLEANUP_TIMEOUT_SECONDS=15
readonly DEVE_REMOTE_IMPORT_DOCKER_ABSENCE_TIMEOUT_SECONDS=5

remote_import_fixture_fail() {
  printf 'docker-remote-import: %s\n' "$*" >&2
  return 1
}

remote_import_fixture_state_file() {
  printf '%s/docker-remote-import/fixture-state.json\n' \
    "${DEVE_ACCEPTANCE_PRODUCER_STATE_DIR:-${DEVE_REMOTE_IMPORT_STATE_ROOT:-${TMPDIR:-/tmp}/deve-remote-import-$$}}"
}

remote_import_fixture_write_state() {
  local state_file="$1"
  mkdir -p -- "$(dirname -- "$state_file")"
  STATE_FILE="$state_file" \
  PROJECT="${DEVE_REMOTE_IMPORT_PROJECT:-}" \
  COMPOSE_FILE="${DEVE_REMOTE_IMPORT_COMPOSE_FILE:-}" \
  WEBDAV_FAILURE_TUNNEL_PID="${DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_PID:-}" \
  WEBDAV_FAILURE_TUNNEL_TOKEN="${DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_TOKEN:-}" \
  WEBDAV_TUNNEL_PID="${DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_PID:-}" \
  WEBDAV_TUNNEL_TOKEN="${DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_TOKEN:-}" \
  S3_TUNNEL_PID="${DEVE_REMOTE_IMPORT_S3_TUNNEL_PID:-}" \
  S3_TUNNEL_TOKEN="${DEVE_REMOTE_IMPORT_S3_TUNNEL_TOKEN:-}" \
    node <<'NODE'
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const state = {
  schema: 1,
  project: process.env.PROJECT,
  compose_file: process.env.COMPOSE_FILE,
  webdav_failure_tunnel: {
    pid: process.env.WEBDAV_FAILURE_TUNNEL_PID || null,
    token: process.env.WEBDAV_FAILURE_TUNNEL_TOKEN || null,
  },
  webdav_tunnel: {
    pid: process.env.WEBDAV_TUNNEL_PID || null,
    token: process.env.WEBDAV_TUNNEL_TOKEN || null,
  },
  s3_tunnel: {
    pid: process.env.S3_TUNNEL_PID || null,
    token: process.env.S3_TUNNEL_TOKEN || null,
  },
};
const stateFile = process.env.STATE_FILE;
const temporary = `${stateFile}.tmp-${process.pid}-${crypto.randomUUID()}`;
let descriptor;
try {
  descriptor = fs.openSync(temporary, "wx", 0o600);
  fs.writeFileSync(descriptor, `${JSON.stringify(state, null, 2)}\n`, "utf8");
  fs.fsyncSync(descriptor);
  fs.closeSync(descriptor);
  descriptor = undefined;
  fs.renameSync(temporary, stateFile);
  fs.chmodSync(stateFile, 0o600);
  try {
    const directory = fs.openSync(path.dirname(stateFile), "r");
    try {
      fs.fsyncSync(directory);
    } finally {
      fs.closeSync(directory);
    }
  } catch (error) {
    if (process.platform !== "win32") throw error;
  }
} finally {
  if (descriptor !== undefined) fs.closeSync(descriptor);
  try {
    fs.unlinkSync(temporary);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}
NODE
  chmod 0600 "$state_file"
}

remote_import_fixture_state_field() {
  local state_file="$1"
  local expression="$2"
  node -e '
const fs = require("node:fs");
const state = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const path = process.argv[2].split(".");
let value = state;
for (const key of path) value = value?.[key];
if (value !== null && value !== undefined) process.stdout.write(String(value));
' "$state_file" "$expression"
}

remote_import_fixture_cleanup() {
  local state_file="${1:-$(remote_import_fixture_state_file)}"
  local state_root
  state_root="$(dirname -- "$state_file")"
  rm -f -- "$state_root/.auth-password" "$state_root/.auth-pass"
  [[ -f "$state_file" ]] || return 0
  local project compose_file
  project="$(remote_import_fixture_state_field "$state_file" project)"
  compose_file="$(remote_import_fixture_state_field "$state_file" compose_file)"
  [[ "$project" =~ ^deve-remote-import-[0-9a-f]{12}$ ]] \
    || remote_import_fixture_fail "refusing cleanup for invalid project identity"
  [[ -f "$compose_file" ]] || remote_import_fixture_fail "cleanup compose file is missing"

  local status=0 pid token
  for label in webdav_failure webdav s3; do
    pid="$(remote_import_fixture_state_field "$state_file" "${label}_tunnel.pid")"
    token="$(remote_import_fixture_state_field "$state_file" "${label}_tunnel.token")"
    if [[ -n "$pid" || -n "$token" ]]; then
      remote_fixture_stop_pid "$label tunnel" "$pid" "$token" || status=1
    fi
  done
  remote_fixture_run_bounded "Remote Import Docker cleanup" \
    "$DEVE_REMOTE_IMPORT_DOCKER_CLEANUP_TIMEOUT_SECONDS" 4194304 \
    "$state_root/cleanup.stdout.log" "$state_root/cleanup.stderr.log" -- \
    env \
      DEVE_RELEASE_CANDIDATE_IMAGE="${DEVE_RELEASE_CANDIDATE_IMAGE:-deve-notebook:cleanup-placeholder}" \
      DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_PORT=19080 \
      DEVE_REMOTE_IMPORT_WEBDAV_PORT=19081 \
      DEVE_REMOTE_IMPORT_S3_PORT=19082 \
      DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_APP_PORT=19085 \
      DEVE_REMOTE_IMPORT_WEBDAV_APP_PORT=19083 \
      DEVE_REMOTE_IMPORT_S3_APP_PORT=19084 \
      DEVE_REMOTE_IMPORT_WEBDAV_FIXTURE="$state_root" \
      DEVE_REMOTE_IMPORT_S3_ACCESS_KEY_ID=cleanup \
      DEVE_REMOTE_IMPORT_S3_SECRET_ACCESS_KEY=cleanup-cleanup \
      DEVE_REMOTE_IMPORT_S3_BUCKET=cleanup-bucket \
      DEVE_REMOTE_IMPORT_S3_PREFIX=cleanup/prefix \
      DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_LOCATOR=https://cleanup.invalid/failure \
      DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_HOST=cleanup.invalid \
      DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_EDGE_IP=127.0.0.1 \
      DEVE_REMOTE_IMPORT_WEBDAV_LOCATOR=https://cleanup.invalid/root \
      DEVE_REMOTE_IMPORT_WEBDAV_HOST=cleanup.invalid \
      DEVE_REMOTE_IMPORT_WEBDAV_EDGE_IP=127.0.0.1 \
      DEVE_REMOTE_IMPORT_S3_LOCATOR=https://cleanup.invalid/bucket/prefix \
      DEVE_REMOTE_IMPORT_S3_ORIGIN=https://cleanup.invalid \
      DEVE_REMOTE_IMPORT_S3_HOST=cleanup.invalid \
      DEVE_REMOTE_IMPORT_S3_EDGE_IP=127.0.0.1 \
      DEVE_REMOTE_IMPORT_AUTH_SECRET=cleanup-cleanup-cleanup-cleanup-32 \
      DEVE_REMOTE_IMPORT_AUTH_USER=cleanup \
      'DEVE_REMOTE_IMPORT_AUTH_PASS=$argon2id$v=19$m=8,t=1,p=1$YQ$YQ' \
      "$DEVE_REMOTE_IMPORT_DOCKER_BIN" compose -f "$compose_file" -p "$project" \
      down --timeout 5 -v --remove-orphans || status=1
  remote_fixture_run_bounded "Remote Import Docker absence verification" \
    "$DEVE_REMOTE_IMPORT_DOCKER_ABSENCE_TIMEOUT_SECONDS" 1048576 \
    "$state_root/absence.stdout.log" "$state_root/absence.stderr.log" -- \
    bash "$DEVE_REMOTE_IMPORT_ABSENCE_SCRIPT" "$project" || status=1
  if ((status == 0)); then
    rm -f -- "$state_file"
  fi
  return "$status"
}

remote_import_fixture_compose() {
  "$DEVE_REMOTE_IMPORT_DOCKER_BIN" compose -f "$DEVE_REMOTE_IMPORT_COMPOSE_FILE" \
    -p "$DEVE_REMOTE_IMPORT_PROJECT" "$@"
}

remote_import_fixture_verify_candidate() {
  local observed
  observed="$("$DEVE_REMOTE_IMPORT_DOCKER_BIN" image inspect --format '{{.Id}}' "$DEVE_RELEASE_CANDIDATE_IMAGE")" \
    || remote_import_fixture_fail "candidate image is unavailable"
  [[ "$observed" == "$DEVE_RELEASE_CANDIDATE_IMAGE_ID" ]] \
    || remote_import_fixture_fail \
      "candidate image mismatch: expected=$DEVE_RELEASE_CANDIDATE_IMAGE_ID observed=$observed"
}

remote_import_fixture_verify_candidate_container() {
  local service="$1"
  local container observed
  container="$(remote_import_fixture_container_id "$service")"
  observed="$("$DEVE_REMOTE_IMPORT_DOCKER_BIN" inspect --format '{{.Image}}' "$container")" \
    || remote_import_fixture_fail "candidate container image is unavailable for $service"
  [[ "$observed" == "$DEVE_RELEASE_CANDIDATE_IMAGE_ID" ]] \
    || remote_import_fixture_fail \
      "$service container image mismatch: expected=$DEVE_RELEASE_CANDIDATE_IMAGE_ID observed=$observed"
}

remote_import_fixture_start_tunnel() {
  local label="$1"
  local port="$2"
  local state_file="$3"
  local cloudflared="$4"
  local log_root
  log_root="$(dirname -- "$state_file")"
  "$cloudflared" tunnel --no-autoupdate --protocol http2 \
    --url "http://127.0.0.1:$port" \
    >"$log_root/${label}-tunnel.stdout.log" \
    2>"$log_root/${label}-tunnel.stderr.log" &
  local pid="$!"
  local token=""
  local attempt
  for attempt in $(seq 1 50); do
    token="$(remote_fixture_process_token "$pid" 2>/dev/null)" || token=""
    [[ -n "$token" ]] && break
    if ! remote_fixture_pid_active "$pid" && ! remote_fixture_job_active "$pid"; then
      wait "$pid" 2>/dev/null || true
      remote_import_fixture_fail "$label tunnel exited before process identity binding"
      return 1
    fi
    sleep 0.1
  done
  if [[ -z "$token" ]]; then
    remote_fixture_stop_owned_job "$label tunnel" "$pid" || {
      remote_import_fixture_fail \
        "could not reclaim unbound $label tunnel process"
      return 1
    }
    remote_import_fixture_fail "could not bind $label tunnel process identity"
    return 1
  fi
  case "$label" in
    webdav_failure)
      export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_PID="$pid"
      export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_TOKEN="$token"
      ;;
    webdav)
      export DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_PID="$pid"
      export DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_TOKEN="$token"
      ;;
    s3)
      export DEVE_REMOTE_IMPORT_S3_TUNNEL_PID="$pid"
      export DEVE_REMOTE_IMPORT_S3_TUNNEL_TOKEN="$token"
      ;;
    *)
      if ! remote_fixture_stop_owned_job "$label tunnel" "$pid" "$token"; then
        remote_import_fixture_fail "could not reclaim unknown tunnel label: $label"
        return 1
      fi
      remote_import_fixture_fail "unknown tunnel label: $label"
      return 1
      ;;
  esac
  if ! remote_import_fixture_write_state "$state_file"; then
    remote_fixture_stop_owned_job "$label tunnel" "$pid" "$token" || {
      remote_import_fixture_fail \
        "could not reclaim unpersisted $label tunnel process"
      return 1
    }
    remote_import_fixture_fail "could not persist $label tunnel ownership state"
    return 1
  fi
  local origin
  origin="$(remote_fixture_wait_tunnel_origin \
    "$pid" "$log_root/${label}-tunnel.stdout.log" "$log_root/${label}-tunnel.stderr.log")"
  case "$label" in
    webdav_failure) export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_ORIGIN="$origin" ;;
    webdav) export DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN="$origin" ;;
    s3) export DEVE_REMOTE_IMPORT_S3_ORIGIN="$origin" ;;
  esac
}

remote_import_fixture_stop_tunnel() {
  local label="$1"
  local state_file="$2"
  local pid token
  case "$label" in
    webdav_failure)
      pid="${DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_PID:-}"
      token="${DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_TOKEN:-}"
      ;;
    webdav)
      pid="${DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_PID:-}"
      token="${DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_TOKEN:-}"
      ;;
    s3)
      pid="${DEVE_REMOTE_IMPORT_S3_TUNNEL_PID:-}"
      token="${DEVE_REMOTE_IMPORT_S3_TUNNEL_TOKEN:-}"
      ;;
    *)
      remote_import_fixture_fail "unknown tunnel label: $label"
      return 1
      ;;
  esac
  remote_fixture_stop_pid "$label tunnel" "$pid" "$token"
  case "$label" in
    webdav_failure)
      unset DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_PID
      unset DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_TOKEN
      unset DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_ORIGIN
      ;;
    webdav)
      unset DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_PID
      unset DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_TOKEN
      unset DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN
      ;;
    s3)
      unset DEVE_REMOTE_IMPORT_S3_TUNNEL_PID
      unset DEVE_REMOTE_IMPORT_S3_TUNNEL_TOKEN
      unset DEVE_REMOTE_IMPORT_S3_ORIGIN
      ;;
  esac
  remote_import_fixture_write_state "$state_file"
}

remote_import_fixture_wait_url() {
  local label="$1"
  local url="$2"
  local attempts="${3:-120}"
  local status
  local probe_error
  probe_error="$(dirname -- "$(remote_import_fixture_state_file)")/${label}-probe.stderr.log"
  for _ in $(seq 1 "$attempts"); do
    status="$(curl --noproxy '127.0.0.1,localhost' --silent --show-error \
      --output /dev/null --write-out '%{http_code}' --max-time 2 "$url" \
      2>"$probe_error" || true)"
    if [[ "$status" =~ ^2[0-9][0-9]$ ]]; then
      rm -f -- "$probe_error"
      return 0
    fi
    sleep 0.5
  done
  remote_import_fixture_fail \
    "timed out waiting for $label at $url (last HTTP status: ${status:-none})"
}

remote_import_fixture_wait_container_url() {
  local service="$1"
  local label="$2"
  local url="$3"
  local attempts="${4:-120}"
  local required_successes="${5:-1}"
  local method="${6:-GET}"
  [[ "$attempts" =~ ^[1-9][0-9]*$ \
    && "$required_successes" =~ ^[1-9][0-9]*$ \
    && "$required_successes" -le "$attempts" ]] \
    || remote_import_fixture_fail "candidate network probe budget is invalid"
  [[ "$method" == "GET" || "$method" == "PROPFIND" ]] \
    || remote_import_fixture_fail "candidate network probe method is invalid"
  local container probe_error status consecutive_successes=0
  container="$(remote_import_fixture_container_id "$service")"
  probe_error="$(dirname -- "$(remote_import_fixture_state_file)")/${label}-probe.stderr.log"
  for _ in $(seq 1 "$attempts"); do
    local -a curl_args=(
      --silent
      --show-error
      --output /dev/null
      --write-out '%{http_code}'
      --max-time 2
      --http1.1
      --request "$method"
    )
    if [[ "$method" == "PROPFIND" ]]; then
      curl_args+=(
        --header "Depth: 1"
        --header "Content-Type: application/xml; charset=utf-8"
        --data-binary '<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/></d:prop></d:propfind>'
      )
    fi
    status="$("$DEVE_REMOTE_IMPORT_DOCKER_BIN" exec "$container" curl \
      "${curl_args[@]}" "$url" 2>"$probe_error" || true)"
    if [[ "$status" =~ ^2[0-9][0-9]$ ]]; then
      ((consecutive_successes += 1))
      if ((consecutive_successes >= required_successes)); then
        rm -f -- "$probe_error"
        return 0
      fi
    else
      consecutive_successes=0
    fi
    sleep 0.5
  done
  remote_import_fixture_fail \
    "timed out waiting for stable $label $method from candidate network (last HTTP status: ${status:-none}; consecutive successes: $consecutive_successes/$required_successes)"
}

remote_import_fixture_container_id() {
  local service="$1"
  local id
  id="$(remote_import_fixture_compose ps -q "$service")"
  [[ -n "$id" ]] || remote_import_fixture_fail "container id unavailable for $service"
  printf '%s\n' "$id"
}
