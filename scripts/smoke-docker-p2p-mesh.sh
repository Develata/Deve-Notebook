#!/usr/bin/env bash
set -euo pipefail

# REL-010 smoke: build the local Docker image, run two isolated servers, and
# verify static FullPeer /ws mesh admission plus handshake diagnostics.
# Cohesion exception (>500 lines): the ordered online/offline/gap/recovery
# journey shares one bounded fixture state machine. Bootstrap, cleanup, live
# shadow verification, and contract tests remain separate responsibility units.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/baseline-wrapper.sh
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
source "$ROOT_DIR/scripts/lib/docker-p2p-mesh-diagnostics.sh"
COMPOSE_FILE="${DEVE_DOCKER_P2P_MESH_COMPOSE_FILE:-$ROOT_DIR/docker-compose.mesh.yml}"
if [[ -n "${DEVE_ACCEPTANCE_PRODUCER_STATE_DIR:-}" ]]; then
  if [[ -n "${DEVE_DOCKER_P2P_MESH_PROJECT:-}" \
    || -n "${DEVE_DOCKER_P2P_MESH_STATE_FILE:-}" ]]; then
    echo "docker-p2p-mesh-smoke: receipt project/state override rejected" >&2
    exit 1
  fi
  if [[ -n "${DEVE_DOCKER_BIN:-}" && "$DEVE_DOCKER_BIN" != "docker" ]]; then
    echo "docker-p2p-mesh-smoke: receipt Docker binary override rejected" >&2
    exit 1
  fi
  state_digest="$(printf '%s' "$DEVE_ACCEPTANCE_PRODUCER_STATE_DIR" \
    | sha256sum | cut -c1-16)"
  PROJECT="deve-p2p-mesh-receipt-$state_digest"
  STATE_FILE="$DEVE_ACCEPTANCE_PRODUCER_STATE_DIR/docker-p2p-mesh/fixture-state"
else
  PROJECT="${DEVE_DOCKER_P2P_MESH_PROJECT:-deve-p2p-mesh-$$}"
  STATE_FILE="${DEVE_DOCKER_P2P_MESH_STATE_FILE:-${TMPDIR:-/tmp}/deve-docker-p2p-mesh-$$/docker-p2p-mesh/fixture-state}"
fi
PORT_A="${DEVE_DOCKER_P2P_MESH_A_PORT:-3111}"
PORT_B="${DEVE_DOCKER_P2P_MESH_B_PORT:-3112}"
REQUIRED="${DEVE_DOCKER_P2P_MESH_REQUIRED:-0}"
KEEP="${DEVE_DOCKER_P2P_MESH_KEEP:-0}"
SKIP_BUILD="${DEVE_DOCKER_P2P_MESH_SKIP_BUILD:-0}"
IMAGE="${DEVE_DOCKER_P2P_MESH_IMAGE:-${DEVE_RELEASE_CANDIDATE_IMAGE:-deve-p2p-mesh-smoke:local}}"
EXPECTED_IMAGE_ID="${DEVE_RELEASE_CANDIDATE_IMAGE_ID:-}"
INJECT_SEQUENCE_GAP="${DEVE_DOCKER_P2P_INJECT_SEQUENCE_GAP:-0}"
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
COOKIE_A="$(dirname -- "$STATE_FILE")/peer-a.cookie"
COOKIE_B="$(dirname -- "$STATE_FILE")/peer-b.cookie"
DELEGATED_SC_HEADER_VALUE=""

run_deve_baseline "$ROOT_DIR" "docker-smoke-preflight" "docker-p2p-mesh-smoke" "p2p-mesh"

MSYS_ARG_CONV_EXCL="${MSYS2_ARG_CONV_EXCL:-}"
if [[ ";${MSYS_ARG_CONV_EXCL};" != *";*;"* ]]; then
  for container_path in /data/ledger /notes; do
    if [[ ";${MSYS_ARG_CONV_EXCL};" != *";${container_path};"* ]]; then
      MSYS_ARG_CONV_EXCL="${MSYS_ARG_CONV_EXCL:+${MSYS_ARG_CONV_EXCL};}${container_path}"
    fi
  done
fi

fail() {
  echo "docker-p2p-mesh-smoke: $*" >&2
  exit 1
}

skip() {
  echo "docker-p2p-mesh-smoke: skipped: $*"
  exit 0
}

is_true() {
  [[ "$1" == "1" || "$1" == "true" ]]
}

docker_cmd() {
  command "$DOCKER_BIN" "$@"
}

verify_candidate_image() {
  [[ -n "$EXPECTED_IMAGE_ID" ]] || return 0
  local observed
  observed="$(docker_cmd image inspect --format '{{.Id}}' "$IMAGE")" \
    || fail "could not inspect candidate image identity: $IMAGE"
  [[ "$observed" == "$EXPECTED_IMAGE_ID" ]] \
    || fail "candidate image identity mismatch: expected=$EXPECTED_IMAGE_ID observed=$observed"
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
  DEVE_DOCKER_P2P_INJECT_SEQUENCE_GAP="$INJECT_SEQUENCE_GAP" \
  DEVE_DOCKER_P2P_MESH_IMAGE="$IMAGE" \
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

peer_a_workspace_root() {
  local locator_content
  locator_content="$(docker_compose exec -T peer-a \
    cat /data/ledger/.host/projection-locators.toml)" || return 1
  local workspace_root
  workspace_root="$(LOCATOR_CONTENT="$locator_content" REPO_ID="$REPO_ID" "$PYTHON_BIN" - <<'PY'
import os
import re

repo_id = os.environ["REPO_ID"]
content = os.environ["LOCATOR_CONTENT"]
records = []
current = None
for raw_line in content.splitlines():
    line = raw_line.strip().rstrip("\r")
    if line == "[[locators]]":
        current = {}
        records.append(current)
        continue
    if current is None:
        continue
    match = re.fullmatch(
        r"(repo_id|workspace_segment|projection_base_abs)\s*=\s*(['\"])([^'\"\r\n]+)\2",
        line,
    )
    if match:
        current[match.group(1)] = match.group(3)
matches = [record for record in records if record.get("repo_id") == repo_id]
if len(matches) != 1:
    raise SystemExit(f"expected exactly one locator for repo {repo_id}")
record = matches[0]
base = record.get("projection_base_abs")
segment = record.get("workspace_segment")
if base != "/notes":
    raise SystemExit(f"Docker smoke projection base must be /notes, observed {base!r}")
if not segment or not re.fullmatch(r"(?:[A-Za-z0-9._-]+--)?[0-9a-f-]+", segment):
    raise SystemExit(f"unsafe projection workspace segment: {segment!r}")
print(f"{base}/{segment}")
PY
  )" || return 1
  local identity_content
  identity_content="$(docker_compose exec -T peer-a sh -c \
    'test ! -L "$1" && test -f "$1" && cat "$1"' \
    _ "${workspace_root}/.notegit/identity.toml")" || return 1
  WORKSPACE_IDENTITY="$identity_content" REPO_ID="$REPO_ID" "$PYTHON_BIN" - <<'PY'
import os
import re

content = os.environ["WORKSPACE_IDENTITY"]
repo_id = os.environ["REPO_ID"]
if not re.search(r"^version\s*=\s*1\s*$", content, re.MULTILINE):
    raise SystemExit("workspace identity marker version is not 1")
match = re.search(r"^repo_id\s*=\s*(['\"])([^'\"\r\n]+)\1\s*$", content, re.MULTILINE)
if match is None or match.group(2) != repo_id:
    raise SystemExit("workspace identity marker RepoId mismatch")
PY
  printf '%s\n' "$workspace_root"
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
  if is_true "$KEEP"; then
    echo "docker-p2p-mesh-smoke: kept compose project '$PROJECT'"
    echo "docker-p2p-mesh-smoke: peer-a http://127.0.0.1:${PORT_A}/"
    echo "docker-p2p-mesh-smoke: peer-b http://127.0.0.1:${PORT_B}/"
    echo "docker-p2p-mesh-smoke: cleanup by rerunning this script with DEVE_DOCKER_P2P_MESH_PROJECT='$PROJECT' and DEVE_DOCKER_P2P_MESH_KEEP=0"
    return
  fi
  if ! DEVE_DOCKER_P2P_MESH_STATE_FILE="$STATE_FILE" \
      DEVE_DOCKER_BIN="$DOCKER_BIN" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh" >/dev/null; then
    echo "docker-p2p-mesh-smoke: warning: compose cleanup failed for project '$PROJECT'" >&2
    return 1
  fi
}

diagnose() {
  docker_p2p_mesh_diagnose
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
  local connections_a
  local connections_b
  for _ in $(seq 1 60); do
    connections_a="$(mesh_connection_count peer-a "$PEER_B_EXPECTED_ID")"
    connections_b="$(mesh_connection_count peer-b "$PEER_A_EXPECTED_ID")"
    if (( connections_a > 0 && connections_b > 0 )); then
      return 0
    fi
    sleep 1
  done
  return 1
}

mesh_connection_count() {
  local service="$1"
  local peer_id="$2"
  docker_stream_parse_command mesh-count "$peer_id" "$REPO_ID" \
    --token "$TOKEN_A" --token "$TOKEN_B" -- \
    docker_compose logs --no-color "$service"
}

# Isolated parser contract used by the static shell/Node regression. Production
# calls use mesh_connection_count so the Docker producer status is checked.
count_mesh_evidence_in_logs() {
  local peer_id="${1:-}"
  local repo_id="${2:-}"
  [[ -n "$peer_id" && -n "$repo_id" ]] || return 1
  docker_stream_parse_stdin mesh-count "$peer_id" "$repo_id"
}

server_peer_id_from_logs() {
  local service="$1"
  docker_stream_parse_command server-peer-id \
    --token "$TOKEN_A" --token "$TOKEN_B" -- \
    docker_compose logs --no-color "$service"
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
  docker_stream_parse_command authenticated-peer "$label" "$REPO_ID" \
    --token "$TOKEN_A" --token "$TOKEN_B" -- \
    docker_compose logs --no-color "$service"
}

wait_for_pending_path() {
  local port="$1"
  local cookie="$2"
  local doc_path="$3"
  local status_json
  for _ in $(seq 1 45); do
    status_json="$(
      curl_local -fsS -b "$cookie" \
        "http://127.0.0.1:${port}/api/sc/pending?scope_nonce=1&repo_id=${REPO_ID}" \
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
  local doc_id="${4:-}"
  local stage_output
  local apply_output
  local commit_output
  local stage_body
  if [[ -n "$doc_id" ]]; then
    stage_body="{\"scope_nonce\":1,\"repo_id\":\"${REPO_ID}\",\"path\":\"${doc_path}\",\"doc_id\":\"${doc_id}\"}"
  else
    stage_body="{\"scope_nonce\":1,\"repo_id\":\"${REPO_ID}\",\"path\":\"${doc_path}\"}"
  fi
  if ! stage_output="$(curl_local -fsS \
    -X POST "http://127.0.0.1:${port}/api/delegated/sc/stage-pending" \
    -H "x-deve-source-control-delegation: ${DELEGATED_SC_HEADER_VALUE}" \
    -H 'Content-Type: application/json' \
    --data "$stage_body" \
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

  local workspace_root
  workspace_root="$(peer_a_workspace_root)" \
    || fail "could not resolve peer-a projection workspace for ${REPO_ID}"
  docker_compose exec -T peer-a sh -c \
    "mkdir -p '${workspace_root}/p2p-mesh' && printf '%s' '${doc_content}' > '${workspace_root}/${doc_path}'"

  if ! wait_for_pending_path "$PORT_A" "$COOKIE_A" "$doc_path"; then
    diagnose
    fail "peer-a source-control status did not observe pending path ${doc_path}"
  fi
  if ! stage_apply_and_commit_path "$PORT_A" "$doc_path" "docker p2p mesh smoke ${fixture_id}"; then
    fail "peer-a could not stage/apply/commit initial fixture ${doc_path}"
  fi

  local docs_json
  local doc_id
  docs_json="$(curl_local -fsS -b "$COOKIE_A" "http://127.0.0.1:${PORT_A}/api/repo/docs?repo_id=${REPO_ID}")"
  doc_id="$(json_doc_id_for_path "$docs_json" "$doc_path")" || fail "committed doc ${doc_path} was not listed by peer-a"

  printf '%s\t%s\t%s\n' "$doc_path" "$doc_content" "$doc_id"
}

wait_for_remote_ops_handled() {
  local peer_id="$1"
  for _ in $(seq 1 90); do
    if docker_stream_parse_command remote-ops "$peer_id" "$REPO_ID" \
      --token "$TOKEN_A" --token "$TOKEN_B" -- \
      docker_compose logs --no-color peer-b; then
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_for_recovered_shadow() {
  local peer_id="$1"
  local doc_id="$2"
  local expected_content="$3"
  for _ in $(seq 1 9); do
    sleep 10
    docker_compose stop peer-b >/dev/null
    if run_offline_shadow_check "$peer_id" "$doc_id" "$expected_content"; then
      return 0
    fi
    docker_compose up -d peer-b >/dev/null
    wait_for_peer "$PORT_B" || return 1
  done
  return 1
}

arm_sequence_gap_fault() {
  docker_compose exec -T peer-a sh -c \
    'mkdir -p /data/ledger/.host/test-faults && : > /data/ledger/.host/test-faults/p2p-sequence-gap-arm'
  docker_compose exec -T peer-a sh -c \
    'case "$DEVE_P2P_FAULT_INJECT_SEQUENCE_GAP" in 1|true) ;; *) exit 1 ;; esac; test -f /data/ledger/.host/test-faults/p2p-sequence-gap-arm' \
    || fail "peer-a sequence-gap fault gate was not fully armed"
}

disarm_sequence_gap_fault() {
  docker_compose exec -T peer-a rm -f \
    /data/ledger/.host/test-faults/p2p-sequence-gap-arm
}

wait_for_sequence_gap_fault() {
  for _ in $(seq 1 60); do
    if docker_stream_parse_command sequence-gap-fault \
      --token "$TOKEN_A" --token "$TOKEN_B" -- \
      docker_compose logs --no-color peer-a; then
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_for_sequence_gap_rejection() {
  for _ in $(seq 1 60); do
    if docker_stream_parse_command sequence-gap-rejection \
      --token "$TOKEN_A" --token "$TOKEN_B" -- \
      docker_compose logs --no-color peer-b; then
      return 0
    fi
    sleep 1
  done
  return 1
}

update_peer_a_fixture() {
  local doc_path="$1"
  local doc_id="$3"
  local fixture_id
  fixture_id="$(date +%s)-$$"
  local phase_one_content="gap-${fixture_id}-phase-one"
  local next_content="gap-${fixture_id}-phase-two"
  local workspace_root
  workspace_root="$(peer_a_workspace_root)" \
    || fail "could not resolve peer-a projection workspace for ${REPO_ID}"

  docker_compose exec -T peer-a sh -c \
    "printf '%s' '${phase_one_content}' > '${workspace_root}/${doc_path}'"
  if ! wait_for_pending_path "$PORT_A" "$COOKIE_A" "$doc_path"; then
    diagnose
    fail "peer-a did not observe gap-phase update for ${doc_path}"
  fi
  if ! stage_apply_and_commit_path "$PORT_A" "$doc_path" "docker p2p sequence gap phase one ${fixture_id}" "$doc_id"; then
    fail "peer-a could not stage/apply/commit first gap-phase fixture ${doc_path}"
  fi
  docker_compose exec -T peer-a sh -c \
    "printf '%s' '${next_content}' > '${workspace_root}/${doc_path}'"
  if ! wait_for_pending_path "$PORT_A" "$COOKIE_A" "$doc_path"; then
    diagnose
    fail "peer-a did not observe second gap-phase update for ${doc_path}"
  fi
  if ! stage_apply_and_commit_path "$PORT_A" "$doc_path" "docker p2p sequence gap phase two ${fixture_id}" "$doc_id"; then
    fail "peer-a could not stage/apply/commit second gap-phase fixture ${doc_path}"
  fi
  printf '%s\n' "$next_content"
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
    --equals "$expected_content" \
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
  local service
  local scan_status
  if docker_stream_parse_command token-scan \
    --token "$TOKEN_A" --token "$TOKEN_B" -- \
    docker_compose logs --no-color; then
    :
  else
    scan_status=$?
    if [[ "$scan_status" -eq "$DOCKER_DIAGNOSTIC_TOKEN_STATUS" ]]; then
      fail "P2P token material appeared in compose logs during ${phase} hygiene check"
    fi
    fail "could not scan complete compose logs during ${phase} hygiene check (status ${scan_status})"
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
[[ "$PROJECT" =~ ^deve-p2p-mesh-[A-Za-z0-9][A-Za-z0-9_.-]*$ ]] \
  || fail "invalid P2P compose project identity"
[[ ${#REPO_KEY} -eq 32 ]] || fail "DEVE_DOCKER_P2P_MESH_REPO_KEY must be exactly 32 ASCII bytes"
[[ -f "$COMPOSE_FILE" ]] || fail "compose file not found: $COMPOSE_FILE"
[[ "$COMPOSE_FILE" == "$ROOT_DIR/docker-compose.mesh.yml" ]] \
  || fail "P2P producer refuses an unbound compose override"
bash "$ROOT_DIR/scripts/docker-p2p-mesh-bootstrap.test.sh"
bash "$ROOT_DIR/scripts/docker-p2p-mesh-cleanup.test.sh"
bash "$ROOT_DIR/scripts/docker-p2p-mesh-diagnostics.test.sh"
docker_cmd info >/dev/null 2>&1 || require_or_skip "docker daemon is not reachable"
preflight_port "$PORT_A"
preflight_port "$PORT_B"

trap cleanup EXIT
DEVE_DOCKER_P2P_MESH_STATE_FILE="$STATE_FILE" \
  bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh" \
    write "$PROJECT" "$COMPOSE_FILE" "$COOKIE_A" "$COOKIE_B"

if [[ "$SKIP_BUILD" != "1" && "$SKIP_BUILD" != "true" ]]; then
  docker_compose build peer-a
else
  docker_cmd image inspect "$IMAGE" >/dev/null 2>&1 || fail "existing image not found: $IMAGE"
  verify_candidate_image
fi
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

if is_true "$INJECT_SEQUENCE_GAP"; then
  arm_sequence_gap_fault
  docker_compose stop peer-b >/dev/null
  NEXT_DOC_CONTENT="$(update_peer_a_fixture "$DOC_PATH" "$DOC_CONTENT" "$DOC_ID")"
  docker_compose up -d peer-b >/dev/null
  wait_for_peer "$PORT_B" || fail "peer-b did not restart for sequence-gap delivery"
  if ! wait_for_sequence_gap_fault; then
    diagnose
    fail "sequence-gap fault was armed but no incomplete range was emitted"
  fi
  if ! wait_for_sequence_gap_rejection; then
    diagnose
    fail "peer-b did not reject the observed N+1 fact while N was missing"
  fi

  docker_compose stop peer-b >/dev/null
  if ! run_offline_shadow_check "$PEER_A_ID" "$DOC_ID" "$DOC_CONTENT"; then
    diagnose
    fail "peer-b advanced its shadow while the source range contained a gap"
  fi

  disarm_sequence_gap_fault
  docker_compose up -d peer-b >/dev/null
  if ! wait_for_peer "$PORT_B"; then
    diagnose
    fail "peer-b did not restart after sequence-gap hold"
  fi
  if ! wait_for_recovered_shadow "$PEER_A_ID" "$DOC_ID" "$NEXT_DOC_CONTENT"; then
    diagnose
    fail "peer-b did not recover the complete range after the gap was restored"
  fi
  docker_compose up -d peer-b >/dev/null
  wait_for_peer "$PORT_B" || fail "peer-b did not restart after recovered shadow verification"
fi

assert_no_token_material_in_logs_or_persisted_data "final"
verify_candidate_image

if ! is_true "$KEEP"; then
  cleanup || fail "compose cleanup failed after successful smoke"
  trap - EXIT
fi

echo "docker-p2p-mesh-smoke: ok"
