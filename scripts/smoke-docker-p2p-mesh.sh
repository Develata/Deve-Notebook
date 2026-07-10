#!/usr/bin/env bash
set -euo pipefail

# REL-010 smoke: build the local Docker image, run two isolated servers, and
# verify static FullPeer /ws mesh admission plus handshake diagnostics.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/baseline-wrapper.sh
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
COMPOSE_FILE="${DEVE_DOCKER_P2P_MESH_COMPOSE_FILE:-$ROOT_DIR/docker-compose.mesh.yml}"
PROJECT="${DEVE_DOCKER_P2P_MESH_PROJECT:-deve-p2p-mesh-$$}"
PORT_A="${DEVE_DOCKER_P2P_MESH_A_PORT:-3111}"
PORT_B="${DEVE_DOCKER_P2P_MESH_B_PORT:-3112}"
REQUIRED="${DEVE_DOCKER_P2P_MESH_REQUIRED:-0}"
KEEP="${DEVE_DOCKER_P2P_MESH_KEEP:-0}"
DOCKER_BIN="${DEVE_DOCKER_BIN:-docker}"
DOCKER_BUILDKIT_MODE="${DEVE_DOCKER_P2P_MESH_BUILDKIT:-0}"
COMPOSE_DOCKER_CLI_BUILD_MODE="${DEVE_DOCKER_P2P_MESH_COMPOSE_DOCKER_CLI_BUILD:-0}"
COMPOSE_PARALLEL_LIMIT_MODE="${DEVE_DOCKER_P2P_MESH_COMPOSE_PARALLEL_LIMIT:-1}"
AUTH_SECRET="${DEVE_DOCKER_P2P_MESH_AUTH_SECRET:-deve_docker_p2p_mesh_secret_32_bytes_ok!!}"
AUTH_USER="${DEVE_DOCKER_P2P_MESH_AUTH_USER:-admin}"
AUTH_PASS="${DEVE_DOCKER_P2P_MESH_AUTH_PASS:-\$argon2id\$v=19\$m=65536,t=2,p=1\$c29tZXNhbHQ\$CTFhFdXPJO1aFaMaO6Mm5c8y7cJHAph8ArZWb2GRPPc}"
AUTH_PASSWORD="${DEVE_DOCKER_P2P_MESH_AUTH_PASSWORD:-password}"
REPO_ID="${DEVE_DOCKER_P2P_MESH_REPO_ID:-11111111-1111-1111-1111-111111111111}"
REPO_KEY="${DEVE_DOCKER_P2P_MESH_REPO_KEY:-deve_mesh_shared_repo_key_32!!!!}"
TOKEN_A="${DEVE_DOCKER_P2P_MESH_TOKEN_A:-deve_mesh_peer_a_token}"
TOKEN_B="${DEVE_DOCKER_P2P_MESH_TOKEN_B:-deve_mesh_peer_b_token}"
PEER_A_EXPECTED_ID="${DEVE_DOCKER_P2P_MESH_PEER_A_ID:-}"
PEER_B_EXPECTED_ID="${DEVE_DOCKER_P2P_MESH_PEER_B_ID:-}"
PYTHON_BIN="${DEVE_DOCKER_P2P_MESH_PYTHON_BIN:-}"
COOKIE_A="${TMPDIR:-/tmp}/deve-p2p-mesh-${PROJECT}-a.cookie"
COOKIE_B="${TMPDIR:-/tmp}/deve-p2p-mesh-${PROJECT}-b.cookie"
DELEGATED_SC_HEADER_VALUE=""

run_deve_baseline "$ROOT_DIR" "docker-smoke-preflight" "docker-p2p-mesh-smoke" "p2p-mesh"

if [[ ";${MSYS2_ARG_CONV_EXCL:-};" == *";*;"* || ";${MSYS2_ARG_CONV_EXCL:-};" == *";/data/ledger;"* ]]; then
  MSYS_ARG_CONV_EXCL="${MSYS2_ARG_CONV_EXCL:-}"
elif [ -n "${MSYS2_ARG_CONV_EXCL:-}" ]; then
  MSYS_ARG_CONV_EXCL="${MSYS2_ARG_CONV_EXCL};/data/ledger"
else
  MSYS_ARG_CONV_EXCL="/data/ledger"
fi

fail() {
  echo "docker-p2p-mesh-smoke: $*" >&2
  exit 1
}

skip() {
  echo "docker-p2p-mesh-smoke: skipped: $*"
  exit 0
}

docker_cmd() {
  command "$DOCKER_BIN" "$@"
}

docker_compose() {
  AUTH_SECRET="$AUTH_SECRET" \
  AUTH_USER="$AUTH_USER" \
  AUTH_PASS="$AUTH_PASS" \
  DEVE_DOCKER_P2P_MESH_REPO_ID="$REPO_ID" \
  DEVE_DOCKER_P2P_MESH_REPO_KEY="$REPO_KEY" \
  DEVE_DOCKER_P2P_MESH_TOKEN_A="$TOKEN_A" \
  DEVE_DOCKER_P2P_MESH_TOKEN_B="$TOKEN_B" \
  DEVE_DOCKER_P2P_MESH_PEER_A_ID="$PEER_A_EXPECTED_ID" \
  DEVE_DOCKER_P2P_MESH_PEER_B_ID="$PEER_B_EXPECTED_ID" \
  DEVE_DOCKER_P2P_MESH_A_PORT="$PORT_A" \
  DEVE_DOCKER_P2P_MESH_B_PORT="$PORT_B" \
  DOCKER_BUILDKIT="$DOCKER_BUILDKIT_MODE" \
  COMPOSE_DOCKER_CLI_BUILD="$COMPOSE_DOCKER_CLI_BUILD_MODE" \
  COMPOSE_PARALLEL_LIMIT="$COMPOSE_PARALLEL_LIMIT_MODE" \
  MSYS2_ARG_CONV_EXCL="$MSYS_ARG_CONV_EXCL" \
    docker_cmd compose -f "$COMPOSE_FILE" -p "$PROJECT" "$@"
}

curl_local() {
  curl --noproxy "127.0.0.1,localhost" "$@"
}

find_python() {
  if [[ -n "$PYTHON_BIN" ]]; then
    command -v "$PYTHON_BIN" >/dev/null 2>&1
    return
  fi
  if command -v python3 >/dev/null 2>&1; then
    PYTHON_BIN=python3
    return 0
  fi
  if command -v python >/dev/null 2>&1; then
    PYTHON_BIN=python
    return 0
  fi
  return 1
}

delegated_sc_header_value() {
  AUTH_SECRET="$AUTH_SECRET" "$PYTHON_BIN" - <<'PY'
import hashlib
import hmac
import os

secret = os.environ["AUTH_SECRET"].encode("utf-8")
transcript = b"deve-source-control-delegation:v1"
signature = hmac.new(secret, transcript, hashlib.sha256).hexdigest()
print(f"v1.{signature}")
PY
}

docker_bin_available() {
  if [[ "$DOCKER_BIN" == */* ]]; then
    [[ -x "$DOCKER_BIN" ]]
    return
  fi
  command -v "$DOCKER_BIN" >/dev/null 2>&1
}

require_or_skip() {
  if [[ "$REQUIRED" == "1" || "$REQUIRED" == "true" ]]; then
    fail "$1"
  fi
  skip "$1"
}

preflight_port() {
  local port="$1"
  if curl_local -fsS "http://127.0.0.1:${port}/api/node/role" >/dev/null 2>&1; then
    require_or_skip "host port $port already serves /api/node/role"
  fi
}

cleanup() {
  rm -f "$COOKIE_A" "$COOKIE_B" >/dev/null 2>&1 || true
  if [[ "$KEEP" == "1" || "$KEEP" == "true" ]]; then
    echo "docker-p2p-mesh-smoke: kept compose project '$PROJECT'"
    echo "docker-p2p-mesh-smoke: peer-a http://127.0.0.1:${PORT_A}/"
    echo "docker-p2p-mesh-smoke: peer-b http://127.0.0.1:${PORT_B}/"
    echo "docker-p2p-mesh-smoke: cleanup with: $DOCKER_BIN compose -f $COMPOSE_FILE -p $PROJECT down -v --remove-orphans"
    return
  fi
  docker_compose down -v --remove-orphans >/dev/null 2>&1 || true
}

diagnose() {
  local logs
  echo "docker-p2p-mesh-smoke: collecting compose diagnostics" >&2
  docker_compose ps >&2 || true
  logs="$(docker_compose logs --no-color 2>/dev/null || true)"
  if grep -qF "$TOKEN_A" <<<"$logs" || grep -qF "$TOKEN_B" <<<"$logs"; then
    echo "docker-p2p-mesh-smoke: compose logs suppressed because token material was detected" >&2
    return
  fi
  printf '%s\n' "$logs" >&2
}

wait_for_peer() {
  local port="$1"
  local status
  local login_status
  for _ in $(seq 1 90); do
    status="$(curl_local -fsS -o /dev/null -w '%{http_code}' "http://127.0.0.1:${port}/api/node/role" || true)"
    if [[ "$status" == "200" ]]; then
      login_status="$(
        curl_local -fsS -o /dev/null -w '%{http_code}' \
          -X POST "http://127.0.0.1:${port}/api/auth/login" \
          -H 'Content-Type: application/json' \
          --data "{\"username\":\"${AUTH_USER}\",\"password\":\"${AUTH_PASSWORD}\"}" \
          || true
      )"
      if [[ "$login_status" == "200" ]]; then
        return 0
      fi
    fi
    sleep 1
  done
  return 1
}

wait_for_mesh_handshake() {
  local logs
  for _ in $(seq 1 60); do
    logs="$(docker_compose logs --no-color 2>/dev/null || true)"
    if grep -q "P2P mesh connector handshake completed" <<<"$logs"; then
      if grep -qF "$TOKEN_A" <<<"$logs" || grep -qF "$TOKEN_B" <<<"$logs"; then
        fail "P2P token material appeared in compose logs"
      fi
      return 0
    fi
    sleep 1
  done
  return 1
}

mesh_handshake_count() {
  local service="$1"
  local logs
  logs="$(docker_compose logs --no-color "$service" 2>/dev/null || true)"
  grep -c "P2P mesh connector handshake completed" <<<"$logs" || true
}

mesh_connection_count() {
  local service="$1"
  local peer_id="$2"
  local logs
  logs="$(docker_compose logs --no-color "$service" 2>/dev/null || true)"
MESH_LOGS="$logs" REPO_ID="$REPO_ID" "$PYTHON_BIN" - "$peer_id" <<'PY'
import os
import re
import sys

peer_id = sys.argv[1]
repo_id = os.environ["REPO_ID"]
ansi = re.compile(r"\x1b\[[0-9;]*m")
hello_pattern = re.compile(
    r"Handling SyncHello from " + re.escape(peer_id) + r" for repo " + re.escape(repo_id)
)
authenticated = f'authenticated_peer_id="{peer_id}"'

count = 0
for raw_line in os.environ["MESH_LOGS"].splitlines():
    line = ansi.sub("", raw_line)
    if "P2P mesh connector handshake completed" in line and authenticated in line:
        count += 1
    elif hello_pattern.search(line):
        count += 1
print(count)
PY
}

server_peer_id_from_logs() {
  local service="$1"
  local logs
  logs="$(docker_compose logs --no-color "$service" 2>/dev/null || true)"
MESH_LOGS="$logs" "$PYTHON_BIN" - <<'PY'
import os
import re

ansi = re.compile(r"\x1b\[[0-9;]*m")
pattern = re.compile(r"Server PeerID: ([^ ]+)")
for line in reversed(os.environ["MESH_LOGS"].splitlines()):
    line = ansi.sub("", line)
    match = pattern.search(line)
    if match:
        print(match.group(1).strip('"'))
        raise SystemExit(0)
raise SystemExit(1)
PY
}

wait_for_server_peer_id() {
  local service="$1"
  local peer_id
  for _ in $(seq 1 60); do
    peer_id="$(server_peer_id_from_logs "$service" || true)"
    if [[ -n "$peer_id" ]]; then
      printf '%s\n' "$peer_id"
      return 0
    fi
    sleep 1
  done
  return 1
}

ensure_static_peer_ids() {
  local discovered_a
  local discovered_b
  discovered_a="$(wait_for_server_peer_id peer-a)" || return 1
  discovered_b="$(wait_for_server_peer_id peer-b)" || return 1

  if [[ -n "${DEVE_DOCKER_P2P_MESH_PEER_A_ID:-}" && "$PEER_A_EXPECTED_ID" != "$discovered_a" ]]; then
    fail "configured peer-a expected id ${PEER_A_EXPECTED_ID} did not match actual ${discovered_a}"
  fi
  if [[ -n "${DEVE_DOCKER_P2P_MESH_PEER_B_ID:-}" && "$PEER_B_EXPECTED_ID" != "$discovered_b" ]]; then
    fail "configured peer-b expected id ${PEER_B_EXPECTED_ID} did not match actual ${discovered_b}"
  fi
  if [[ "$PEER_A_EXPECTED_ID" == "$discovered_a" && "$PEER_B_EXPECTED_ID" == "$discovered_b" ]]; then
    return 0
  fi

  PEER_A_EXPECTED_ID="$discovered_a"
  PEER_B_EXPECTED_ID="$discovered_b"
  echo "docker-p2p-mesh-smoke: discovered static peer ids peer-a=${PEER_A_EXPECTED_ID} peer-b=${PEER_B_EXPECTED_ID}"
  docker_compose up -d --force-recreate --no-build >/dev/null
  wait_for_peer "$PORT_A" && wait_for_peer "$PORT_B"
}

login_peer() {
  local port="$1"
  local cookie="$2"
  curl_local -fsS -c "$cookie" -b "$cookie" \
    -X POST "http://127.0.0.1:${port}/api/auth/login" \
    -H 'Content-Type: application/json' \
    --data "{\"username\":\"${AUTH_USER}\",\"password\":\"${AUTH_PASSWORD}\"}" \
    >/dev/null
}

json_doc_id_for_path() {
  local docs_json="$1"
  local doc_path="$2"
  DOCS_JSON="$docs_json" "$PYTHON_BIN" - "$doc_path" <<'PY'
import json
import os
import sys

target = sys.argv[1]
data = json.loads(os.environ["DOCS_JSON"])
for item in data:
    if isinstance(item, list) and len(item) >= 2 and item[1] == target:
        print(item[0])
        raise SystemExit(0)
    if isinstance(item, dict) and item.get("path") == target:
        print(item.get("doc_id") or item.get("docId") or item.get("id"))
        raise SystemExit(0)
raise SystemExit(1)
PY
}

json_has_path() {
  local changes_json="$1"
  local doc_path="$2"
  CHANGES_JSON="$changes_json" "$PYTHON_BIN" - "$doc_path" <<'PY'
import json
import os
import sys

target = sys.argv[1]
data = json.loads(os.environ["CHANGES_JSON"])
for item in data:
    if isinstance(item, dict) and item.get("path") == target:
        raise SystemExit(0)
    if isinstance(item, list) and target in item:
        raise SystemExit(0)
raise SystemExit(1)
PY
}

authenticated_peer_id_from_logs() {
  local service="$1"
  local label="$2"
  local logs
  logs="$(docker_compose logs --no-color "$service" 2>/dev/null || true)"
MESH_LOGS="$logs" REPO_ID="$REPO_ID" "$PYTHON_BIN" - "$label" <<'PY'
import os
import re
import sys

label = sys.argv[1]
repo_id = os.environ["REPO_ID"]
ansi = re.compile(r"\x1b\[[0-9;]*m")
pattern = re.compile(r"authenticated_peer_id=([^ ]+)")
for line in reversed(os.environ["MESH_LOGS"].splitlines()):
    line = ansi.sub("", line)
    if "P2P mesh connector handshake completed" not in line:
        continue
    if f"peer_label={label}" not in line:
        continue
    match = pattern.search(line)
    if match:
        print(match.group(1).strip('"'))
        raise SystemExit(0)
# The two-peer smoke has exactly one remote per service. If the outbound
# connector keeps the socket open, the completion log can lag behind inbound
# admission; the SyncHello log is still emitted after peer authentication.
hello_pattern = re.compile(
    r"Handling SyncHello from ([^ ]+) for repo " + re.escape(repo_id)
)
for line in reversed(os.environ["MESH_LOGS"].splitlines()):
    line = ansi.sub("", line)
    match = hello_pattern.search(line)
    if match:
        print(match.group(1).strip('"'))
        raise SystemExit(0)
raise SystemExit(1)
PY
}

wait_for_pending_path() {
  local port="$1"
  local cookie="$2"
  local doc_path="$3"
  local status_json
  for _ in $(seq 1 45); do
    status_json="$(
      curl_local -fsS -b "$cookie" \
        "http://127.0.0.1:${port}/api/sc/status?scope_nonce=1&repo_id=${REPO_ID}" \
        || true
    )"
    if [[ -n "$status_json" ]] && json_has_path "$status_json" "$doc_path"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

stage_apply_and_commit_path() {
  local port="$1"
  local doc_path="$2"
  local message="$3"
  local stage_output
  local apply_output
  local commit_output
  if ! stage_output="$(curl_local -fsS \
    -X POST "http://127.0.0.1:${port}/api/delegated/sc/stage-pending" \
    -H "x-deve-source-control-delegation: ${DELEGATED_SC_HEADER_VALUE}" \
    -H 'Content-Type: application/json' \
    --data "{\"scope_nonce\":1,\"repo_id\":\"${REPO_ID}\",\"path\":\"${doc_path}\"}" \
    2>&1)"; then
    printf '%s\n' "$stage_output" >&2
    return 1
  fi
  if ! apply_output="$(curl_local -fsS \
    -X POST "http://127.0.0.1:${port}/api/delegated/sc/apply-external-changes" \
    -H "x-deve-source-control-delegation: ${DELEGATED_SC_HEADER_VALUE}" \
    -H 'Content-Type: application/json' \
    --data "{\"scope_nonce\":1,\"repo_id\":\"${REPO_ID}\"}" \
    2>&1)"; then
    printf '%s\n' "$apply_output" >&2
    return 1
  fi
  if ! commit_output="$(curl_local -fsS \
    -X POST "http://127.0.0.1:${port}/api/delegated/sc/commit" \
    -H "x-deve-source-control-delegation: ${DELEGATED_SC_HEADER_VALUE}" \
    -H 'Content-Type: application/json' \
    --data "{\"scope_nonce\":1,\"repo_id\":\"${REPO_ID}\",\"message\":\"${message}\"}" \
    2>&1)"; then
    printf '%s\n' "$commit_output" >&2
    return 1
  fi
}

create_peer_a_fixture() {
  local fixture_id
  fixture_id="$(date +%s)-$$"
  local doc_path="p2p-mesh/${fixture_id}.md"
  local doc_content="p2p-mesh-content-${fixture_id}"

  local workspace_root="/notes/default--${REPO_ID}"
  docker_compose exec -T peer-a sh -c \
    "mkdir -p '${workspace_root}/p2p-mesh' && printf '%s\n' '${doc_content}' > '${workspace_root}/${doc_path}'"

  if ! wait_for_pending_path "$PORT_A" "$COOKIE_A" "$doc_path"; then
    diagnose
    fail "peer-a source-control status did not observe pending path ${doc_path}"
  fi
  stage_apply_and_commit_path "$PORT_A" "$doc_path" "docker p2p mesh smoke ${fixture_id}"

  local docs_json
  local doc_id
  docs_json="$(curl_local -fsS -b "$COOKIE_A" "http://127.0.0.1:${PORT_A}/api/repo/docs?repo_id=${REPO_ID}")"
  doc_id="$(json_doc_id_for_path "$docs_json" "$doc_path")" || fail "committed doc ${doc_path} was not listed by peer-a"

  printf '%s\t%s\t%s\n' "$doc_path" "$doc_content" "$doc_id"
}

wait_for_remote_ops_handled() {
  local peer_id="$1"
  local logs
  for _ in $(seq 1 90); do
    logs="$(docker_compose logs --no-color peer-b 2>/dev/null || true)"
    if REMOTE_LOGS="$logs" REPO_ID="$REPO_ID" "$PYTHON_BIN" - "$peer_id" <<'PY'
import os
import re
import sys

peer_id = sys.argv[1]
repo_id = os.environ["REPO_ID"]
ansi = re.compile(r"\x1b\[[0-9;]*m")
handled_pattern = re.compile(
    r"Handled [1-9][0-9]* remote ops from "
    + re.escape(peer_id)
    + r" for repo "
    + re.escape(repo_id)
)
authenticated = f'authenticated_peer_id="{peer_id}"'
applied_pattern = re.compile(r"applied_pushes=([0-9]+)")

for raw_line in os.environ["REMOTE_LOGS"].splitlines():
    line = ansi.sub("", raw_line)
    if handled_pattern.search(line):
        raise SystemExit(0)
    if "P2P mesh connector handshake completed" not in line:
        continue
    if authenticated not in line:
        continue
    match = applied_pattern.search(line)
    if match and int(match.group(1)) > 0:
        raise SystemExit(0)
raise SystemExit(1)
PY
    then
      return 0
    fi
    sleep 1
  done
  return 1
}

run_offline_shadow_check() {
  local peer_id="$1"
  local doc_id="$2"
  local expected_content="$3"
  local output
  docker_compose stop peer-b >/dev/null
  if ! output="$(docker_compose run --rm --no-deps --entrypoint deve_cli peer-b verify-p2p \
    --live-ledger-dir /data/ledger \
    --repo-id "$REPO_ID" \
    --peer-id "$peer_id" \
    --doc-id "$doc_id" \
    --contains "$expected_content" \
    --local-must-not-contain "$expected_content" \
    2>&1)"; then
    printf '%s\n' "$output" >&2
    return 1
  fi
  printf '%s\n' "$output"
}

restart_peer_b_and_wait_reconnect() {
  local previous_connections="$1"
  local peer_id="$2"
  local current_connections
  docker_compose up -d peer-b >/dev/null
  if ! wait_for_peer "$PORT_B"; then
    return 1
  fi
  for _ in $(seq 1 60); do
    current_connections="$(mesh_connection_count peer-b "$peer_id")"
    if (( current_connections > previous_connections )); then
      return 0
    fi
    sleep 1
  done
  return 1
}

assert_no_token_material_in_logs_or_persisted_data() {
  local phase="$1"
  local logs
  local service
  local scan_status
  logs="$(docker_compose logs --no-color 2>/dev/null || true)"
  if grep -qF "$TOKEN_A" <<<"$logs" || grep -qF "$TOKEN_B" <<<"$logs"; then
    fail "P2P token material appeared in compose logs during ${phase} hygiene check"
  fi

  for service in peer-a peer-b; do
    if docker_compose exec -T "$service" sh -c \
      'grep -R -F -e "$1" -e "$2" /data /notes >/dev/null 2>&1' \
      sh "$TOKEN_A" "$TOKEN_B"; then
      fail "P2P token material appeared in persisted data/projection files for ${service}"
    else
      scan_status=$?
      if [[ "$scan_status" -ne 1 ]]; then
        fail "P2P token persisted data/projection scan failed for ${service} (status ${scan_status})"
      fi
    fi
  done
}

docker_bin_available || require_or_skip "docker command not found"
command -v curl >/dev/null 2>&1 || require_or_skip "curl command not found"
find_python || require_or_skip "python3/python command not found"
DELEGATED_SC_HEADER_VALUE="$(delegated_sc_header_value)" || fail "could not compute delegated source-control header"
[[ ${#REPO_KEY} -eq 32 ]] || fail "DEVE_DOCKER_P2P_MESH_REPO_KEY must be exactly 32 ASCII bytes"
docker_cmd info >/dev/null 2>&1 || require_or_skip "docker daemon is not reachable"
[[ -f "$COMPOSE_FILE" ]] || fail "compose file not found: $COMPOSE_FILE"
preflight_port "$PORT_A"
preflight_port "$PORT_B"

trap cleanup EXIT

docker_compose build peer-a
docker_compose up -d --no-build

if ! wait_for_peer "$PORT_A"; then
  diagnose
  fail "peer-a did not become login-ready at http://127.0.0.1:${PORT_A}"
fi
if ! wait_for_peer "$PORT_B"; then
  diagnose
  fail "peer-b did not become login-ready at http://127.0.0.1:${PORT_B}"
fi
if ! ensure_static_peer_ids; then
  diagnose
  fail "could not configure static expected peer ids"
fi
if ! wait_for_mesh_handshake; then
  diagnose
  fail "P2P mesh handshake did not complete"
fi

login_peer "$PORT_A" "$COOKIE_A"
login_peer "$PORT_B" "$COOKIE_B"

fixture="$(
  create_peer_a_fixture
)"
IFS=$'\t' read -r DOC_PATH DOC_CONTENT DOC_ID <<<"$fixture"
echo "docker-p2p-mesh-smoke: fixture ${DOC_PATH} doc_id=${DOC_ID}"
PEER_A_ID="$(authenticated_peer_id_from_logs peer-b peer-a)" || {
  diagnose
  fail "could not resolve peer-a authenticated peer id from peer-b logs"
}

if ! wait_for_remote_ops_handled "$PEER_A_ID"; then
  diagnose
  fail "peer-b did not handle peer-a remote ops"
fi

CONNECTIONS_BEFORE="$(mesh_connection_count peer-b "$PEER_A_ID")"
if ! run_offline_shadow_check "$PEER_A_ID" "$DOC_ID" "$DOC_CONTENT"; then
  diagnose
  fail "peer-b shadow repo did not contain peer-a content or local repo was polluted"
fi

if ! restart_peer_b_and_wait_reconnect "$CONNECTIONS_BEFORE" "$PEER_A_ID"; then
  diagnose
  fail "peer-b did not reconnect to the mesh after offline shadow verification"
fi

assert_no_token_material_in_logs_or_persisted_data "final"

echo "docker-p2p-mesh-smoke: ok"
