#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED:-0}"
BUNDLES="${DEVE_DESKTOP_PACKAGE_BUNDLES:-}"
NO_SIGN="${DEVE_DESKTOP_PACKAGE_NO_SIGN:-0}"
FEATURES="${DEVE_DESKTOP_PACKAGE_FEATURES:-native-packaging}"
SIDECAR_CONFIG="$ROOT_DIR/target/desktop-tauri-sidecar-config.json"

fail() {
  echo "desktop-platform-package-build-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

host_target_triple() {
  rustc -vV | awk '/^host: / {print $2; exit}'
}

host_os() {
  uname -s 2>/dev/null || printf 'unknown'
}

is_linux_host() {
  [[ "$(host_os)" == "Linux" ]]
}

missing=()

require_file() {
  local path="$1"
  [[ -f "$ROOT_DIR/$path" ]] || missing+=("$path")
}

require_command() {
  local label="$1"
  shift
  "$@" >/dev/null 2>&1 || missing+=("$label")
}

requires_bundle() {
  local bundle="$1"
  [[ -z "$BUNDLES" ]] && return 0
  [[ ",${BUNDLES// /,}," == *",$bundle,"* ]]
}

prepare_desktop_cli_sidecar() {
  local target_triple
  local src
  local dest
  local ext=""

  target_triple="$(host_target_triple)"
  [[ -n "$target_triple" ]] || fail "failed to resolve Rust host target triple"
  if [[ "$target_triple" == *windows* ]]; then
    ext=".exe"
  fi

  run cargo build --release --locked -p deve_cli --bin deve_cli

  src="$ROOT_DIR/target/release/deve_cli${ext}"
  dest="$ROOT_DIR/apps/desktop/binaries/deve_cli-${target_triple}${ext}"
  [[ -f "$src" ]] || fail "deve_cli sidecar build output is missing: ${src#$ROOT_DIR/}"
  mkdir -p "$ROOT_DIR/apps/desktop/binaries"
  cp "$src" "$dest"
  chmod +x "$dest" 2>/dev/null || true

  mkdir -p "$(dirname "$SIDECAR_CONFIG")"
  cat >"$SIDECAR_CONFIG" <<'JSON'
{
  "bundle": {
    "externalBin": ["binaries/deve_cli"]
  }
}
JSON
  echo "desktop-platform-package-build-check: prepared deve_cli sidecar ${dest#$ROOT_DIR/}"
}

run "$ROOT_DIR/scripts/check-desktop-package-preflight.sh"

echo "desktop-platform-package-build-check: host_os=$(host_os)"

require_file "apps/desktop/src/main.rs"
require_file "apps/desktop/build.rs"
require_file "apps/web/dist/index.html"
require_command "cargo tauri CLI" cargo tauri --version
if [[ "$REQUIRED" == "1" ]] && is_linux_host && requires_bundle "appimage"; then
  require_command "pkg-config librsvg-2.0 (install librsvg2-dev)" pkg-config --exists librsvg-2.0
fi

if ((${#missing[@]} > 0)); then
  for item in "${missing[@]}"; do
    echo "desktop-platform-package-build-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "platform package build prerequisites are incomplete"
  fi
  echo "desktop-platform-package-build-check: skip package build; set DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 on a target host to require it"
  echo "desktop-platform-package-build-check: ok"
  exit 0
fi

if [[ "$REQUIRED" != "1" ]]; then
  echo "desktop-platform-package-build-check: package prerequisites present; set DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 to run cargo tauri build"
  echo "desktop-platform-package-build-check: set DEVE_DESKTOP_PACKAGE_BUNDLES=app,dmg / msi,nsis / deb,rpm to verify a target-host subset"
  echo "desktop-platform-package-build-check: ok"
  exit 0
fi

prepare_desktop_cli_sidecar
(
  cd "$ROOT_DIR/apps/desktop"
  build_args=(--ci --config "$SIDECAR_CONFIG")
  if [[ -n "$FEATURES" ]]; then
    build_args+=(--features "$FEATURES")
  fi
  if [[ "$NO_SIGN" == "1" ]]; then
    build_args+=(--no-sign)
  fi
  if [[ -n "$BUNDLES" ]]; then
    build_args+=(--bundles "$BUNDLES")
  fi
  run cargo tauri build "${build_args[@]}"
)

echo "desktop-platform-package-build-check: ok"
