#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED:-0}"
BUNDLES="${DEVE_DESKTOP_PACKAGE_BUNDLES:-}"
TIMEOUT_SECS="${DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS:-20}"
WORK_ROOT="${DEVE_DESKTOP_INSTALLER_SMOKE_WORK_DIR:-$ROOT_DIR/target/desktop-installer-smoke}"
SMOKE_ROOT_NAME="DeveNotebookInstallerSmoke"

cleanup_paths=()
cleanup_mounts=()
missing=()
failures=()

fail() {
  echo "desktop-installer-smoke-check: $*" >&2
  exit 1
}

cleanup() {
  local mount
  local path
  for mount in "${cleanup_mounts[@]:-}"; do
    hdiutil detach "$mount" >/dev/null 2>&1 || true
  done
  for path in "${cleanup_paths[@]:-}"; do
    rm -rf "$path" >/dev/null 2>&1 || true
  done
}

trap cleanup EXIT

host_os() {
  uname -s 2>/dev/null || printf 'unknown'
}

is_windows_host() {
  [[ "${OS:-}" == "Windows_NT" ]] && return 0
  [[ "$(host_os)" == MINGW* || "$(host_os)" == MSYS* || "$(host_os)" == CYGWIN* ]]
}

requires_bundle() {
  local bundle="$1"
  [[ -z "$BUNDLES" ]] && return 0
  [[ ",${BUNDLES// /,}," == *",$bundle,"* ]]
}

validate_bundles() {
  local bundle
  [[ -n "$BUNDLES" ]] || return 0
  IFS=',' read -ra bundle_parts <<< "$BUNDLES"
  for bundle in "${bundle_parts[@]}"; do
    bundle="${bundle//[[:space:]]/}"
    case "$bundle" in
      app|dmg|msi|nsis) ;;
      "") fail "empty desktop installer bundle selector in DEVE_DESKTOP_PACKAGE_BUNDLES" ;;
      *) fail "unsupported desktop installer bundle selector: $bundle" ;;
    esac
  done
}

first_match() {
  local root="$1"
  shift
  [[ -d "$root" ]] || return 1
  find "$root" "$@" -print -quit
}

record_missing() {
  missing+=("$1")
}

record_failure() {
  failures+=("$1")
}

run_with_timeout() {
  local binary="$1"

  if command -v gtimeout >/dev/null 2>&1; then
    DEVE_DESKTOP_STARTUP_SMOKE=1 gtimeout "${TIMEOUT_SECS}s" "$binary"
    return
  fi
  if command -v timeout >/dev/null 2>&1 && timeout --version >/dev/null 2>&1; then
    DEVE_DESKTOP_STARTUP_SMOKE=1 timeout "${TIMEOUT_SECS}s" "$binary"
    return
  fi
  DEVE_DESKTOP_STARTUP_SMOKE=1 "$binary"
}

run_startup_probe() {
  local binary="$1"
  local output
  local status

  if [[ ! -x "$binary" ]]; then
    echo "desktop-installer-smoke-check: installed startup binary is not executable: $binary" >&2
    return 1
  fi

  set +e
  output="$(run_with_timeout "$binary" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$output"
  ((status == 0)) && [[ "$output" == *"desktop-startup-smoke: ok"* ]]
}

prepare_work_dir() {
  mkdir -p "$WORK_ROOT"
}

copy_bundle() {
  local src="$1"
  local dst="$2"

  if command -v ditto >/dev/null 2>&1; then
    ditto "$src" "$dst"
  else
    cp -R "$src" "$dst"
  fi
}

smoke_macos_app_install() {
  local label="$1"
  local app="$2"
  local install_root
  local installed_app
  local binary
  local status=0

  prepare_work_dir
  install_root="$(mktemp -d "$WORK_ROOT/macos-app.XXXXXX")"
  cleanup_paths+=("$install_root")
  installed_app="$install_root/Applications/$(basename "$app")"
  mkdir -p "$(dirname "$installed_app")"

  echo "+ install $label into ${installed_app#"$ROOT_DIR"/}"
  copy_bundle "$app" "$installed_app" || status=1
  binary="$installed_app/Contents/MacOS/deve_desktop"
  if ((status == 0)); then
    run_startup_probe "$binary" || status=1
  fi
  echo "+ uninstall $label from ${installed_app#"$ROOT_DIR"/}"
  rm -rf "$installed_app" || status=1
  [[ ! -e "$installed_app" ]] || status=1
  return "$status"
}

smoke_macos_dmg_install() {
  local dmg="$1"
  local mount_dir
  local mounted_app
  local status=0

  prepare_work_dir
  mount_dir="$(mktemp -d "$WORK_ROOT/dmg-mount.XXXXXX")"
  cleanup_paths+=("$mount_dir")
  cleanup_mounts+=("$mount_dir")

  echo "+ hdiutil attach ${dmg#"$ROOT_DIR"/}"
  hdiutil attach "$dmg" -nobrowse -readonly -mountpoint "$mount_dir" >/dev/null || return 1
  mounted_app="$(
    first_match "$mount_dir" -maxdepth 2 -name '*.app' -type d || true
  )"
  if [[ -z "$mounted_app" ]]; then
    echo "desktop-installer-smoke-check: mounted dmg contains no .app bundle" >&2
    status=1
  else
    smoke_macos_app_install "dmg app" "$mounted_app" || status=1
  fi

  echo "+ hdiutil detach ${mount_dir#"$ROOT_DIR"/}"
  hdiutil detach "$mount_dir" >/dev/null || status=1
  cleanup_mounts=("${cleanup_mounts[@]/$mount_dir}")
  return "$status"
}

to_windows_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s\n' "$1"
  fi
}

to_unix_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -u "$1"
  else
    printf '%s\n' "$1"
  fi
}

windows_env_path() {
  local name="$1"
  local value
  value="$(printenv "$name" 2>/dev/null || true)"
  [[ -n "$value" ]] || return 0
  to_unix_path "$value"
}

find_desktop_exe() {
  local root="$1"
  first_match "$root" -type f \( -iname 'deve_desktop.exe' -o -iname 'Deve Notebook.exe' \)
}

windows_program_files_root() {
  local root

  root="$(windows_env_path ProgramFiles)"
  if [[ -z "$root" ]]; then
    root="$(windows_env_path 'ProgramFiles(x86)')"
  fi
  if [[ -z "$root" ]]; then
    return 1
  fi
  printf '%s\n' "$root"
}

smoke_windows_msi_install() {
  local msi="$1"
  local install_root
  local install_dir
  local exe=""
  local status=0

  install_root="$(windows_program_files_root)" || return 1
  install_dir="$install_root/$SMOKE_ROOT_NAME"
  cleanup_paths+=("$install_dir")

  echo "+ msiexec.exe /i ${msi#"$ROOT_DIR"/} /qn /norestart APPLICATIONFOLDER=$(to_windows_path "$install_dir")"
  msiexec.exe /i "$(to_windows_path "$msi")" /qn /norestart "APPLICATIONFOLDER=$(to_windows_path "$install_dir")" || return 1
  sleep 3
  exe="$(find_desktop_exe "$install_dir" || true)"
  if [[ -z "$exe" ]]; then
    echo "desktop-installer-smoke-check: MSI install completed but installed binary was not found under $install_dir" >&2
    status=1
  else
    run_startup_probe "$exe" || status=1
  fi

  echo "+ msiexec.exe /x ${msi#"$ROOT_DIR"/} /qn /norestart"
  msiexec.exe /x "$(to_windows_path "$msi")" /qn /norestart || status=1
  sleep 3
  [[ -z "$exe" || ! -f "$exe" ]] || status=1
  return "$status"
}

smoke_windows_nsis_install() {
  local nsis="$1"
  local install_root
  local install_dir
  local exe
  local uninstaller
  local status=0

  prepare_work_dir
  install_root="$(mktemp -d "$WORK_ROOT/windows-nsis.XXXXXX")"
  cleanup_paths+=("$install_root")
  install_dir="$install_root/$SMOKE_ROOT_NAME"

  echo "+ ${nsis#"$ROOT_DIR"/} /S /D=$(to_windows_path "$install_dir")"
  "$nsis" /S "/D=$(to_windows_path "$install_dir")" || return 1
  sleep 3
  exe="$(find_desktop_exe "$install_dir" || true)"
  if [[ -z "$exe" ]]; then
    echo "desktop-installer-smoke-check: NSIS install completed but installed binary was not found" >&2
    status=1
  else
    run_startup_probe "$exe" || status=1
  fi

  uninstaller="$(
    first_match "$install_dir" -type f \( -iname 'uninstall.exe' -o -iname 'unins*.exe' \) || true
  )"
  if [[ -z "$uninstaller" ]]; then
    echo "desktop-installer-smoke-check: NSIS uninstaller was not found" >&2
    status=1
  else
    echo "+ ${uninstaller#"$ROOT_DIR"/} /S"
    "$uninstaller" /S || status=1
    sleep 3
  fi
  [[ -z "${exe:-}" || ! -f "$exe" ]] || status=1
  return "$status"
}

run_macos_smoke() {
  local app=""
  local dmg=""

  if requires_bundle app; then
    app="$(
      first_match "$ROOT_DIR/target/release/bundle/macos" -name '*.app' -type d || true
    )"
    [[ -n "$app" ]] || record_missing "macOS .app bundle target/release/bundle/macos/*.app"
  fi
  if requires_bundle dmg; then
    dmg="$(
      first_match "$ROOT_DIR/target/release/bundle/dmg" -name '*.dmg' -type f || true
    )"
    [[ -n "$dmg" ]] || record_missing "macOS dmg target/release/bundle/dmg/*.dmg"
  fi

  [[ -z "$app" ]] || smoke_macos_app_install ".app bundle" "$app" || record_failure "macOS .app install/uninstall smoke"
  [[ -z "$dmg" ]] || smoke_macos_dmg_install "$dmg" || record_failure "macOS dmg install/uninstall smoke"
}

run_windows_smoke() {
  local msi=""
  local nsis=""

  if requires_bundle msi; then
    msi="$(
      first_match "$ROOT_DIR/target/release/bundle/msi" -name '*.msi' -type f || true
    )"
    [[ -n "$msi" ]] || record_missing "Windows MSI target/release/bundle/msi/*.msi"
  fi
  if requires_bundle nsis; then
    nsis="$(
      first_match "$ROOT_DIR/target/release/bundle/nsis" -name '*.exe' -type f || true
    )"
    [[ -n "$nsis" ]] || record_missing "Windows NSIS target/release/bundle/nsis/*.exe"
  fi

  [[ -z "$msi" ]] || smoke_windows_msi_install "$msi" || record_failure "Windows MSI install/uninstall smoke"
  [[ -z "$nsis" ]] || smoke_windows_nsis_install "$nsis" || record_failure "Windows NSIS install/uninstall smoke"
}

echo "desktop-installer-smoke-check: host_os=$(host_os)"
validate_bundles

case "$(host_os)" in
  Darwin)
    run_macos_smoke
    ;;
  *)
    if is_windows_host; then
      run_windows_smoke
    elif [[ "$REQUIRED" == "1" ]]; then
      fail "desktop installer smoke requires macOS or Windows target host"
    else
      echo "desktop-installer-smoke-check: skip; installer smoke requires macOS or Windows target host"
      echo "desktop-installer-smoke-check: ok"
      exit 0
    fi
    ;;
esac

if ((${#missing[@]} > 0)); then
  for item in "${missing[@]}"; do
    echo "desktop-installer-smoke-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "desktop installer smoke prerequisites are incomplete"
  fi
  echo "desktop-installer-smoke-check: diagnostic-only; installer prerequisites are incomplete"
fi

if ((${#failures[@]} > 0)); then
  for item in "${failures[@]}"; do
    echo "desktop-installer-smoke-check: failed $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "desktop installer smoke failed"
  fi
  echo "desktop-installer-smoke-check: diagnostic-only; installer smoke failures were not required"
fi

echo "desktop-installer-smoke-check: ok"
