#!/usr/bin/env bash
set -euo pipefail

# REL-006 runtime release info smoke.
# Checks a running Deve server's public runtime endpoint without starting it.

BASE_URL="${DEVE_RUNTIME_BASE_URL:-http://127.0.0.1:3001}"
REQUIRED="${DEVE_RUNTIME_SMOKE_REQUIRED:-0}"
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

if ! payload="$(curl --noproxy '*' --fail --silent --show-error --max-time 5 "$URL")"; then
  skip_or_fail "runtime endpoint is not reachable at $URL"
fi

PAYLOAD="$payload" python3 - <<'PY'
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

if payload["delivery"] not in allowed_delivery:
    print(
        f"runtime-release-info-smoke: unsupported delivery {payload['delivery']}",
        file=sys.stderr,
    )
    sys.exit(1)

summary = (
    f"{payload['role']} v{payload['version']} "
    f"{payload['profile']} {payload['delivery']} {payload['environment']}"
)
print(f"runtime-release-info-smoke: ok: {summary}")
PY
