#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GH_BIN="${DEVE_GH_BIN:-gh}"
TARGET="${DEVE_NATIVE_TARGET_HOST_TARGET:-all}"
REQUIRED_PREFLIGHT="${DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT:-false}"
RUN_DESKTOP_PACKAGE_BUILD="${DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD:-false}"
DISPATCH="${DEVE_NATIVE_TARGET_HOST_DISPATCH:-0}"
REF="${DEVE_NATIVE_TARGET_HOST_REF:-}"

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

case "$TARGET" in
  all|desktop-macos|desktop-windows|mobile-ios) ;;
  *) fail "invalid target: $TARGET" ;;
esac

REQUIRED_PREFLIGHT="$(normalize_bool "$REQUIRED_PREFLIGHT")"
RUN_DESKTOP_PACKAGE_BUILD="$(normalize_bool "$RUN_DESKTOP_PACKAGE_BUILD")"

if [[ -z "$REF" ]]; then
  REF="$(git -C "$ROOT_DIR" branch --show-current 2>/dev/null || true)"
fi

command_args=(
  workflow run native-target-host.yml
  --field "target=$TARGET"
  --field "required_preflight=$REQUIRED_PREFLIGHT"
  --field "run_desktop_package_build=$RUN_DESKTOP_PACKAGE_BUILD"
)

if [[ -n "$REF" ]]; then
  command_args+=(--ref "$REF")
fi

printf 'native-target-host-workflow-dispatch: command: %q' "$GH_BIN"
printf ' %q' "${command_args[@]}"
printf '\n'

if [[ "$DISPATCH" != "1" ]]; then
  echo "native-target-host-workflow-dispatch: dry-run; set DEVE_NATIVE_TARGET_HOST_DISPATCH=1 to run"
  exit 0
fi

command -v "$GH_BIN" >/dev/null 2>&1 || fail "GitHub CLI not found: $GH_BIN"
"$GH_BIN" auth status >/dev/null 2>&1 || fail "GitHub CLI is not authenticated"

"$GH_BIN" "${command_args[@]}"
echo "native-target-host-workflow-dispatch: dispatched"
