#!/usr/bin/env bash
set -euo pipefail

# REL-006 runtime release info smoke.
# Checks a running Deve server's public runtime endpoint without starting it.

BASE_URL="${DEVE_RUNTIME_BASE_URL:-http://127.0.0.1:3001}"
REQUIRED="${DEVE_RUNTIME_SMOKE_REQUIRED:-0}"
EXPECTED_DELIVERY="${DEVE_RUNTIME_EXPECTED_DELIVERY:-}"
MIN_LOCAL_REPOS="${DEVE_RUNTIME_MIN_LOCAL_REPOS:-0}"
PYTHON_BIN="${DEVE_RUNTIME_PYTHON_BIN:-}"
URL="${BASE_URL%/}/api/node/role"

fail() {
  echo "runtime-release-info-smoke: $*" >&2
  exit 1
}

skip_or_fail() {
  local message="$1"
  if [[ "$REQUIRED" == "1" ]]; then
    fail "$message"
  fi
  echo "runtime-release-info-smoke: skipped: $message"
  exit 0
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

find_python || skip_or_fail "python3/python command not found"

if ! payload="$(curl --noproxy '*' --fail --silent --show-error --max-time 5 "$URL")"; then
  skip_or_fail "runtime endpoint is not reachable at $URL"
fi

PAYLOAD="$payload" EXPECTED_DELIVERY="$EXPECTED_DELIVERY" MIN_LOCAL_REPOS="$MIN_LOCAL_REPOS" "$PYTHON_BIN" - <<'PY'
import json
import os
import sys

allowed_delivery = {
    "embedded-frontend",
    "static-dir",
    "static-dir-override",
    "api-only",
    "plugin-host-proxy",
}

try:
    payload = json.loads(os.environ["PAYLOAD"])
except Exception as exc:
    print(f"runtime-release-info-smoke: invalid JSON: {exc}", file=sys.stderr)
    sys.exit(1)

required_strings = ["role", "version", "profile", "delivery", "environment"]
for key in required_strings:
    value = payload.get(key)
    if not isinstance(value, str) or not value:
        print(f"runtime-release-info-smoke: missing non-empty string field {key}", file=sys.stderr)
        sys.exit(1)

for key in ["ws_port", "main_port"]:
    value = payload.get(key)
    if not isinstance(value, int) or value < 0:
        print(f"runtime-release-info-smoke: invalid port field {key}", file=sys.stderr)
        sys.exit(1)

repo_health = payload.get("repo_health")
if not isinstance(repo_health, dict):
    print("runtime-release-info-smoke: missing repo_health object", file=sys.stderr)
    sys.exit(1)

if repo_health.get("status") not in {"healthy", "degraded", "unknown"}:
    print("runtime-release-info-smoke: invalid repo_health.status", file=sys.stderr)
    sys.exit(1)

for key in ["local_total", "healthy", "degraded"]:
    value = repo_health.get(key)
    if not isinstance(value, int) or value < 0:
        print(f"runtime-release-info-smoke: invalid repo_health.{key}", file=sys.stderr)
        sys.exit(1)

if repo_health["healthy"] + repo_health["degraded"] != repo_health["local_total"]:
    print("runtime-release-info-smoke: repo_health counts do not add up", file=sys.stderr)
    sys.exit(1)

if repo_health["status"] == "healthy" and repo_health["degraded"] != 0:
    print("runtime-release-info-smoke: healthy repo_health has degraded repos", file=sys.stderr)
    sys.exit(1)

if repo_health["status"] == "degraded" and repo_health["degraded"] == 0:
    print("runtime-release-info-smoke: degraded repo_health has no degraded repos", file=sys.stderr)
    sys.exit(1)

if repo_health["status"] == "unknown" and (
    repo_health["local_total"] != 0
    or repo_health["healthy"] != 0
    or repo_health["degraded"] != 0
):
    print("runtime-release-info-smoke: unknown repo_health must use zero counts", file=sys.stderr)
    sys.exit(1)

if payload["delivery"] not in allowed_delivery:
    print(
        f"runtime-release-info-smoke: unsupported delivery {payload['delivery']}",
        file=sys.stderr,
    )
    sys.exit(1)

expected_delivery = os.environ["EXPECTED_DELIVERY"]
if expected_delivery and payload["delivery"] != expected_delivery:
    print(
        f"runtime-release-info-smoke: expected delivery {expected_delivery}, got {payload['delivery']}",
        file=sys.stderr,
    )
    sys.exit(1)

try:
    min_local_repos = int(os.environ["MIN_LOCAL_REPOS"])
except ValueError:
    print("runtime-release-info-smoke: invalid DEVE_RUNTIME_MIN_LOCAL_REPOS", file=sys.stderr)
    sys.exit(1)
if min_local_repos < 0:
    print("runtime-release-info-smoke: minimum local repo count must be non-negative", file=sys.stderr)
    sys.exit(1)
if repo_health["local_total"] < min_local_repos:
    print(
        f"runtime-release-info-smoke: expected at least {min_local_repos} local repos, got {repo_health['local_total']}",
        file=sys.stderr,
    )
    sys.exit(1)

summary = (
    f"{payload['role']} v{payload['version']} "
    f"{payload['profile']} {payload['delivery']} {payload['environment']} "
    f"repos:{repo_health['status']}({repo_health['degraded']}/{repo_health['local_total']})"
)
print(f"runtime-release-info-smoke: ok: {summary}")
PY
