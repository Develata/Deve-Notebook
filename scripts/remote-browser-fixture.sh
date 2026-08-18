#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-json.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-json.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-supervisor.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-supervisor.sh"

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

start_fixture_worker() {
  set -Eeuo pipefail
  local state_dir=""
  local expected_head=""
  local repo_root="$ROOT_DIR"
  local external_origin=""
  local external_head_proof_url=""
  local external_credentials=""
  local backend_executable=""
  local backend_head_file=""
  local backend_container_image=""
  local password_hasher=""
  local cloudflared_executable=""
  local backend_port=""
  local -a password_hasher_args=()

  while (($#)); do
    case "$1" in
      --state-dir) state_dir="${2:-}"; shift 2 ;;
      --expected-head) expected_head="${2:-}"; shift 2 ;;
      --repo-root) repo_root="${2:-}"; shift 2 ;;
      --external-origin) external_origin="${2:-}"; shift 2 ;;
      --external-head-proof-url) external_head_proof_url="${2:-}"; shift 2 ;;
      --external-credentials-file) external_credentials="${2:-}"; shift 2 ;;
      --backend-executable) backend_executable="${2:-}"; shift 2 ;;
      --backend-head-file) backend_head_file="${2:-}"; shift 2 ;;
      --backend-container-image) backend_container_image="${2:-}"; shift 2 ;;
      --password-hasher) password_hasher="${2:-}"; shift 2 ;;
      --password-hasher-arg) password_hasher_args+=("${2:-}"); shift 2 ;;
      --backend-port) backend_port="${2:-}"; shift 2 ;;
      --cloudflared-executable) cloudflared_executable="${2:-}"; shift 2 ;;
      *) usage ;;
    esac
  done

  [[ -n "$state_dir" && -n "$expected_head" ]] || usage
  if [[ -n "$external_origin" || -n "$external_head_proof_url" || -n "$external_credentials" ]]; then
    [[ -n "$external_origin" && -n "$external_head_proof_url" && -n "$external_credentials" ]] || usage
    [[ -z "$backend_executable" && -z "$backend_container_image" && -z "$password_hasher" ]] || usage
  else
    [[ -z "$backend_executable" || -z "$backend_container_image" ]] || usage
    [[ -n "$backend_executable" || -n "$backend_container_image" ]] || usage
  fi
  remote_fixture_require_command git
  remote_fixture_require_command node
  remote_fixture_require_command curl
  remote_fixture_assert_expected_head "$repo_root" "$expected_head"
  state_dir="$(remote_fixture_canonical_dir "$state_dir")"
  local state_file="$state_dir/fixture-state.json"
  local environment_file="$state_dir/fixture-env.json"
  local credentials_file="$state_dir/credentials.json"
  local owner_file="$state_dir/.fixture-owner"
  if [[ -e "$state_file" || -e "$owner_file" ]]; then
    remote_fixture_fail "fixture state already exists; stop or remove the prior fixture first"
    return 1
  fi

  local fixture_id="$(remote_fixture_random_hex 16)"
  local source_kind=""
  local https_origin=""
  local backend_pid=""
  local backend_token=""
  local tunnel_pid=""
  local tunnel_token=""
  local container_name=""
  local start_complete=0
  local username_file="$state_dir/.username"
  local password_file="$state_dir/.password"
  local auth_secret_file="$state_dir/.auth-secret"
  local auth_pass_file="$state_dir/.auth-pass"
  local password_hasher_stderr_file="$state_dir/password-hasher.stderr.log"
  local docker_env_file="$state_dir/.backend.env"

  cleanup_failed_start() {
    local original_status="${1:-1}"
    [[ "$start_complete" == "1" ]] && return "$original_status"
    trap - ERR RETURN INT TERM
    set +e
    local cleanup_failed=0

    # Remove ephemeral secret duplicates before any potentially failing process
    # cleanup. The owned credentials copy is also unusable after failed start.
    local secret_path
    for secret_path in "$username_file" "$password_file" "$auth_secret_file" "$auth_pass_file" \
      "$password_hasher_stderr_file" \
      "$docker_env_file" "$credentials_file" "$environment_file"; do
      if [[ -e "$secret_path" || -L "$secret_path" ]]; then
        rm -f -- "$secret_path" || cleanup_failed=1
      fi
    done

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
      [[ "$container_presence" == "1" ]] || cleanup_failed=1
    fi

    if [[ "$cleanup_failed" == "1" ]]; then
      FIXTURE_ID="$fixture_id" EXPECTED_HEAD="$expected_head" SOURCE_KIND="$source_kind" \
        HTTPS_ORIGIN="$https_origin" CREDENTIALS_FILE="$credentials_file" ENVIRONMENT_FILE="$environment_file" \
        BACKEND_PID="$backend_pid" BACKEND_TOKEN="$backend_token" TUNNEL_PID="$tunnel_pid" TUNNEL_TOKEN="$tunnel_token" \
        CONTAINER_NAME="$container_name" remote_fixture_write_state "$state_file" || true
      remote_fixture_fail "startup failed and at least one owned resource survived cleanup; ownership state was preserved" || true
    else
      rm -f -- "$state_file" "$owner_file"
    fi
    exit "$original_status"
  }
  # ERR covers command failures while this local ownership scope is active;
  # RETURN covers explicit non-zero returns without deferring cleanup to EXIT.
  trap 'cleanup_failed_start $?' ERR RETURN
  trap 'cleanup_failed_start 130' INT
  trap 'cleanup_failed_start 143' TERM
  if ! kill -s USR1 -- "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}" 2>/dev/null; then
    remote_fixture_fail "startup supervisor disappeared before ownership admission"
    return 1
  fi
  printf '%s\n' "$fixture_id" >"$owner_file"
  chmod 0600 "$owner_file"

  if [[ -n "$external_origin" || -n "$external_head_proof_url" || -n "$external_credentials" ]]; then
    remote_fixture_assert_https_origin "$external_origin"
    if ! node -e 'const origin=new URL(process.argv[1]);const proof=new URL(process.argv[2]);if(origin.protocol!=="https:"||proof.protocol!=="https:"||origin.origin!==proof.origin)process.exit(1);' "$external_origin" "$external_head_proof_url"; then
      remote_fixture_fail "external HEAD proof URL must use the RemoteBrowser HTTPS origin"
      return 1
    fi
    local observed_external_head
    observed_external_head="$(curl --fail --silent --show-error --max-time 15 --max-redirs 0 "$external_head_proof_url" | tr -d '\r\n')"
    if [[ "${observed_external_head,,}" != "${expected_head,,}" ]]; then
      remote_fixture_fail "external backend HEAD proof does not match expected HEAD"
      return 1
    fi
    if [[ ! -f "$external_credentials" || -L "$external_credentials" ]]; then
      remote_fixture_fail "external credentials must be a regular non-symlink JSON file"
      return 1
    fi
    if ! node - "$external_credentials" "$credentials_file" <<'NODE'
const fs = require("fs");
const value = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const forbidden = /[\0\r\n]/;
if (typeof value.username !== "string" || !value.username || forbidden.test(value.username) ||
    typeof value.password !== "string" || !value.password || forbidden.test(value.password)) {
  process.exit(1);
}
fs.writeFileSync(process.argv[3], `${JSON.stringify({
  username: value.username,
  password: value.password,
  auth_secret: null,
}, null, 2)}\n`, { mode: 0o600 });
NODE
    then
      remote_fixture_fail "external credentials JSON requires non-empty username and password"
      return 1
    fi
    chmod 0600 "$credentials_file"
    https_origin="$external_origin"
    source_kind="external"
    curl --fail --silent --show-error --max-time 10 "$https_origin/api/node/role" >/dev/null
  else
    if [[ -z "$password_hasher" || ! -x "$password_hasher" ]]; then
      remote_fixture_fail "internal fixture requires an executable --password-hasher"
      return 1
    fi
    printf 'deve-ci-%s\n' "$(remote_fixture_random_hex 8)" >"$username_file"
    remote_fixture_random_hex 24 >"$password_file"; printf '\n' >>"$password_file"
    remote_fixture_random_hex 48 >"$auth_secret_file"; printf '\n' >>"$auth_secret_file"
    chmod 0600 "$username_file" "$password_file" "$auth_secret_file"
    remote_fixture_run_bounded "password hasher" 30 65536 \
      "$auth_pass_file" "$password_hasher_stderr_file" -- \
      "$password_hasher" "${password_hasher_args[@]}" --password-file "$password_file"
    if ! grep -Eq '^\$argon2id\$[^[:space:]]+$' "$auth_pass_file"; then
      remote_fixture_fail "password hasher did not emit one Argon2id PHC string"
      return 1
    fi
    rm -f -- "$password_hasher_stderr_file"
    remote_fixture_write_credentials "$credentials_file" "$username_file" "$password_file" "$auth_secret_file"

    local port="${backend_port:-$(remote_fixture_find_free_port)}"
    if [[ ! "$port" =~ ^[0-9]+$ || "$port" -lt 1024 || "$port" -gt 65535 ]]; then
      remote_fixture_fail "backend port must be in 1024..65535"
      return 1
    fi
    local runtime_dir="$state_dir/runtime"
    mkdir -p -- "$runtime_dir/ledger" "$runtime_dir/notes"
    local username password auth_secret auth_pass
    username="$(<"$username_file")"
    password="$(<"$password_file")"
    auth_secret="$(<"$auth_secret_file")"
    auth_pass="$(<"$auth_pass_file")"
    unset password

    if [[ -n "$backend_container_image" ]]; then
      source_kind="container"
      remote_fixture_require_command docker
      local image_head
      image_head="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$backend_container_image")"
      if [[ "${image_head,,}" != "${expected_head,,}" ]]; then
        remote_fixture_fail "candidate image revision label does not match expected HEAD"
        return 1
      fi
      container_name="deve-remote-fixture-${fixture_id:0:12}"
      printf 'AUTH_USER=%s\nAUTH_PASS=%s\nAUTH_SECRET=%s\n' "$username" "$auth_pass" "$auth_secret" >"$docker_env_file"
      chmod 0600 "$docker_env_file"
      docker run --detach --name "$container_name" \
        --label "deve.remote-fixture-id=$fixture_id" \
        --publish "127.0.0.1:$port:3001" \
        --env-file "$docker_env_file" \
        --volume "$runtime_dir/ledger:/data/ledger" --volume "$runtime_dir/notes:/notes" \
        "$backend_container_image" >/dev/null
      rm -f -- "$docker_env_file"
    else
      source_kind="executable"
      if [[ ! -x "$backend_executable" || ! -f "$backend_head_file" || -L "$backend_head_file" ]]; then
        remote_fixture_fail "executable source requires executable binary and regular --backend-head-file"
        return 1
      fi
      if [[ "$(tr -d '\r\n' <"$backend_head_file")" != "$expected_head" ]]; then
        remote_fixture_fail "backend build HEAD proof does not match expected HEAD"
        return 1
      fi
      local -a backend_args=(serve --port '{port}' --loopback-only)
      local index
      for index in "${!backend_args[@]}"; do
        backend_args[$index]="${backend_args[$index]//\{port\}/$port}"
        backend_args[$index]="${backend_args[$index]//\{data_dir\}/$runtime_dir}"
      done
      (
        cd -- "$runtime_dir"
        export AUTH_USER="$username" AUTH_PASS="$auth_pass" AUTH_SECRET="$auth_secret"
        export DEVE_ENV=production DEVE_LEDGER_DIR="$runtime_dir/ledger"
        remote_fixture_run_bounded "exact-HEAD backend init" 60 4194304 \
          "$state_dir/backend-init.stdout.log" "$state_dir/backend-init.stderr.log" -- \
          "$backend_executable" init --repo default --projection-base "$runtime_dir/notes" --path "$runtime_dir"
      )
      (
        cd -- "$runtime_dir"
        export AUTH_USER="$username" AUTH_PASS="$auth_pass" AUTH_SECRET="$auth_secret"
        export DEVE_ENV=production DEVE_LEDGER_DIR="$runtime_dir/ledger"
        exec "$backend_executable" "${backend_args[@]}"
      ) >"$state_dir/backend.stdout.log" 2>"$state_dir/backend.stderr.log" &
      backend_pid="$!"
      backend_token="$(remote_fixture_process_token "$backend_pid")"
    fi
    rm -f -- "$username_file" "$password_file" "$auth_secret_file" "$auth_pass_file"

    local backend_health="http://127.0.0.1:$port/api/node/role"
    if [[ -n "$container_name" ]]; then
      local attempt
      for attempt in $(seq 1 120); do
        curl --fail --silent --show-error --max-time 2 "$backend_health" >/dev/null 2>&1 && break
        if ! docker inspect "$container_name" >/dev/null 2>&1; then
          remote_fixture_fail "backend container exited before health check"
          return 1
        fi
        sleep 0.25
      done
      curl --fail --silent --show-error --max-time 2 "$backend_health" >/dev/null
    else
      remote_fixture_wait_http "$backend_health" "$backend_pid" "$state_dir/backend.stderr.log"
    fi

    local cloudflared
    cloudflared="$(remote_fixture_install_cloudflared "$state_dir" "$cloudflared_executable")"
    "$cloudflared" tunnel --no-autoupdate --protocol http2 \
      --url "http://127.0.0.1:$port" \
      >"$state_dir/cloudflared.stdout.log" 2>"$state_dir/cloudflared.stderr.log" &
    tunnel_pid="$!"
    tunnel_token="$(remote_fixture_process_token "$tunnel_pid")"
    https_origin="$(remote_fixture_wait_tunnel_origin "$tunnel_pid" "$state_dir/cloudflared.stdout.log" "$state_dir/cloudflared.stderr.log")"
    remote_fixture_wait_tunnel_http \
      "$https_origin/api/node/role" "$tunnel_pid" "$tunnel_token" \
      "$state_dir/cloudflared.stderr.log"
  fi

  remote_fixture_write_environment "$environment_file" "$https_origin" "$credentials_file" "$state_file"
  FIXTURE_ID="$fixture_id" EXPECTED_HEAD="$expected_head" SOURCE_KIND="$source_kind" \
    HTTPS_ORIGIN="$https_origin" CREDENTIALS_FILE="$credentials_file" ENVIRONMENT_FILE="$environment_file" \
    BACKEND_PID="$backend_pid" BACKEND_TOKEN="$backend_token" TUNNEL_PID="$tunnel_pid" TUNNEL_TOKEN="$tunnel_token" \
    CONTAINER_NAME="$container_name" remote_fixture_write_state "$state_file"
  start_complete=1
  trap - ERR RETURN INT TERM
  printf '%s\n' "$environment_file"
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
  if [[ -n "$backend_pid" ]] && remote_fixture_pid_active "$backend_pid"; then
    remote_fixture_fail "owned backend process survived cleanup"
    cleanup_failed=1
  fi
  if [[ -n "$tunnel_pid" ]] && remote_fixture_pid_active "$tunnel_pid"; then
    remote_fixture_fail "owned tunnel process survived cleanup"
    cleanup_failed=1
  fi
  if [[ "$cleanup_failed" == "1" ]]; then
    remote_fixture_fail "one or more fixture resources survived cleanup; ownership state was preserved"
    return 1
  fi
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
    trap - EXIT INT TERM
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
  start) shift; start_fixture "$@" ;;
  stop) shift; stop_fixture "$@" ;;
  run) shift; run_fixture "$@" ;;
  *) usage ;;
esac
