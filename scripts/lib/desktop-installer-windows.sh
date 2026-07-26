# shellcheck shell=bash
#
# Windows MSI/NSIS install, packaged-app journey, and uninstall helpers.
# Sourced by check-desktop-installer-smoke.sh after shared lifecycle helpers.

to_windows_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s\n' "$1"
  fi
}

windows_install_registry_state() {
  local status=0

  if ! command -v powershell.exe >/dev/null 2>&1; then
    return 4
  fi
  printf '%s' "$windows_install_registry_subkey" |
    run_bounded_command "$REGISTRY_OPERATION_TIMEOUT_SECS" \
      powershell.exe -NoProfile -NonInteractive -Command '
      try {
        $subkey = [Console]::In.ReadToEnd()
        if ([string]::IsNullOrWhiteSpace($subkey)) { exit 4 }
        $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey(
          $subkey
        )
        if ($null -eq $key) { exit 3 }
        $key.Dispose()
        exit 0
      } catch {
        exit 4
      }
    ' >/dev/null 2>&1 || status=$?
  case "$status" in
    0 | 3) return "$status" ;;
    *) return 4 ;;
  esac
}

run_windows_registry_command() {
  local previous_arg_conv="${MSYS2_ARG_CONV_EXCL-}"
  local had_arg_conv=0
  local status

  [[ -v MSYS2_ARG_CONV_EXCL ]] && had_arg_conv=1
  export MSYS2_ARG_CONV_EXCL='*'
  if run_bounded_command "$REGISTRY_OPERATION_TIMEOUT_SECS" "$@"; then
    status=0
  else
    status=$?
  fi
  if ((had_arg_conv)); then
    export MSYS2_ARG_CONV_EXCL="$previous_arg_conv"
  else
    unset MSYS2_ARG_CONV_EXCL
  fi
  return "$status"
}

snapshot_windows_install_registry() {
  local snapshot_dir
  local state=0

  is_windows_host || return 0
  command -v reg.exe >/dev/null 2>&1 ||
    fail "Windows registry tool is unavailable before installer smoke"
  ((windows_registry_cleanup_needed == 0)) || return 0

  prepare_work_dir
  snapshot_dir="$WORK_ROOT/windows-registry-snapshot"
  mkdir -p "$snapshot_dir"
  cleanup_paths+=("$snapshot_dir")
  windows_registry_snapshot="$snapshot_dir/deve-notebook-install.reg"

  windows_install_registry_state || state=$?
  case "$state" in
    0) windows_registry_existed=1 ;;
    3) windows_registry_existed=0 ;;
    *) fail "failed to determine Windows install registry state before installer smoke" ;;
  esac
  if ((windows_registry_existed == 1)); then
    if ! run_windows_registry_command \
      reg.exe export "$windows_install_registry_key" "$(to_windows_path "$windows_registry_snapshot")" /y \
      >/dev/null 2>&1; then
      fail "failed to snapshot Windows install registry key before installer smoke"
    fi
    [[ -s "$windows_registry_snapshot" ]] ||
      fail "Windows install registry snapshot is missing or empty"
  fi

  echo "desktop-installer-smoke-check: isolating Windows install registry key $windows_install_registry_key"
  windows_registry_cleanup_needed=1
  if ((windows_registry_existed == 1)); then
    if ! run_windows_registry_command \
      reg.exe delete "$windows_install_registry_key" /f >/dev/null 2>&1; then
      fail "failed to clear Windows install registry key before installer smoke"
    fi
  fi
}

restore_windows_install_registry() {
  local snapshot_dir=""
  local state=0
  local status=0

  ((windows_registry_cleanup_needed == 1)) || return 0
  if ! is_windows_host ||
    ! command -v reg.exe >/dev/null 2>&1 ||
    ! command -v powershell.exe >/dev/null 2>&1; then
    echo "desktop-installer-smoke-check: Windows registry cleanup tool is unavailable" >&2
    if [[ -n "$windows_registry_snapshot" ]]; then
      preserve_failure_path "$(dirname "$windows_registry_snapshot")"
    fi
    return 1
  fi

  windows_install_registry_state || state=$?
  if ((state == 0)); then
    if ! run_windows_registry_command \
      reg.exe delete "$windows_install_registry_key" /f >/dev/null 2>&1; then
      echo "desktop-installer-smoke-check: failed to clear smoke-owned Windows install registry state during cleanup" >&2
      status=1
    fi
  elif ((state != 3)); then
    echo "desktop-installer-smoke-check: failed to determine Windows install registry state during cleanup" >&2
    status=1
  fi
  if ((status == 0 && windows_registry_existed == 1)); then
    if [[ ! -f "$windows_registry_snapshot" ]]; then
      echo "desktop-installer-smoke-check: Windows install registry snapshot is missing during cleanup" >&2
      status=1
    elif ! run_windows_registry_command \
      reg.exe import "$(to_windows_path "$windows_registry_snapshot")" >/dev/null 2>&1; then
      echo "desktop-installer-smoke-check: failed to restore Windows install registry snapshot" >&2
      status=1
    fi
  fi
  if ((status != 0)); then
    if [[ -n "$windows_registry_snapshot" ]]; then
      snapshot_dir="$(dirname "$windows_registry_snapshot")"
      preserve_failure_path "$snapshot_dir"
    fi
    return 1
  fi
  windows_registry_cleanup_needed=0
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
    if ! run_installed_git_unavailable_native_session_smoke "$exe"; then
      preserve_failure_path "$install_root"
      status=1
    fi
    if ! run_installed_git_bridge_smoke "$exe" "$install_root"; then
      preserve_failure_path "$install_root"
      status=1
    fi
    if ! run_installed_packaged_ui_smoke "$exe"; then
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
    if ! run_installed_git_unavailable_native_session_smoke "$exe"; then
      preserve_failure_path "$install_root"
      status=1
    fi
    if ! run_installed_git_bridge_smoke "$exe" "$install_root"; then
      preserve_failure_path "$install_root"
      status=1
    fi
    if ! run_installed_packaged_ui_smoke "$exe"; then
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
