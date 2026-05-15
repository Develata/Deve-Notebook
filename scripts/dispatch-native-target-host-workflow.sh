#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GH_BIN="${DEVE_GH_BIN:-gh}"
WORKFLOW_FILE="native-target-host.yml"
TARGET="${DEVE_NATIVE_TARGET_HOST_TARGET:-all}"
REQUIRED_PREFLIGHT="${DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT:-false}"
RUN_DESKTOP_PACKAGE_BUILD="${DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD:-false}"
RUN_DESKTOP_STARTUP_SMOKE="${DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_STARTUP_SMOKE:-false}"
RUN_MOBILE_IOS_PACKAGE_BUILD="${DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_PACKAGE_BUILD:-false}"
DISPATCH="${DEVE_NATIVE_TARGET_HOST_DISPATCH:-0}"
REF="${DEVE_NATIVE_TARGET_HOST_REF:-}"
REPOSITORY="${DEVE_NATIVE_TARGET_HOST_REPOSITORY:-${GITHUB_REPOSITORY:-}}"
TOKEN="${DEVE_GITHUB_TOKEN:-${GH_TOKEN:-${GITHUB_TOKEN:-}}}"

fail() {
  echo "native-target-host-workflow-dispatch: $*" >&2
  exit 1
}

normalize_bool() {
  case "$1" in
    1|true|TRUE|yes|YES) printf 'true' ;;
    0|false|FALSE|no|NO) printf 'false' ;;
    *) fail "invalid boolean: $1" ;;
  esac
}

resolve_repository() {
  local repo="$REPOSITORY"
  local remote_url

  if [[ -z "$repo" ]]; then
    remote_url="$(git -C "$ROOT_DIR" remote get-url origin 2>/dev/null || true)"
    case "$remote_url" in
      https://github.com/*)
        repo="${remote_url#https://github.com/}"
        repo="${repo%.git}"
        ;;
      git@github.com:*)
        repo="${remote_url#git@github.com:}"
        repo="${repo%.git}"
        ;;
      ssh://git@github.com/*)
        repo="${remote_url#ssh://git@github.com/}"
        repo="${repo%.git}"
        ;;
    esac
  fi

  case "$repo" in
    */*) printf '%s\n' "$repo" ;;
    *) return 1 ;;
  esac
}

python_bin() {
  if command -v python3 >/dev/null 2>&1; then
    command -v python3
    return 0
  fi
  command -v python
}

dispatch_payload() {
  local python

  python="$(python_bin)" || fail "python3 or python is required for GitHub API dispatch fallback"
  "$python" - "$REF" "$TARGET" "$REQUIRED_PREFLIGHT" "$RUN_DESKTOP_PACKAGE_BUILD" "$RUN_DESKTOP_STARTUP_SMOKE" "$RUN_MOBILE_IOS_PACKAGE_BUILD" <<'PY'
import json
import sys

(
    ref,
    target,
    required_preflight,
    run_desktop_package_build,
    run_desktop_startup_smoke,
    run_mobile_ios_package_build,
) = sys.argv[1:]
payload = {
    "ref": ref,
    "inputs": {
        "target": target,
        "required_preflight": required_preflight,
        "run_desktop_package_build": run_desktop_package_build,
        "run_desktop_startup_smoke": run_desktop_startup_smoke,
        "run_mobile_ios_package_build": run_mobile_ios_package_build,
    },
}
print(json.dumps(payload, separators=(",", ":")))
PY
}

dispatch_with_api() {
  local repo
  local payload
  local api_url

  [[ -n "$TOKEN" ]] || fail "GitHub API dispatch fallback requires DEVE_GITHUB_TOKEN, GH_TOKEN, or GITHUB_TOKEN"
  command -v curl >/dev/null 2>&1 || fail "GitHub API dispatch fallback requires curl"
  repo="$(resolve_repository)" || fail "cannot resolve GitHub repository; set DEVE_NATIVE_TARGET_HOST_REPOSITORY=owner/repo"
  payload="$(dispatch_payload)"
  api_url="https://api.github.com/repos/$repo/actions/workflows/$WORKFLOW_FILE/dispatches"

  curl -fsS \
    -X POST \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer $TOKEN" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    --data-binary "$payload" \
    "$api_url" >/dev/null
  echo "native-target-host-workflow-dispatch: dispatched via GitHub API"
}

case "$TARGET" in
  all|desktop-macos|desktop-windows|mobile-ios) ;;
  *) fail "invalid target: $TARGET" ;;
esac

REQUIRED_PREFLIGHT="$(normalize_bool "$REQUIRED_PREFLIGHT")"
RUN_DESKTOP_PACKAGE_BUILD="$(normalize_bool "$RUN_DESKTOP_PACKAGE_BUILD")"
RUN_DESKTOP_STARTUP_SMOKE="$(normalize_bool "$RUN_DESKTOP_STARTUP_SMOKE")"
RUN_MOBILE_IOS_PACKAGE_BUILD="$(normalize_bool "$RUN_MOBILE_IOS_PACKAGE_BUILD")"

if [[ -z "$REF" ]]; then
  REF="$(git -C "$ROOT_DIR" branch --show-current 2>/dev/null || true)"
fi
if [[ -z "$REF" ]]; then
  REF="main"
fi

command_args=(
  workflow run "$WORKFLOW_FILE"
  --field "target=$TARGET"
  --field "required_preflight=$REQUIRED_PREFLIGHT"
  --field "run_desktop_package_build=$RUN_DESKTOP_PACKAGE_BUILD"
  --field "run_desktop_startup_smoke=$RUN_DESKTOP_STARTUP_SMOKE"
  --field "run_mobile_ios_package_build=$RUN_MOBILE_IOS_PACKAGE_BUILD"
  --ref "$REF"
)

printf 'native-target-host-workflow-dispatch: command: %q' "$GH_BIN"
printf ' %q' "${command_args[@]}"
printf '\n'
if repo="$(resolve_repository 2>/dev/null)"; then
  echo "native-target-host-workflow-dispatch: api: POST https://api.github.com/repos/$repo/actions/workflows/$WORKFLOW_FILE/dispatches"
else
  echo "native-target-host-workflow-dispatch: api: set DEVE_NATIVE_TARGET_HOST_REPOSITORY=owner/repo for token fallback"
fi

if [[ "$DISPATCH" != "1" ]]; then
  echo "native-target-host-workflow-dispatch: dry-run; set DEVE_NATIVE_TARGET_HOST_DISPATCH=1 to run"
  exit 0
fi

if command -v "$GH_BIN" >/dev/null 2>&1 && "$GH_BIN" auth status >/dev/null 2>&1; then
  "$GH_BIN" "${command_args[@]}"
  echo "native-target-host-workflow-dispatch: dispatched via GitHub CLI"
  exit 0
fi

dispatch_with_api
