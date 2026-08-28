#!/usr/bin/env bash
# shellcheck shell=bash

# Atomic, nonsecret startup ownership journal for the Unix RemoteBrowser
# fixture. The worker updates this file before admitting each externally owned
# resource; the supervisor consumes it only after exact owner preflight.

remote_fixture_startup_stage_valid() {
  case "$1" in
    secure-state-directory|external-proof|generate-credentials|hash-password|\
      initialize-backend|start-backend|wait-backend-health|prepare-cloudflared|\
      start-tunnel|wait-tunnel-origin|wait-public-health|publish-ready-state) return 0 ;;
    *) return 1 ;;
  esac
}

remote_fixture_save_startup_state() {
  local stage="$1"
  remote_fixture_startup_stage_valid "$stage" || {
    remote_fixture_fail "unknown fixture startup stage: $stage"
    return 1
  }
  REMOTE_FIXTURE_STARTUP_STAGE="$stage" \
    REMOTE_FIXTURE_STARTUP_STATE_FILE="$REMOTE_FIXTURE_STARTUP_STATE_FILE" \
    REMOTE_FIXTURE_STARTUP_FIXTURE_ID="$REMOTE_FIXTURE_STARTUP_FIXTURE_ID" \
    REMOTE_FIXTURE_STARTUP_SOURCE_KIND="$REMOTE_FIXTURE_STARTUP_SOURCE_KIND" \
    REMOTE_FIXTURE_STARTUP_BACKEND_PID="$REMOTE_FIXTURE_STARTUP_BACKEND_PID" \
    REMOTE_FIXTURE_STARTUP_BACKEND_TOKEN="$REMOTE_FIXTURE_STARTUP_BACKEND_TOKEN" \
    REMOTE_FIXTURE_STARTUP_TUNNEL_PID="$REMOTE_FIXTURE_STARTUP_TUNNEL_PID" \
    REMOTE_FIXTURE_STARTUP_TUNNEL_TOKEN="$REMOTE_FIXTURE_STARTUP_TUNNEL_TOKEN" \
    REMOTE_FIXTURE_STARTUP_CONTAINER_NAME="$REMOTE_FIXTURE_STARTUP_CONTAINER_NAME" \
    REMOTE_FIXTURE_STARTUP_CREDENTIALS_FILE="$REMOTE_FIXTURE_STARTUP_CREDENTIALS_FILE" \
    REMOTE_FIXTURE_STARTUP_ENVIRONMENT_FILE="$REMOTE_FIXTURE_STARTUP_ENVIRONMENT_FILE" \
    node <<'NODE'
const crypto = require("crypto");
const fs = require("fs");
const env = process.env;
const nullableNumber = (value) => value ? Number(value) : null;
const nullableString = (value) => value || null;
const state = {
  schema: 1,
  fixture_id: env.REMOTE_FIXTURE_STARTUP_FIXTURE_ID,
  stage: env.REMOTE_FIXTURE_STARTUP_STAGE,
  updated_at: new Date().toISOString(),
  source_kind: nullableString(env.REMOTE_FIXTURE_STARTUP_SOURCE_KIND),
  backend_pid: nullableNumber(env.REMOTE_FIXTURE_STARTUP_BACKEND_PID),
  backend_process_token: nullableString(env.REMOTE_FIXTURE_STARTUP_BACKEND_TOKEN),
  tunnel_pid: nullableNumber(env.REMOTE_FIXTURE_STARTUP_TUNNEL_PID),
  tunnel_process_token: nullableString(env.REMOTE_FIXTURE_STARTUP_TUNNEL_TOKEN),
  container_name: nullableString(env.REMOTE_FIXTURE_STARTUP_CONTAINER_NAME),
  credentials_file: env.REMOTE_FIXTURE_STARTUP_CREDENTIALS_FILE,
  environment_file: env.REMOTE_FIXTURE_STARTUP_ENVIRONMENT_FILE,
};
const destination = env.REMOTE_FIXTURE_STARTUP_STATE_FILE;
const temporary = `${destination}.${process.pid}.${crypto.randomBytes(8).toString("hex")}.tmp`;
let descriptor;
try {
  descriptor = fs.openSync(temporary, "wx", 0o600);
  fs.writeFileSync(descriptor, `${JSON.stringify(state, null, 2)}\n`);
  fs.fsyncSync(descriptor);
  fs.closeSync(descriptor);
  descriptor = undefined;
  fs.renameSync(temporary, destination);
  fs.chmodSync(destination, 0o600);
} finally {
  if (descriptor !== undefined) fs.closeSync(descriptor);
  try { fs.unlinkSync(temporary); } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}
NODE
}

remote_fixture_initialize_startup_state() {
  local state_dir="$1"
  local fixture_id="$2"
  REMOTE_FIXTURE_STARTUP_STATE_FILE="$state_dir/startup-state.json"
  REMOTE_FIXTURE_STARTUP_FIXTURE_ID="$fixture_id"
  REMOTE_FIXTURE_STARTUP_SOURCE_KIND=""
  REMOTE_FIXTURE_STARTUP_BACKEND_PID=""
  REMOTE_FIXTURE_STARTUP_BACKEND_TOKEN=""
  REMOTE_FIXTURE_STARTUP_TUNNEL_PID=""
  REMOTE_FIXTURE_STARTUP_TUNNEL_TOKEN=""
  REMOTE_FIXTURE_STARTUP_CONTAINER_NAME=""
  REMOTE_FIXTURE_STARTUP_CREDENTIALS_FILE="$state_dir/credentials.json"
  REMOTE_FIXTURE_STARTUP_ENVIRONMENT_FILE="$state_dir/fixture-env.json"
  remote_fixture_save_startup_state secure-state-directory
}

remote_fixture_remove_startup_state() {
  local state_dir="$1"
  rm -f -- "$state_dir/startup-state.json" "$state_dir"/startup-state.json.*.tmp
}

remote_fixture_remove_startup_admission() {
  local state_dir="$1"
  rm -f -- "$state_dir/.startup-admitted" "$state_dir"/.startup-admitted.*.tmp \
    "$state_dir/.startup-admission-decision"
}

remote_fixture_validate_startup_state() {
  local state_dir="$1"
  local state_file="$state_dir/startup-state.json"
  [[ -f "$state_file" && ! -L "$state_file" ]] || {
    remote_fixture_fail "startup ownership state is missing or unsafe"
    return 1
  }
  if ! node - "$state_file" "$state_dir/credentials.json" "$state_dir/fixture-env.json" <<'NODE'
const fs = require("fs");
const [statePath, credentialsPath, environmentPath] = process.argv.slice(2);
try {
const state = JSON.parse(fs.readFileSync(statePath, "utf8"));
const stages = new Set([
  "secure-state-directory", "external-proof", "generate-credentials", "hash-password",
  "initialize-backend", "start-backend", "wait-backend-health", "prepare-cloudflared",
  "start-tunnel", "wait-tunnel-origin", "wait-public-health", "publish-ready-state",
]);
const required = [
  "schema", "fixture_id", "stage", "updated_at", "source_kind",
  "backend_pid", "backend_process_token", "tunnel_pid", "tunnel_process_token",
  "container_name", "credentials_file", "environment_file",
];
if (!required.every((field) => Object.hasOwn(state, field))) process.exit(1);
if (state.schema !== 1 || !/^[0-9a-f]{32}$/.test(state.fixture_id)
    || !stages.has(state.stage) || typeof state.updated_at !== "string"
    || state.credentials_file !== credentialsPath || state.environment_file !== environmentPath) process.exit(1);
const pair = (pid, token) => (pid === null && token === null)
  || (Number.isSafeInteger(pid) && pid > 0 && typeof token === "string" && token.length > 0);
if (!pair(state.backend_pid, state.backend_process_token)
    || !pair(state.tunnel_pid, state.tunnel_process_token)) process.exit(1);
if (state.source_kind !== null && !["external", "executable", "container"].includes(state.source_kind)) process.exit(1);
if (state.container_name !== null
    && (typeof state.container_name !== "string" || !/^deve-remote-fixture-[0-9a-f]{12}$/.test(state.container_name))) process.exit(1);
if ((state.source_kind === null || state.source_kind === "external")
    && (state.backend_pid !== null || state.tunnel_pid !== null || state.container_name !== null)) process.exit(1);
if (state.source_kind === "executable" && state.container_name !== null) process.exit(1);
if (state.source_kind === "container" && (state.backend_pid !== null || state.container_name === null)) process.exit(1);
} catch {
  process.exit(1);
}
NODE
  then
    remote_fixture_fail "startup ownership state is invalid and will not be consumed"
    return 1
  fi
}

remote_fixture_existing_state_dir() {
  local state_dir="$1"
  [[ -d "$state_dir" && ! -L "$state_dir" ]] || {
    remote_fixture_fail "startup recovery state directory is missing or a symlink"
    return 1
  }
  (cd -- "$state_dir" && pwd -P)
}

remote_fixture_publish_startup_admission() {
  local admission_file="$1"
  local fixture_id="$2"
  local decision_file="$3"
  REMOTE_FIXTURE_ADMISSION_FILE="$admission_file" \
    REMOTE_FIXTURE_ADMISSION_ID="$fixture_id" \
    REMOTE_FIXTURE_ADMISSION_DECISION_FILE="$decision_file" node <<'NODE'
const crypto = require("crypto");
const fs = require("fs");
const destination = process.env.REMOTE_FIXTURE_ADMISSION_FILE;
const decision = process.env.REMOTE_FIXTURE_ADMISSION_DECISION_FILE;
const temporary = `${destination}.${process.pid}.${crypto.randomBytes(8).toString("hex")}.tmp`;
let decisionDescriptor;
let descriptor;
try {
  try {
    decisionDescriptor = fs.openSync(decision, "wx", 0o600);
  } catch (error) {
    if (error.code === "EEXIST") process.exit(3);
    throw error;
  }
  fs.fsyncSync(decisionDescriptor);
  fs.closeSync(decisionDescriptor);
  decisionDescriptor = undefined;
  descriptor = fs.openSync(temporary, "wx", 0o600);
  fs.writeFileSync(descriptor, `${process.env.REMOTE_FIXTURE_ADMISSION_ID}\n`);
  fs.fsyncSync(descriptor);
  fs.closeSync(descriptor);
  descriptor = undefined;
  fs.renameSync(temporary, destination);
  fs.chmodSync(destination, 0o600);
} finally {
  if (decisionDescriptor !== undefined) fs.closeSync(decisionDescriptor);
  if (descriptor !== undefined) fs.closeSync(descriptor);
  try { fs.unlinkSync(temporary); } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}
NODE
}

remote_fixture_admit_startup_state() {
  local state_dir="$1"
  local decision_file="${2:-}"
  state_dir="$(remote_fixture_existing_state_dir "$state_dir")" || return 1
  [[ -n "$decision_file" ]] || decision_file="$state_dir/.startup-admission-decision"
  remote_fixture_validate_startup_state "$state_dir" || return 1
  local state_file="$state_dir/startup-state.json"
  local owner_file="$state_dir/.fixture-owner"
  local admission_file="$state_dir/.startup-admitted"
  [[ "$decision_file" == "$state_dir/.startup-admission-decision" ]] || {
    remote_fixture_fail "startup admission decision path escaped its state directory"
    return 1
  }
  [[ -f "$owner_file" && ! -L "$owner_file" ]] || {
    remote_fixture_fail "fixture owner marker is missing or unsafe for startup admission"
    return 1
  }
  [[ ! -e "$admission_file" && ! -L "$admission_file" ]] || {
    remote_fixture_fail "startup admission marker already exists"
    return 1
  }
  [[ ! -L "$decision_file" ]] || {
    remote_fixture_fail "startup admission decision capability is unsafe"
    return 1
  }
  local fixture_id
  fixture_id="$(remote_fixture_json_field "$state_file" fixture_id)"
  [[ "$(tr -d '\r\n' <"$owner_file")" == "$fixture_id" ]] || {
    remote_fixture_fail "fixture owner marker does not match startup admission state"
    return 1
  }
  remote_fixture_publish_startup_admission \
    "$admission_file" "$fixture_id" "$decision_file"
}

remote_fixture_wait_startup_admission() {
  local state_dir="$1"
  local fixture_id="$2"
  local parent_pid="$3"
  local parent_token="$4"
  local attempts=200
  local delay=0.1
  if [[ "${DEVE_REMOTE_FIXTURE_TEST_MODE:-0}" == 1 ]]; then
    attempts="${DEVE_REMOTE_FIXTURE_TEST_ADMISSION_ATTEMPTS:-$attempts}"
    delay="${DEVE_REMOTE_FIXTURE_TEST_ADMISSION_DELAY:-$delay}"
  fi
  [[ "$attempts" =~ ^[0-9]+$ && "$attempts" -gt 0 ]] || {
    remote_fixture_fail "startup admission attempts must be a positive integer"
    return 1
  }
  [[ "$delay" =~ ^[0-9]+([.][0-9]+)?$ && "$delay" != 0 && "$delay" != 0.0 ]] || {
    remote_fixture_fail "startup admission delay must be a positive number"
    return 1
  }
  local admission_file="$state_dir/.startup-admitted"
  local attempt observed actual_parent_token readiness_notified=0
  for ((attempt = 0; attempt < attempts; attempt += 1)); do
    if [[ "$PPID" != "$parent_pid" ]]; then
      remote_fixture_fail "startup supervisor parent relation changed before ownership admission"
      return 1
    fi
    actual_parent_token="$(remote_fixture_process_token "$parent_pid" 2>/dev/null || true)"
    if [[ -z "$parent_token" || "$actual_parent_token" != "$parent_token" ]]; then
      remote_fixture_fail "startup supervisor process token changed before ownership admission"
      return 1
    fi
    if [[ -e "$admission_file" || -L "$admission_file" ]]; then
      [[ -f "$admission_file" && ! -L "$admission_file" ]] || {
        remote_fixture_fail "startup admission marker is unsafe"
        return 1
      }
      observed="$(tr -d '\r\n' <"$admission_file")"
      [[ "$observed" == "$fixture_id" ]] || {
        remote_fixture_fail "startup admission marker does not match fixture ownership"
        return 1
      }
      [[ "$PPID" == "$parent_pid" \
        && "$(remote_fixture_process_token "$parent_pid" 2>/dev/null || true)" == "$parent_token" ]] || {
        remote_fixture_fail "startup supervisor identity changed while consuming ownership admission"
        return 1
      }
      remote_fixture_remove_startup_admission "$state_dir"
      [[ "$PPID" == "$parent_pid" \
        && "$(remote_fixture_process_token "$parent_pid" 2>/dev/null || true)" == "$parent_token" ]] || {
        remote_fixture_fail "startup supervisor identity changed after ownership admission"
        return 1
      }
      return 0
    fi
    if ((readiness_notified == 0)); then
      if [[ "$PPID" != "$parent_pid" ]] || ! kill -s USR1 -- "$PPID" 2>/dev/null; then
        remote_fixture_fail "startup supervisor disappeared before ownership admission"
        return 1
      fi
      readiness_notified=1
    fi
    sleep "$delay"
  done
  remote_fixture_fail "startup supervisor did not admit the exact worker before deadline"
  return 1
}

remote_fixture_recover_startup_state() {
  local state_dir="$1"
  state_dir="$(remote_fixture_existing_state_dir "$state_dir")" || return 1
  local state_file="$state_dir/startup-state.json"
  local owner_file="$state_dir/.fixture-owner"
  remote_fixture_validate_startup_state "$state_dir" || return 1
  [[ -f "$owner_file" && ! -L "$owner_file" ]] || {
    remote_fixture_fail "fixture owner marker is missing or unsafe for startup state"
    return 1
  }
  local fixture_id backend_pid backend_token tunnel_pid tunnel_token container_name
  fixture_id="$(remote_fixture_json_field "$state_file" fixture_id)"
  [[ "$(tr -d '\r\n' <"$owner_file")" == "$fixture_id" ]] || {
    remote_fixture_fail "fixture owner marker does not match startup state"
    return 1
  }
  backend_pid="$(remote_fixture_json_field "$state_file" backend_pid)"
  backend_token="$(remote_fixture_json_field "$state_file" backend_process_token)"
  tunnel_pid="$(remote_fixture_json_field "$state_file" tunnel_pid)"
  tunnel_token="$(remote_fixture_json_field "$state_file" tunnel_process_token)"
  container_name="$(remote_fixture_json_field "$state_file" container_name)"
  local pid token label
  for label in backend tunnel; do
    if [[ "$label" == backend ]]; then pid="$backend_pid"; token="$backend_token"; else pid="$tunnel_pid"; token="$tunnel_token"; fi
    if [[ -n "$pid" ]] && remote_fixture_pid_active "$pid" \
      && ! remote_fixture_live_pid_matches_token "$pid" "$token"; then
      remote_fixture_fail "startup state does not own live $label PID $pid"
      return 1
    fi
  done
  if [[ -n "$container_name" ]]; then
    local presence
    if remote_fixture_container_presence "$container_name"; then presence=0; else presence=$?; fi
    if [[ "$presence" == 0 ]]; then
      remote_fixture_verify_container_owner "$container_name" "$fixture_id" || return 1
    elif [[ "$presence" == 2 ]]; then
      return 1
    fi
  fi

  local cleanup_failed=0 secret_path
  for secret_path in "$state_dir/.username" "$state_dir/.password" "$state_dir/.auth-secret" \
    "$state_dir/.auth-pass" "$state_dir/password-hasher.stderr.log" "$state_dir/.backend.env" \
    "$state_dir/credentials.json" "$state_dir/fixture-env.json"; do
    [[ ! -e "$secret_path" && ! -L "$secret_path" ]] || rm -f -- "$secret_path" || cleanup_failed=1
  done
  remote_fixture_stop_pid tunnel "$tunnel_pid" "$tunnel_token" || cleanup_failed=1
  remote_fixture_stop_pid backend "$backend_pid" "$backend_token" || cleanup_failed=1
  if [[ -n "$container_name" ]]; then
    if remote_fixture_container_presence "$container_name"; then
      remote_fixture_verify_container_owner "$container_name" "$fixture_id" \
        && docker rm --force "$container_name" >/dev/null || cleanup_failed=1
    else
      presence=$?
      [[ "$presence" == 1 ]] || cleanup_failed=1
    fi
  fi
  if [[ "$cleanup_failed" == 1 ]]; then
    remote_fixture_fail "startup recovery failed; ownership state was preserved"
    return 1
  fi
  remote_fixture_remove_startup_admission "$state_dir"
  remote_fixture_remove_startup_state "$state_dir"
  rm -f -- "$owner_file"
  return 0
}

remote_fixture_cancel_owned_state() {
  local state_dir="$1"
  [[ -e "$state_dir" || -L "$state_dir" ]] || return 0
  state_dir="$(remote_fixture_existing_state_dir "$state_dir")" || return 1
  local final_state="$state_dir/fixture-state.json"
  if [[ -e "$final_state" || -L "$final_state" ]]; then
    stop_fixture --state-dir "$state_dir" || return 1
    remote_fixture_remove_startup_state "$state_dir"
    return 0
  fi
  if [[ -e "$state_dir/startup-state.json" || -L "$state_dir/startup-state.json" ]]; then
    remote_fixture_recover_startup_state "$state_dir"
    return
  fi
  if [[ -e "$state_dir/.fixture-owner" || -L "$state_dir/.fixture-owner" ]]; then
    remote_fixture_fail "fixture owner marker exists without valid ownership state"
    return 1
  fi
  return 0
}
