#!/usr/bin/env bash
set -euo pipefail

# REL-002 smoke: build the local Docker image, run it with production auth
# material, and verify the public node-role health endpoint responds.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${DEVE_DOCKER_SMOKE_IMAGE:-deve-notebook:local-smoke}"
CONTAINER_NAME="${DEVE_DOCKER_SMOKE_CONTAINER:-deve-docker-smoke-$$}"
HOST_PORT="${DEVE_DOCKER_SMOKE_PORT:-3001}"
REQUIRED="${DEVE_DOCKER_SMOKE_REQUIRED:-0}"
DOCKER_BIN="${DEVE_DOCKER_BIN:-docker}"
AUTH_SECRET="${DEVE_DOCKER_SMOKE_AUTH_SECRET:-deve_docker_smoke_secret_32_bytes_ok!!}"
AUTH_USER="${DEVE_DOCKER_SMOKE_AUTH_USER:-admin}"
AUTH_PASS="${DEVE_DOCKER_SMOKE_AUTH_PASS:-\$argon2id\$v=19\$m=65536,t=2,p=1\$c29tZXNhbHQ\$CTFhFdXPJO1aFaMaO6Mm5c8y7cJHAph8ArZWb2GRPPc}"
AUTH_PASSWORD="${DEVE_DOCKER_SMOKE_AUTH_PASSWORD:-password}"
DATA_DIR=""

fail() {
  echo "docker-release-smoke: $*" >&2
  exit 1
}

skip() {
  echo "docker-release-smoke: skipped: $*"
  exit 0
}

docker_cmd() {
  command "$DOCKER_BIN" "$@"
}

curl_local() {
  curl --noproxy "127.0.0.1,localhost" "$@"
}

diagnose_container_endpoint() {
  echo "docker-release-smoke: host endpoint probe failed; collecting container diagnostics" >&2
  docker_cmd inspect -f 'health={{if .State.Health}}{{.State.Health.Status}}{{else}}<none>{{end}} status={{.State.Status}}' "$CONTAINER_NAME" >&2 || true
  docker_cmd exec "$CONTAINER_NAME" curl -fsS "http://127.0.0.1:3001/api/node/role" >&2 || true
  echo >&2
}

docker_bin_available() {
  if [[ "$DOCKER_BIN" == */* ]]; then
    [[ -x "$DOCKER_BIN" ]]
    return
  fi
  command -v "$DOCKER_BIN" >/dev/null 2>&1
}

docker_env_summary() {
  echo "docker-release-smoke: docker_bin=$DOCKER_BIN"
  echo "docker-release-smoke: DOCKER_HOST=${DOCKER_HOST:-<unset>}"
  echo "docker-release-smoke: DOCKER_CONTEXT=${DOCKER_CONTEXT:-<unset>}"
}

require_or_skip() {
  if [[ "$REQUIRED" == "1" || "$REQUIRED" == "true" ]]; then
    docker_env_summary >&2
    fail "$1"
  fi
  docker_env_summary
  skip "$1"
}

cleanup() {
  docker_cmd rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
  if [[ -n "$DATA_DIR" ]]; then
    rm -rf "$DATA_DIR"
  fi
}

docker_bin_available || require_or_skip "docker command not found"
command -v curl >/dev/null 2>&1 || require_or_skip "curl command not found"
docker_cmd info >/dev/null 2>&1 || require_or_skip "docker daemon is not reachable"

DATA_DIR="$(mktemp -d)"
trap cleanup EXIT

docker_cmd build -t "$IMAGE" "$ROOT_DIR"

docker_cmd run -d \
  --name "$CONTAINER_NAME" \
  -p "$HOST_PORT:3001" \
  -v "$DATA_DIR:/data" \
  -e DEVE_LEDGER_DIR=/data/ledger \
  -e DEVE_VAULT_PATH=/data/vault \
  -e DEVE_BIND_ADDR=0.0.0.0:3001 \
  -e AUTH_SECRET="$AUTH_SECRET" \
  -e AUTH_USER="$AUTH_USER" \
  -e AUTH_PASS="$AUTH_PASS" \
  "$IMAGE" >/dev/null

for _ in $(seq 1 60); do
  status="$(curl_local -fsS -o /dev/null -w '%{http_code}' "http://127.0.0.1:${HOST_PORT}/api/node/role" || true)"
  if [[ "$status" == "200" ]]; then
    login_status="$(
      curl_local -fsS -o /dev/null -w '%{http_code}' \
        -X POST "http://127.0.0.1:${HOST_PORT}/api/auth/login" \
        -H 'Content-Type: application/json' \
        --data "{\"username\":\"${AUTH_USER}\",\"password\":\"${AUTH_PASSWORD}\"}" \
        || true
    )"
    if [[ "$login_status" == "200" ]]; then
      echo "docker-release-smoke: ok"
      exit 0
    fi
    echo "docker-release-smoke: node-role ready but login returned $login_status; retrying" >&2
  fi
  sleep 1
done

diagnose_container_endpoint
docker_cmd logs "$CONTAINER_NAME" >&2 || true
fail "node-role endpoint did not become healthy on port $HOST_PORT"
