#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${DEVE_RELEASE_VERSION_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
TAG="${1:-${GITHUB_REF_NAME:-}}"
PYTHON_BIN="${DEVE_PYTHON_BIN:-python3}"

fail() {
  echo "release-version-match: $*" >&2
  exit 1
}

[[ "$TAG" == v* ]] || fail "release tag must start with v: ${TAG:-<empty>}"
command -v "$PYTHON_BIN" >/dev/null 2>&1 || fail "python interpreter not found: $PYTHON_BIN"

"$PYTHON_BIN" - "$ROOT_DIR" "${TAG#v}" <<'PY'
import json
import pathlib
import sys
import tomllib

root = pathlib.Path(sys.argv[1])
tag_version = sys.argv[2]

with (root / "Cargo.toml").open("rb") as handle:
    workspace_version = tomllib.load(handle)["workspace"]["package"]["version"]

def tauri_version(relative: str) -> str:
    with (root / relative).open(encoding="utf-8") as handle:
        value = json.load(handle).get("version")
    if not isinstance(value, str) or not value:
        raise SystemExit(f"release-version-match: missing non-empty version in {relative}")
    return value

versions = {
    "workspace": workspace_version,
    "desktop-tauri": tauri_version("apps/desktop/tauri.conf.json"),
    "mobile-tauri": tauri_version("apps/mobile/tauri.conf.json"),
}
mismatches = [f"{name}={version}" for name, version in versions.items() if version != tag_version]
if mismatches:
    raise SystemExit(
        "release-version-match: tag version "
        + repr(tag_version)
        + " does not exactly match "
        + ", ".join(mismatches)
    )
print(f"release-version-match: ok: {tag_version}")
PY
