#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${DEVE_NATIVE_TOOL_BIN_DIR:-$ROOT_DIR/target/native-tools/bin}"
TRUNK_VERSION="${DEVE_TRUNK_VERSION:-0.21.14}"
TAURI_CLI_VERSION="${DEVE_TAURI_CLI_VERSION:-2.11.1}"
INSTALL_TRUNK="${DEVE_NATIVE_INSTALL_TRUNK:-1}"
INSTALL_TAURI_CLI="${DEVE_NATIVE_INSTALL_TAURI_CLI:-0}"

fail() {
  echo "native-target-host-tools-install: $*" >&2
  exit 1
}

host_triple() {
  local os
  local arch
  os="$(uname -s 2>/dev/null || printf unknown)"
  arch="$(uname -m 2>/dev/null || printf unknown)"

  if [[ "${OS:-}" == "Windows_NT" || "$os" == MINGW* || "$os" == MSYS* || "$os" == CYGWIN* ]]; then
    printf 'x86_64-pc-windows-msvc'
    return
  fi

  case "$os:$arch" in
    Darwin:arm64|Darwin:aarch64) printf 'aarch64-apple-darwin' ;;
    Darwin:x86_64) printf 'x86_64-apple-darwin' ;;
    Linux:aarch64|Linux:arm64) printf 'aarch64-unknown-linux-gnu' ;;
    Linux:x86_64) printf 'x86_64-unknown-linux-gnu' ;;
    *) fail "unsupported host for native tool binary install: os=$os arch=$arch" ;;
  esac
}

archive_ext() {
  local triple="$1"
  case "$triple" in
    *windows*) printf 'zip' ;;
    *linux*) printf 'tgz' ;;
    *darwin*) printf 'zip' ;;
    *) fail "unsupported tauri archive triple: $triple" ;;
  esac
}

trunk_ext() {
  local triple="$1"
  case "$triple" in
    *windows*) printf 'zip' ;;
    *) printf 'tar.gz' ;;
  esac
}

download() {
  local url="$1"
  local out="$2"

  command -v curl >/dev/null 2>&1 || fail "curl is required"
  curl -fsSL "$url" -o "$out"
}

extract_archive() {
  local archive="$1"
  local out_dir="$2"

  mkdir -p "$out_dir"
  case "$archive" in
    *.zip)
      if command -v unzip >/dev/null 2>&1; then
        unzip -q "$archive" -d "$out_dir"
      elif command -v powershell.exe >/dev/null 2>&1; then
        powershell.exe -NoProfile -Command "Expand-Archive -Path '$archive' -DestinationPath '$out_dir' -Force" >/dev/null
      else
        fail "unzip or powershell.exe is required to extract $archive"
      fi
      ;;
    *.tar.gz|*.tgz)
      tar -xzf "$archive" -C "$out_dir"
      ;;
    *) fail "unsupported archive: $archive" ;;
  esac
}

copy_tool_from_extract() {
  local extract_dir="$1"
  local tool_name="$2"
  local target_name="$3"
  local source_path

  source_path="$(
    find "$extract_dir" -type f \( -name "$tool_name" -o -name "$tool_name.exe" \) -print -quit
  )"
  [[ -n "$source_path" ]] || fail "extracted archive did not contain $tool_name"
  mkdir -p "$BIN_DIR"
  cp "$source_path" "$BIN_DIR/$target_name"
  chmod +x "$BIN_DIR/$target_name"
}

ensure_path() {
  export PATH="$BIN_DIR:$PATH"
  if [[ -n "${GITHUB_PATH:-}" ]]; then
    printf '%s\n' "$BIN_DIR" >>"$GITHUB_PATH"
  fi
}

has_trunk() {
  command -v trunk >/dev/null 2>&1 && trunk --version 2>/dev/null | grep -Fx "trunk $TRUNK_VERSION" >/dev/null
}

has_tauri_cli() {
  command -v cargo-tauri >/dev/null 2>&1 \
    && cargo tauri --version 2>/dev/null | grep -Fx "tauri-cli $TAURI_CLI_VERSION" >/dev/null
}

install_trunk() {
  local triple="$1"
  local ext
  local asset
  local tmp_dir
  local archive
  local exe_suffix=""

  ensure_path
  if has_trunk; then
    echo "native-target-host-tools-install: trunk $TRUNK_VERSION already available"
    return
  fi

  ext="$(trunk_ext "$triple")"
  asset="trunk-$triple.$ext"
  tmp_dir="$(mktemp -d)"
  archive="$tmp_dir/$asset"
  [[ "$triple" == *windows* ]] && exe_suffix=".exe"

  download "https://github.com/trunk-rs/trunk/releases/download/v$TRUNK_VERSION/$asset" "$archive"
  extract_archive "$archive" "$tmp_dir/extract"
  copy_tool_from_extract "$tmp_dir/extract" trunk "trunk$exe_suffix"
  ensure_path
  has_trunk || fail "installed trunk version mismatch"
  rm -rf "$tmp_dir"
  echo "native-target-host-tools-install: installed trunk $TRUNK_VERSION"
}

install_tauri_cli() {
  local triple="$1"
  local ext
  local asset
  local tmp_dir
  local archive
  local exe_suffix=""

  ensure_path
  if has_tauri_cli; then
    echo "native-target-host-tools-install: tauri-cli $TAURI_CLI_VERSION already available"
    return
  fi

  ext="$(archive_ext "$triple")"
  asset="cargo-tauri-$triple.$ext"
  tmp_dir="$(mktemp -d)"
  archive="$tmp_dir/$asset"
  [[ "$triple" == *windows* ]] && exe_suffix=".exe"

  download "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v$TAURI_CLI_VERSION/$asset" "$archive"
  extract_archive "$archive" "$tmp_dir/extract"
  copy_tool_from_extract "$tmp_dir/extract" cargo-tauri "cargo-tauri$exe_suffix"
  ensure_path
  has_tauri_cli || fail "installed tauri-cli version mismatch"
  rm -rf "$tmp_dir"
  echo "native-target-host-tools-install: installed tauri-cli $TAURI_CLI_VERSION"
}

TRIPLE="$(host_triple)"
echo "native-target-host-tools-install: host_triple=$TRIPLE"

mkdir -p "$BIN_DIR"
ensure_path

if [[ "$INSTALL_TRUNK" == "1" ]]; then
  install_trunk "$TRIPLE"
fi
if [[ "$INSTALL_TAURI_CLI" == "1" ]]; then
  install_tauri_cli "$TRIPLE"
fi

echo "native-target-host-tools-install: ok"
