#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHECK="$ROOT_DIR/scripts/validate-release-image-tags.sh"

expect_ok() {
  local expected_version="$1"
  local expected_latest="$2"
  shift 2
  mapfile -t result < <(bash "$CHECK" "$@")
  [[ "${#result[@]}" -eq 2 ]]
  [[ "${result[0]}" == "$expected_version" && "${result[1]}" == "$expected_latest" ]]
}

expect_fail() {
  if bash "$CHECK" "$@" >/dev/null 2>&1; then
    echo "release-image-tags-test: expected failure: $*" >&2
    exit 1
  fi
}

expect_ok ghcr.io/acme/deve:1.2.3 ghcr.io/acme/deve:latest \
  1.2.3 ghcr.io/acme/deve:1.2.3 ghcr.io/acme/deve:latest
mapfile -t prerelease < <(bash "$CHECK" 1.2.3-rc.1 ghcr.io/acme/deve:1.2.3-rc.1)
[[ "${prerelease[*]}" == 'ghcr.io/acme/deve:1.2.3-rc.1' ]]
mapfile -t prerelease_build < <(bash "$CHECK" 1.2.3-build.7 ghcr.io/acme/deve:1.2.3-build.7)
[[ "${prerelease_build[*]}" == 'ghcr.io/acme/deve:1.2.3-build.7' ]]
expect_ok ghcr.io/acme/deve:1.2.3_build_build.7 ghcr.io/acme/deve:latest \
  1.2.3+build.7 ghcr.io/acme/deve:1.2.3_build_build.7 ghcr.io/acme/deve:latest

expect_fail 1.2.3 ghcr.io/acme/deve:1.2.3 ghcr.io/acme/deve:latest ghcr.io/acme/deve:latest
expect_fail 1.2.3 ghcr.io/acme/deve:1.2.3
expect_fail 1.2.3 ghcr.io/acme/deve:1.2.3 ghcr.io/other/deve:latest
expect_fail 1.2.3 ghcr.io/acme/deve:1.2.4 ghcr.io/acme/deve:latest
expect_fail 1.2.3-rc.1 ghcr.io/acme/deve:1.2.3-rc.1 ghcr.io/acme/deve:latest
expect_fail 1.2.3+build.7 ghcr.io/acme/deve:1.2.3+build.7 ghcr.io/acme/deve:latest

echo "release-image-tags-test: ok"
