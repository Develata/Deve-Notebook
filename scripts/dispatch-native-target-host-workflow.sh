#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GH_BIN="${DEVE_GH_BIN:-gh}"
WORKFLOW_FILE="native-target-host.yml"
TARGET="${DEVE_NATIVE_TARGET_HOST_TARGET:-all}"
REQUIRED_PREFLIGHT="${DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT:-false}"
RUN_DESKTOP_PACKAGE_BUILD="${DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD:-false}"
RUN_DESKTOP_STARTUP_SMOKE="${DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_STARTUP_SMOKE:-false}"
RUN_DESKTOP_INSTALLER_SMOKE="${DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE:-false}"
RUN_DESKTOP_REMOTE_BROWSER_SMOKE="${DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE:-false}"
DESKTOP_REMOTE_HTTPS_ORIGIN="${DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN:-}"
DESKTOP_REMOTE_USERNAME="${DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_USERNAME:-}"
DESKTOP_REMOTE_HEAD_PROOF_URL="${DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HEAD_PROOF_URL:-}"
RUN_MOBILE_ANDROID_PACKAGE_BUILD="${DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD:-false}"
RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE="${DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE:-false}"
RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE="${DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE:-false}"
MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN="${DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN:-}"
MOBILE_ANDROID_REMOTE_USERNAME="${DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_USERNAME:-}"
MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL="${DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL:-}"
RUN_MOBILE_IOS_PACKAGE_BUILD="${DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_PACKAGE_BUILD:-false}"
RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE="${DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE:-false}"
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

reject_crlf() {
  local label="$1"
  local value="$2"

  if [[ "$value" == *$'\r'* || "$value" == *$'\n'* ]]; then
    fail "$label contains a forbidden CR/LF control character"
  fi
}

validate_external_override() {
  local label="$1"
  local enabled="$2"
  local origin="$3"
  local username="$4"
  local head_proof_url="$5"
  local count=0
  local value

  reject_crlf "$label HTTPS origin" "$origin"
  reject_crlf "$label username" "$username"
  reject_crlf "$label HEAD proof URL" "$head_proof_url"

  for value in "$origin" "$username" "$head_proof_url"; do
    if [[ -n "$value" ]]; then
      count=$((count + 1))
    fi
  done

  if [[ "$count" != 0 && "$count" != 3 ]]; then
    fail "$label external override requires HTTPS origin, username, and same-origin HEAD proof URL together"
  fi
  if [[ "$count" == 3 && "$enabled" != "true" ]]; then
    fail "$label external override requires its RemoteBrowser smoke to be enabled"
  fi
  if [[ "$count" == 3 ]]; then
    if [[ ! "$origin" =~ ^https://[A-Za-z0-9.-]+(:[0-9]+)?$ ]]; then
      fail "$label external override requires an exact HTTPS origin"
    fi
    case "$head_proof_url" in
      "$origin"|"$origin"/*) ;;
      *) fail "$label HEAD proof URL must use the RemoteBrowser HTTPS origin" ;;
    esac
  fi
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
  "$python" - "$REF" "$TARGET" "$REQUIRED_PREFLIGHT" "$RUN_DESKTOP_PACKAGE_BUILD" "$RUN_DESKTOP_STARTUP_SMOKE" "$RUN_DESKTOP_INSTALLER_SMOKE" "$RUN_DESKTOP_REMOTE_BROWSER_SMOKE" "$DESKTOP_REMOTE_HTTPS_ORIGIN" "$DESKTOP_REMOTE_USERNAME" "$DESKTOP_REMOTE_HEAD_PROOF_URL" "$RUN_MOBILE_ANDROID_PACKAGE_BUILD" "$RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE" "$RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE" "$MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN" "$MOBILE_ANDROID_REMOTE_USERNAME" "$MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL" "$RUN_MOBILE_IOS_PACKAGE_BUILD" "$RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE" <<'PY'
import json
import sys

(
    ref,
    target,
    required_preflight,
    run_desktop_package_build,
    run_desktop_startup_smoke,
    run_desktop_installer_smoke,
    run_desktop_remote_browser_smoke,
    desktop_remote_https_origin,
    desktop_remote_username,
    desktop_remote_head_proof_url,
    run_mobile_android_package_build,
    run_mobile_android_install_startup_smoke,
    run_mobile_android_remote_browser_smoke,
    mobile_android_remote_https_origin,
    mobile_android_remote_username,
    mobile_android_remote_head_proof_url,
    run_mobile_ios_package_build,
    run_mobile_ios_install_startup_smoke,
) = sys.argv[1:]
payload = {
    "ref": ref,
    "inputs": {
        "target": target,
        "required_preflight": required_preflight,
        "run_desktop_package_build": run_desktop_package_build,
        "run_desktop_startup_smoke": run_desktop_startup_smoke,
        "run_desktop_installer_smoke": run_desktop_installer_smoke,
        "run_desktop_remote_browser_smoke": run_desktop_remote_browser_smoke,
        "desktop_remote_https_origin": desktop_remote_https_origin,
        "desktop_remote_username": desktop_remote_username,
        "desktop_remote_head_proof_url": desktop_remote_head_proof_url,
        "run_mobile_android_package_build": run_mobile_android_package_build,
        "run_mobile_android_install_startup_smoke": run_mobile_android_install_startup_smoke,
        "run_mobile_android_remote_browser_smoke": run_mobile_android_remote_browser_smoke,
        "mobile_android_remote_https_origin": mobile_android_remote_https_origin,
        "mobile_android_remote_username": mobile_android_remote_username,
        "mobile_android_remote_head_proof_url": mobile_android_remote_head_proof_url,
        "run_mobile_ios_package_build": run_mobile_ios_package_build,
        "run_mobile_ios_install_startup_smoke": run_mobile_ios_install_startup_smoke,
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
  all|desktop-macos|desktop-windows|mobile-android|mobile-ios) ;;
  *) fail "invalid target: $TARGET" ;;
esac

REQUIRED_PREFLIGHT="$(normalize_bool "$REQUIRED_PREFLIGHT")"
RUN_DESKTOP_PACKAGE_BUILD="$(normalize_bool "$RUN_DESKTOP_PACKAGE_BUILD")"
RUN_DESKTOP_STARTUP_SMOKE="$(normalize_bool "$RUN_DESKTOP_STARTUP_SMOKE")"
RUN_DESKTOP_INSTALLER_SMOKE="$(normalize_bool "$RUN_DESKTOP_INSTALLER_SMOKE")"
RUN_DESKTOP_REMOTE_BROWSER_SMOKE="$(normalize_bool "$RUN_DESKTOP_REMOTE_BROWSER_SMOKE")"
RUN_MOBILE_ANDROID_PACKAGE_BUILD="$(normalize_bool "$RUN_MOBILE_ANDROID_PACKAGE_BUILD")"
RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE="$(normalize_bool "$RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE")"
RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE="$(normalize_bool "$RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE")"
RUN_MOBILE_IOS_PACKAGE_BUILD="$(normalize_bool "$RUN_MOBILE_IOS_PACKAGE_BUILD")"
RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE="$(normalize_bool "$RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE")"

if [[ "$RUN_DESKTOP_PACKAGE_BUILD" != "true" && "$RUN_DESKTOP_STARTUP_SMOKE" == "true" ]]; then
  fail "desktop startup/native-session smoke requires DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true"
fi
if [[ "$RUN_DESKTOP_PACKAGE_BUILD" != "true" && "$RUN_DESKTOP_INSTALLER_SMOKE" == "true" ]]; then
  fail "desktop installer smoke requires DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true"
fi
if [[ "$RUN_DESKTOP_REMOTE_BROWSER_SMOKE" == "true" && ( "$RUN_DESKTOP_PACKAGE_BUILD" != "true" || "$RUN_DESKTOP_INSTALLER_SMOKE" != "true" ) ]]; then
  fail "desktop RemoteBrowser smoke requires desktop package build and installer smoke"
fi
if [[ "$RUN_DESKTOP_REMOTE_BROWSER_SMOKE" == "true" && "$TARGET" != "all" && "$TARGET" != "desktop-windows" ]]; then
  fail "desktop RemoteBrowser smoke requires target=all or target=desktop-windows"
fi

if [[ "$RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE" == "true" && ( "$RUN_MOBILE_ANDROID_PACKAGE_BUILD" != "true" || "$RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE" != "true" ) ]]; then
  fail "Android RemoteBrowser smoke requires Android package build and install/startup smoke"
fi
if [[ "$RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE" == "true" && "$TARGET" != "all" && "$TARGET" != "mobile-android" ]]; then
  fail "Android RemoteBrowser smoke requires target=all or target=mobile-android"
fi

validate_external_override \
  "desktop RemoteBrowser" \
  "$RUN_DESKTOP_REMOTE_BROWSER_SMOKE" \
  "$DESKTOP_REMOTE_HTTPS_ORIGIN" \
  "$DESKTOP_REMOTE_USERNAME" \
  "$DESKTOP_REMOTE_HEAD_PROOF_URL"
validate_external_override \
  "Android RemoteBrowser" \
  "$RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE" \
  "$MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN" \
  "$MOBILE_ANDROID_REMOTE_USERNAME" \
  "$MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL"

if [[ -z "$REF" ]]; then
  REF="$(git -C "$ROOT_DIR" branch --show-current 2>/dev/null || true)"
fi
if [[ -z "$REF" ]]; then
  REF="main"
fi
reject_crlf "Git ref" "$REF"
reject_crlf "GitHub repository" "$REPOSITORY"
reject_crlf "GitHub token" "$TOKEN"

command_args=(
  workflow run "$WORKFLOW_FILE"
  --field "target=$TARGET"
  --field "required_preflight=$REQUIRED_PREFLIGHT"
  --field "run_desktop_package_build=$RUN_DESKTOP_PACKAGE_BUILD"
  --field "run_desktop_startup_smoke=$RUN_DESKTOP_STARTUP_SMOKE"
  --field "run_desktop_installer_smoke=$RUN_DESKTOP_INSTALLER_SMOKE"
  --field "run_desktop_remote_browser_smoke=$RUN_DESKTOP_REMOTE_BROWSER_SMOKE"
  --field "desktop_remote_https_origin=$DESKTOP_REMOTE_HTTPS_ORIGIN"
  --field "desktop_remote_username=$DESKTOP_REMOTE_USERNAME"
  --field "desktop_remote_head_proof_url=$DESKTOP_REMOTE_HEAD_PROOF_URL"
  --field "run_mobile_android_package_build=$RUN_MOBILE_ANDROID_PACKAGE_BUILD"
  --field "run_mobile_android_install_startup_smoke=$RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE"
  --field "run_mobile_android_remote_browser_smoke=$RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE"
  --field "mobile_android_remote_https_origin=$MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN"
  --field "mobile_android_remote_username=$MOBILE_ANDROID_REMOTE_USERNAME"
  --field "mobile_android_remote_head_proof_url=$MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL"
  --field "run_mobile_ios_package_build=$RUN_MOBILE_IOS_PACKAGE_BUILD"
  --field "run_mobile_ios_install_startup_smoke=$RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE"
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
