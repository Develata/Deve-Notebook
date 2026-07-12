#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

write_fixture() {
  local workspace="$1"
  local desktop="$2"
  local mobile="$3"

  mkdir -p "$WORK_DIR/apps/desktop" "$WORK_DIR/apps/mobile"
  printf '[workspace]\n[workspace.package]\nversion = "%s"\n' "$workspace" >"$WORK_DIR/Cargo.toml"
  printf '{"version":"%s"}\n' "$desktop" >"$WORK_DIR/apps/desktop/tauri.conf.json"
  printf '{"version":"%s"}\n' "$mobile" >"$WORK_DIR/apps/mobile/tauri.conf.json"
}

expect_ok() {
  DEVE_RELEASE_VERSION_ROOT="$WORK_DIR" bash "$ROOT_DIR/scripts/check-release-version-match.sh" "$1" >/dev/null
}

expect_fail() {
  if DEVE_RELEASE_VERSION_ROOT="$WORK_DIR" bash "$ROOT_DIR/scripts/check-release-version-match.sh" "$1" >/dev/null 2>&1; then
    echo "release-version-match-test: expected failure for $1" >&2
    exit 1
  fi
}

write_fixture "1.2.3" "1.2.3" "1.2.3"
expect_ok "v1.2.3"

write_fixture "1.2.3-rc.1+build.7" "1.2.3-rc.1+build.7" "1.2.3-rc.1+build.7"
expect_ok "v1.2.3-rc.1+build.7"
expect_fail "v1.2.3-rc.1"

write_fixture "1.2.4" "1.2.3" "1.2.3"
expect_fail "v1.2.3"
write_fixture "1.2.3" "1.2.4" "1.2.3"
expect_fail "v1.2.3"
write_fixture "1.2.3" "1.2.3" "1.2.4"
expect_fail "v1.2.3"
expect_fail "1.2.3"

echo "release-version-match-test: ok"
