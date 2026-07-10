#!/usr/bin/env bash
set -euo pipefail

# REL-009 smoke: build the local Docker image, run one containerized server,
# and exercise it from multiple isolated Playwright browser contexts.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/baseline-wrapper.sh
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
COMPOSE_FILE="${DEVE_DOCKER_MULTI_COMPOSE_FILE:-$ROOT_DIR/docker-compose.multiclient.yml}"
PROJECT="${DEVE_DOCKER_MULTI_PROJECT:-deve-multiclient-$$}"
HOST_PORT="${DEVE_DOCKER_MULTI_PORT:-3101}"
BASE_URL="${DEVE_DOCKER_MULTI_BASE_URL:-http://127.0.0.1:${HOST_PORT}}"
BASE_ORIGIN="${BASE_URL%/}"
REQUIRED="${DEVE_DOCKER_MULTI_REQUIRED:-0}"
KEEP="${DEVE_DOCKER_MULTI_KEEP:-0}"
DOCKER_BIN="${DEVE_DOCKER_BIN:-docker}"
AUTH_SECRET="${DEVE_DOCKER_MULTI_AUTH_SECRET:-deve_docker_multi_secret_32_bytes_ok!!}"
AUTH_USER="${DEVE_DOCKER_MULTI_AUTH_USER:-admin}"
AUTH_PASS="${DEVE_DOCKER_MULTI_AUTH_PASS:-\$argon2id\$v=19\$m=65536,t=2,p=1\$c29tZXNhbHQ\$CTFhFdXPJO1aFaMaO6Mm5c8y7cJHAph8ArZWb2GRPPc}"
AUTH_PASSWORD="${DEVE_DOCKER_MULTI_AUTH_PASSWORD:-password}"
PLAYWRIGHT_PACKAGE="${DEVE_DOCKER_MULTI_PLAYWRIGHT_PACKAGE:-playwright}"
PLAYWRIGHT_WORK_DIR="${DEVE_DOCKER_MULTI_PLAYWRIGHT_WORK_DIR:-${TMPDIR:-/tmp}/deve-docker-multiclient-playwright}"
NODE_SCRIPT="${DEVE_DOCKER_MULTI_NODE_SCRIPT:-$ROOT_DIR/scripts/smoke-docker-multiclient.mjs}"

run_deve_baseline "$ROOT_DIR" "docker-smoke-preflight" "docker-multiclient-smoke" "multiclient"

fail() {
  echo "docker-multiclient-smoke: $*" >&2
  exit 1
}

skip() {
  echo "docker-multiclient-smoke: skipped: $*"
  exit 0
}

docker_cmd() {
  command "$DOCKER_BIN" "$@"
}

docker_compose() {
  AUTH_SECRET="$AUTH_SECRET" \
  AUTH_USER="$AUTH_USER" \
  AUTH_PASS="$AUTH_PASS" \
  DEVE_DOCKER_MULTI_PORT="$HOST_PORT" \
    docker_cmd compose -f "$COMPOSE_FILE" -p "$PROJECT" "$@"
}

curl_local() {
  curl --noproxy "127.0.0.1,localhost" "$@"
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

cleanup() {
  if [[ "$KEEP" == "1" || "$KEEP" == "true" ]]; then
    echo "docker-multiclient-smoke: kept compose project '$PROJECT' at $BASE_URL"
    echo "docker-multiclient-smoke: cleanup with: $DOCKER_BIN compose -f $COMPOSE_FILE -p $PROJECT down -v --remove-orphans"
    return
  fi
  docker_compose down -v --remove-orphans >/dev/null 2>&1 || true
}

diagnose() {
  echo "docker-multiclient-smoke: collecting compose diagnostics" >&2
  docker_compose ps >&2 || true
  docker_compose logs --no-color >&2 || true
}

wait_for_server() {
  local status
  local login_status
  for _ in $(seq 1 90); do
    status="$(curl_local -fsS -o /dev/null -w '%{http_code}' "$BASE_URL/api/node/role" || true)"
    if [[ "$status" == "200" ]]; then
      login_status="$(
        curl_local -fsS -o /dev/null -w '%{http_code}' \
          -X POST "$BASE_URL/api/auth/login" \
          -H 'Content-Type: application/json' \
          --data "{\"username\":\"${AUTH_USER}\",\"password\":\"${AUTH_PASSWORD}\"}" \
          || true
      )"
      if [[ "$login_status" == "200" ]]; then
        return 0
      fi
      echo "docker-multiclient-smoke: node-role ready but login returned $login_status; retrying" >&2
    fi
    sleep 1
  done
  return 1
}

run_playwright() {
  node --test "$ROOT_DIR/scripts/smoke-docker-multiclient.test.mjs"
  mkdir -p "$PLAYWRIGHT_WORK_DIR"
  if [[ ! -f "$PLAYWRIGHT_WORK_DIR/package.json" ]]; then
    printf '{"private":true,"type":"module"}\n' >"$PLAYWRIGHT_WORK_DIR/package.json"
  fi
  if [[ ! -d "$PLAYWRIGHT_WORK_DIR/node_modules/playwright" ]]; then
    npm --prefix "$PLAYWRIGHT_WORK_DIR" install --no-audit --no-fund "$PLAYWRIGHT_PACKAGE"
  fi
  npm --prefix "$PLAYWRIGHT_WORK_DIR" exec -- playwright install chromium
  DEVE_DOCKER_MULTI_BASE_URL="$BASE_URL" \
  DEVE_DOCKER_MULTI_EXPECTED_ORIGIN="$BASE_ORIGIN" \
  DEVE_DOCKER_MULTI_AUTH_USER="$AUTH_USER" \
  DEVE_DOCKER_MULTI_AUTH_PASSWORD="$AUTH_PASSWORD" \
  DEVE_DOCKER_MULTI_PLAYWRIGHT_REQUIRE_FROM="$PLAYWRIGHT_WORK_DIR/package.json" \
    node "$NODE_SCRIPT"
}

docker_bin_available || require_or_skip "docker command not found"
command -v curl >/dev/null 2>&1 || require_or_skip "curl command not found"
command -v npm >/dev/null 2>&1 || require_or_skip "npm command not found"
case "$BASE_ORIGIN" in
  "http://127.0.0.1:${HOST_PORT}" | "http://localhost:${HOST_PORT}") ;;
  *) fail "base URL must use the container's loopback host port ${HOST_PORT}: ${BASE_URL}" ;;
esac
docker_cmd info >/dev/null 2>&1 || require_or_skip "docker daemon is not reachable"
[[ -f "$COMPOSE_FILE" ]] || fail "compose file not found: $COMPOSE_FILE"
[[ -f "$NODE_SCRIPT" ]] || fail "Playwright script not found: $NODE_SCRIPT"

trap cleanup EXIT

docker_compose up -d --build

if ! wait_for_server; then
  diagnose
  fail "server did not become login-ready at $BASE_URL"
fi

if ! run_playwright; then
  diagnose
  fail "Playwright multi-client smoke failed"
fi

echo "docker-multiclient-smoke: ok"
