#!/usr/bin/env bash
# Bounded process-ownership helpers for the manual Android admission diagnostic.
# The caller must set ROOT_DIR, EMULATOR_SERIAL, and AVD_NAME.

android_admission_write_emulator_owner() {
  local owner_file="$1"
  local pid="${2:-}"
  local launch_state="reserved"
  local temporary="$owner_file.tmp.$$"
  [[ -z "$pid" ]] || launch_state="launched"
  mkdir -p "$(dirname "$owner_file")"
  printf 'launch_state=%s\nemulator_pid=%s\nemulator_serial=%s\navd_name=%s\n' \
    "$launch_state" "$pid" "$EMULATOR_SERIAL" "$AVD_NAME" >"$temporary"
  mv -f -- "$temporary" "$owner_file"
}

android_admission_direct_child_alive() {
  local pid="${1:-}"
  [[ -n "$pid" ]] \
    && jobs -pr | grep -Fx -- "$pid" >/dev/null 2>&1
}

android_admission_cleanup_emulator() {
  local owner_file="$1"
  local cycle_dir="$2"
  local emulator_pid="${3:-}"
  local cleanup_script="${ANDROID_ADMISSION_CLEANUP_SCRIPT:-$ROOT_DIR/scripts/cleanup-mobile-android-emulator.sh}"
  local cleanup_status=0

  if [[ -f "$owner_file" ]]; then
    DEVE_MOBILE_ANDROID_EMULATOR_LOG_DIR="$cycle_dir" \
    DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE="$owner_file" \
      timeout --signal=TERM --kill-after=5s 45s \
      bash "$cleanup_script" || cleanup_status=$?
  else
    cleanup_status=1
  fi

  if android_admission_direct_child_alive "$emulator_pid"; then
    cleanup_status=1
    kill "$emulator_pid" >/dev/null 2>&1 || true
    for _ in 1 2 3 4 5; do
      android_admission_direct_child_alive "$emulator_pid" || break
      sleep 1
    done
    if android_admission_direct_child_alive "$emulator_pid"; then
      kill -KILL "$emulator_pid" >/dev/null 2>&1 || true
    fi
  fi
  [[ -z "$emulator_pid" ]] || wait "$emulator_pid" >/dev/null 2>&1 || true
  return "$cleanup_status"
}
