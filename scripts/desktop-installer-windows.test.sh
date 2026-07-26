#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/desktop-installer-windows.sh"

fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT

REGISTRY_OPERATION_TIMEOUT_SECS=2
bounded_mode="run"
run_bounded_command() {
  local _timeout_secs="$1"
  shift
  if [[ "$bounded_mode" == "timeout-all" ]]; then
    return 124
  fi
  if [[ "$1" == "reg.exe" ]]; then
    case "$bounded_mode:$2" in
      timeout-export:export | timeout-delete:delete | timeout-import:import)
        return 124
        ;;
    esac
  fi
  "$@"
}

windows_install_registry_subkey="Software\\deve\\Deve Notebook\\state-probe-$BASHPID-$RANDOM"
observed_subkey="$(
  printf '%s' "$windows_install_registry_subkey" |
    powershell.exe -NoProfile -NonInteractive -Command \
      '[Console]::Out.Write([Console]::In.ReadToEnd())'
)"
if [[ "$observed_subkey" != "$windows_install_registry_subkey" ]]; then
  echo "desktop-installer-windows-test: registry subkey stdin binding drifted: $observed_subkey" >&2
  exit 1
fi
actual_state=0
windows_install_registry_state || actual_state=$?
if [[ "$actual_state" != 3 ]]; then
  echo "desktop-installer-windows-test: absent registry probe returned $actual_state" >&2
  exit 1
fi
bounded_mode="timeout-all"
timed_out_state=0
windows_install_registry_state || timed_out_state=$?
[[ "$timed_out_state" == 4 ]]
bounded_mode="run"

windows_registry_cleanup_needed=1
windows_registry_existed=1
windows_registry_snapshot="$fixture/install.reg"
windows_install_registry_subkey='Software\deve\Deve Notebook'
windows_install_registry_key='HKCU\Software\deve\Deve Notebook'
touch "$windows_registry_snapshot"
preserved=""
reg_mode="delete-fails"
registry_state=0

is_windows_host() {
  return 0
}

preserve_failure_path() {
  preserved="$1"
}

windows_install_registry_state() {
  return "$registry_state"
}

reg.exe() {
  case "$1:$reg_mode" in
    delete:delete-fails) return 1 ;;
    export:pre-delete-fails | export:success)
      printf 'registry fixture\n' >"$3"
      return 0
      ;;
    delete:success | import:success) return 0 ;;
    *) return 1 ;;
  esac
}

if restore_windows_install_registry; then
  echo "desktop-installer-windows-test: restore failure was accepted" >&2
  exit 1
fi
[[ "$windows_registry_cleanup_needed" == 1 ]]
[[ "$preserved" == "$fixture" ]]

reg_mode="success"
preserved=""
restore_windows_install_registry
[[ "$windows_registry_cleanup_needed" == 0 ]]
[[ -z "$preserved" ]]

windows_registry_cleanup_needed=1
registry_state=4
preserved=""
if restore_windows_install_registry; then
  echo "desktop-installer-windows-test: unknown registry state was accepted" >&2
  exit 1
fi
[[ "$windows_registry_cleanup_needed" == 1 ]]
[[ "$preserved" == "$fixture" ]]

windows_registry_cleanup_needed=1
windows_registry_existed=1
registry_state=3
bounded_mode="timeout-import"
preserved=""
if restore_windows_install_registry; then
  echo "desktop-installer-windows-test: timed-out registry import was accepted" >&2
  exit 1
fi
[[ "$windows_registry_cleanup_needed" == 1 ]]
[[ "$preserved" == "$fixture" ]]
bounded_mode="run"

pre_delete_marker="$fixture/pre-delete-admitted"
failure_observation="$fixture/failure-observation"
WORK_ROOT="$fixture/snapshot-root"
cleanup_paths=()
windows_registry_cleanup_needed=0
windows_registry_existed=0
registry_state=0
reg_mode="success"
bounded_mode="timeout-delete"
to_windows_path() {
  printf '%s\n' "$1"
}
prepare_work_dir() {
  mkdir -p "$WORK_ROOT"
}
fail() {
  local snapshot_state="absent"
  if [[ -s "$windows_registry_snapshot" ]]; then
    snapshot_state="readable"
  fi
  printf '%s|%s\n' \
    "$windows_registry_cleanup_needed" \
    "$snapshot_state" >"$failure_observation"
  if [[ "$windows_registry_cleanup_needed" == 1 && -s "$windows_registry_snapshot" ]]; then
    touch "$pre_delete_marker"
  fi
  exit 1
}
status=0
(snapshot_windows_install_registry) || status=$?
[[ "$status" == 1 ]]
[[ -f "$pre_delete_marker" ]]
[[ "$(cat "$failure_observation")" == "1|readable" ]]

bounded_mode="timeout-export"
WORK_ROOT="$fixture/export-timeout-root"
cleanup_paths=()
windows_registry_cleanup_needed=0
windows_registry_existed=0
rm -f -- "$failure_observation"
status=0
(snapshot_windows_install_registry) || status=$?
[[ "$status" == 1 ]]
[[ "$(cat "$failure_observation")" == "0|absent" ]]

echo "desktop-installer-windows-test: ok"
