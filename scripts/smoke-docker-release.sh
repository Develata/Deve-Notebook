#!/usr/bin/env bash
set -euo pipefail

# REL-002 smoke: build the local Docker image, run it with production auth
# material, and verify the public node-role health endpoint responds.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${DEVE_DOCKER_SMOKE_IMAGE:-deve-notebook:local-smoke}"
CONTAINER_NAME="${DEVE_DOCKER_SMOKE_CONTAINER:-deve-docker-smoke-$$}"
HOST_PORT="${DEVE_DOCKER_SMOKE_PORT:-3001}"
REQUIRED="${DEVE_DOCKER_SMOKE_REQUIRED:-0}"
AUTH_SECRET="${DEVE_DOCKER_SMOKE_AUTH_SECRET:-deve_docker_smoke_secret_32_bytes_ok!!}"
AUTH_USER="${DEVE_DOCKER_SMOKE_AUTH_USER:-admin}"
AUTH_PASS="${DEVE_DOCKER_SMOKE_AUTH_PASS:-\$argon2id\$v=19\$m=65536,t=2,p=1\$c29tZXNhbHQ\$CTFhFdXPJO1aFaMaO6Mm5c8y7cJHAph8ArZWb2GRPPc}"
DATA_DIR=""

fail() {
  echo "docker-release-smoke: $*" >&2
  exit 1
}

skip() {
  echo "docker-release-smoke: skipped: $*"
  exit 0
}

require_or_skip() {
  if [[ "$REQUIRED" == "1" || "$REQUIRED" == "true" ]]; then
    fail "$1"
  fi
  skip "$1"
}

cleanup() {
  docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
  if [[ -n "$DATA_DIR" ]]; then
    rm -rf "$DATA_DIR"
  fi
}

command -v docker >/dev/null 2>&1 || require_or_skip "docker command not found"
command -v curl >/dev/null 2>&1 || require_or_skip "curl command not found"
docker info >/dev/null 2>&1 || require_or_skip "docker daemon is not reachable"

DATA_DIR="$(mktemp -d)"
trap cleanup EXIT

docker build -t "$IMAGE" "$ROOT_DIR"

docker run -d \
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
  status="$(curl -fsS -o /dev/null -w '%{http_code}' "http://127.0.0.1:${HOST_PORT}/api/node/role" || true)"
  if [[ "$status" == "200" ]]; then
    echo "docker-release-smoke: ok"
    exit 0
  fi
  sleep 1
done

docker logs "$CONTAINER_NAME" >&2 || true
fail "node-role endpoint did not become healthy on port $HOST_PORT"
