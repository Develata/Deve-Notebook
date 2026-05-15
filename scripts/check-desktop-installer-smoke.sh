#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED:-0}"
BUNDLES="${DEVE_DESKTOP_PACKAGE_BUNDLES:-}"
STARTUP_TIMEOUT_SECS="${DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS:-20}"
INSTALLER_TIMEOUT_SECS="${DEVE_DESKTOP_INSTALLER_SMOKE_TIMEOUT_SECS:-180}"
TIMEOUT_KILL_AFTER_SECS="${DEVE_DESKTOP_INSTALLER_SMOKE_KILL_AFTER_SECS:-10}"
WORK_ROOT="${DEVE_DESKTOP_INSTALLER_SMOKE_WORK_DIR:-$ROOT_DIR/target/desktop-installer-smoke}"
SMOKE_ROOT_NAME="DeveNotebookInstallerSmoke"

cleanup_paths=()
cleanup_mounts=()
preserved_paths=()
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
    if command -v hdiutil >/dev/null 2>&1; then
      run_bounded_command 15 hdiutil detach "$mount" >/dev/null 2>&1 || true
    fi
  done
  for path in "${cleanup_paths[@]:-}"; do
    if path_is_preserved "$path"; then
      continue
    fi
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

path_is_preserved() {
  local candidate="$1"
  local preserved

  for preserved in "${preserved_paths[@]:-}"; do
    [[ "$candidate" == "$preserved" ]] && return 0
  done
  return 1
}

preserve_failure_path() {
  local path="$1"

  path_is_preserved "$path" && return 0
  preserved_paths+=("$path")
  echo "desktop-installer-smoke-check: preserving failure evidence at ${path#"$ROOT_DIR"/}" >&2
}

print_log_tail() {
  local log="$1"

  [[ -f "$log" ]] || return 0
  echo "desktop-installer-smoke-check: tail of ${log#"$ROOT_DIR"/}" >&2
  tail -n 120 "$log" >&2 || true
  echo "desktop-installer-smoke-check: path hints from ${log#"$ROOT_DIR"/}" >&2
  grep -Ei 'APPLICATIONFOLDER|INSTALLDIR|TARGETDIR|DeveNotebookInstallerSmoke|deve_desktop|Deve Notebook' "$log" \
    | tail -n 80 >&2 || true
}

terminate_child() {
  local child="$1"
  local signal="$2"

  kill "-$signal" "$child" >/dev/null 2>&1 || true
  kill "-$signal" "-$child" >/dev/null 2>&1 || true
  if is_windows_host && command -v taskkill.exe >/dev/null 2>&1; then
    taskkill.exe //PID "$child" //T //F >/dev/null 2>&1 || true
  fi
}

run_bounded_command() {
  local timeout_secs="$1"
  local child
  local elapsed=0
  local kill_wait=0
  local status
  shift

  if command -v gtimeout >/dev/null 2>&1; then
    gtimeout --kill-after="${TIMEOUT_KILL_AFTER_SECS}s" "${timeout_secs}s" "$@"
    return
  fi
  if command -v timeout >/dev/null 2>&1 && timeout --version >/dev/null 2>&1; then
    timeout --kill-after="${TIMEOUT_KILL_AFTER_SECS}s" "${timeout_secs}s" "$@"
    return
  fi

  set +e
  if ! is_windows_host && command -v setsid >/dev/null 2>&1; then
    setsid "$@" &
  else
    "$@" &
  fi
  child=$!
  while kill -0 "$child" >/dev/null 2>&1; do
    if ((elapsed >= timeout_secs)); then
      terminate_child "$child" TERM
      while kill -0 "$child" >/dev/null 2>&1 && ((kill_wait < TIMEOUT_KILL_AFTER_SECS)); do
        sleep 1
        kill_wait=$((kill_wait + 1))
      done
      terminate_child "$child" KILL
      wait "$child" >/dev/null 2>&1
      set -e
      return 124
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done
  wait "$child"
  status=$?
  set -e
  return "$status"
}

run_with_timeout() {
  local binary="$1"

  run_bounded_command "$STARTUP_TIMEOUT_SECS" env DEVE_DESKTOP_STARTUP_SMOKE=1 "$binary"
}

run_installer_command() {
  local label="$1"
  shift

  echo "+ $label"
  run_bounded_command "$INSTALLER_TIMEOUT_SECS" "$@"
}

run_windows_installer_command() {
  local label="$1"
  local previous_arg_conv="${MSYS2_ARG_CONV_EXCL-}"
  local had_arg_conv=0
  local status
  shift

  echo "+ $label"
  [[ -v MSYS2_ARG_CONV_EXCL ]] && had_arg_conv=1
  export MSYS2_ARG_CONV_EXCL='*'
  set +e
  run_bounded_command "$INSTALLER_TIMEOUT_SECS" "$@"
  status=$?
  set -e
  if ((had_arg_conv)); then
    export MSYS2_ARG_CONV_EXCL="$previous_arg_conv"
  else
    unset MSYS2_ARG_CONV_EXCL
  fi
  return "$status"
}

run_logged_windows_installer_command() {
  local log="$1"
  local label="$2"
  local output
  local status
  shift 2

  set +e
  output="$(run_windows_installer_command "$label" "$@" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$output"
  printf '%s\n' "$output" >"$log" || true
  return "$status"
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
  if ((status != 0)); then
    preserve_failure_path "$install_root"
  fi
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

  if ! run_installer_command "hdiutil attach ${dmg#"$ROOT_DIR"/}" \
    hdiutil attach "$dmg" -nobrowse -readonly -mountpoint "$mount_dir"; then
    preserve_failure_path "$mount_dir"
    return 1
  fi
  mounted_app="$(
    first_match "$mount_dir" -maxdepth 2 -name '*.app' -type d || true
  )"
  if [[ -z "$mounted_app" ]]; then
    echo "desktop-installer-smoke-check: mounted dmg contains no .app bundle" >&2
    status=1
  else
    smoke_macos_app_install "dmg app" "$mounted_app" || status=1
  fi

  if run_installer_command "hdiutil detach ${mount_dir#"$ROOT_DIR"/}" \
    hdiutil detach "$mount_dir"; then
    cleanup_mounts=("${cleanup_mounts[@]/$mount_dir}")
  else
    status=1
  fi
  if ((status != 0)); then
    preserve_failure_path "$mount_dir"
  fi
  return "$status"
}

to_windows_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s\n' "$1"
  fi
}

find_desktop_exe() {
  local root="$1"
  first_match "$root" -type f \( -iname 'deve_desktop.exe' -o -iname 'Deve Notebook.exe' \)
}

smoke_windows_msi_install() {
  local msi="$1"
  local install_root
  local install_dir
  local install_log
  local uninstall_log
  local exe=""
  local status=0

  prepare_work_dir
  install_root="$(mktemp -d "$WORK_ROOT/windows-msi.XXXXXX")"
  cleanup_paths+=("$install_root")
  install_dir="$install_root/$SMOKE_ROOT_NAME"
  install_log="$install_root/msiexec-install.log"
  uninstall_log="$install_root/msiexec-uninstall.log"

  if ! run_windows_installer_command \
    "msiexec.exe /i ${msi#"$ROOT_DIR"/} /qn /norestart ALLUSERS=2 MSIINSTALLPERUSER=1 APPLICATIONFOLDER=$(to_windows_path "$install_dir") INSTALLDIR=$(to_windows_path "$install_dir") /l*v $(to_windows_path "$install_log")" \
    msiexec.exe /i "$(to_windows_path "$msi")" /qn /norestart \
    ALLUSERS=2 \
    MSIINSTALLPERUSER=1 \
    "APPLICATIONFOLDER=$(to_windows_path "$install_dir")" \
    "INSTALLDIR=$(to_windows_path "$install_dir")" \
    /l*v "$(to_windows_path "$install_log")"; then
    print_log_tail "$install_log"
    preserve_failure_path "$install_root"
    return 1
  fi
  sleep 3
  exe="$(find_desktop_exe "$install_dir" || true)"
  if [[ -z "$exe" ]]; then
    echo "desktop-installer-smoke-check: MSI install completed but installed binary was not found under $install_dir" >&2
    print_log_tail "$install_log"
    preserve_failure_path "$install_root"
    status=1
  else
    if ! run_startup_probe "$exe"; then
      print_log_tail "$install_log"
      preserve_failure_path "$install_root"
      status=1
    fi
  fi

  if ! run_windows_installer_command \
    "msiexec.exe /x ${msi#"$ROOT_DIR"/} /qn /norestart /l*v $(to_windows_path "$uninstall_log")" \
    msiexec.exe /x "$(to_windows_path "$msi")" /qn /norestart \
    /l*v "$(to_windows_path "$uninstall_log")"; then
    print_log_tail "$uninstall_log"
    preserve_failure_path "$install_root"
    status=1
  fi
  sleep 3
  if [[ -n "$exe" && -f "$exe" ]]; then
    echo "desktop-installer-smoke-check: MSI uninstall completed but installed binary still exists: $exe" >&2
    print_log_tail "$uninstall_log"
    preserve_failure_path "$install_root"
    status=1
  fi
  return "$status"
}

smoke_windows_nsis_install() {
  local nsis="$1"
  local install_root
  local install_dir
  local install_log
  local uninstall_log
  local exe
  local uninstaller
  local status=0

  prepare_work_dir
  install_root="$(mktemp -d "$WORK_ROOT/windows-nsis.XXXXXX")"
  cleanup_paths+=("$install_root")
  install_dir="$install_root/$SMOKE_ROOT_NAME"
  install_log="$install_root/nsis-install.log"
  uninstall_log="$install_root/nsis-uninstall.log"

  if ! run_logged_windows_installer_command "$install_log" \
    "${nsis#"$ROOT_DIR"/} /S /D=$(to_windows_path "$install_dir")" \
    "$nsis" /S "/D=$(to_windows_path "$install_dir")"; then
    print_log_tail "$install_log"
    preserve_failure_path "$install_root"
    return 1
  fi
  sleep 3
  exe="$(find_desktop_exe "$install_dir" || true)"
  if [[ -z "$exe" ]]; then
    echo "desktop-installer-smoke-check: NSIS install completed but installed binary was not found" >&2
    print_log_tail "$install_log"
    preserve_failure_path "$install_root"
    status=1
  else
    if ! run_startup_probe "$exe"; then
      print_log_tail "$install_log"
      preserve_failure_path "$install_root"
      status=1
    fi
  fi

  uninstaller="$(
    first_match "$install_dir" -type f \( -iname 'uninstall.exe' -o -iname 'unins*.exe' \) || true
  )"
  if [[ -z "$uninstaller" ]]; then
    echo "desktop-installer-smoke-check: NSIS uninstaller was not found" >&2
    preserve_failure_path "$install_root"
    status=1
  else
    if ! run_logged_windows_installer_command "$uninstall_log" \
      "${uninstaller#"$ROOT_DIR"/} /S" \
      "$uninstaller" /S; then
      print_log_tail "$uninstall_log"
      preserve_failure_path "$install_root"
      status=1
    fi
    sleep 3
  fi
  if [[ -n "${exe:-}" && -f "$exe" ]]; then
    echo "desktop-installer-smoke-check: NSIS uninstall completed but installed binary still exists: $exe" >&2
    print_log_tail "$uninstall_log"
    preserve_failure_path "$install_root"
    status=1
  fi
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
