#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-emulator-owner.sh"
LOG_DIR="${DEVE_MOBILE_ANDROID_EMULATOR_LOG_DIR:-$ROOT_DIR/target/mobile-android-emulator-smoke}"
OWNER_FILE="$(android_emulator_owner_file "$LOG_DIR")" || exit 1
CLEANUP_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_EMULATOR_CLEANUP_TIMEOUT_SECS:-30}"
source "$ROOT_DIR/scripts/lib/android-tools.sh"

[[ -f "$OWNER_FILE" ]] || {
  echo "mobile-android-emulator-cleanup: no owned emulator"
  exit 0
}

launch_state=""
emulator_pid=""
emulator_serial=""
avd_name=""
while IFS='=' read -r key value; do
  case "$key" in
    launch_state) launch_state="$value" ;;
    emulator_pid) emulator_pid="$value" ;;
    emulator_serial) emulator_serial="$value" ;;
    avd_name) avd_name="$value" ;;
    *) echo "mobile-android-emulator-cleanup: invalid owner field $key" >&2; exit 1 ;;
  esac
done <"$OWNER_FILE"

[[ "$launch_state" == "reserved" || "$launch_state" == "launched" ]] \
  || { echo "mobile-android-emulator-cleanup: invalid launch state" >&2; exit 1; }
[[ -z "$emulator_pid" || "$emulator_pid" =~ ^[1-9][0-9]*$ ]] \
  || { echo "mobile-android-emulator-cleanup: invalid owned PID" >&2; exit 1; }
[[ "$launch_state" == "reserved" && -z "$emulator_pid" \
    || "$launch_state" == "launched" && -n "$emulator_pid" ]] \
  || { echo "mobile-android-emulator-cleanup: launch state and PID disagree" >&2; exit 1; }
[[ "$emulator_serial" =~ ^emulator-[0-9]+$ ]] \
  || { echo "mobile-android-emulator-cleanup: invalid owned serial" >&2; exit 1; }
[[ "$avd_name" =~ ^[A-Za-z0-9._-]+$ ]] \
  || { echo "mobile-android-emulator-cleanup: invalid owned AVD" >&2; exit 1; }
[[ "$CLEANUP_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] \
  || { echo "mobile-android-emulator-cleanup: invalid cleanup timeout" >&2; exit 1; }
[[ "$launch_state" == "launched" ]] || {
  echo "mobile-android-emulator-cleanup: reserved launch has no termination authority" >&2
  exit 1
}

adb_cmd() {
  android_run_tool adb "$@"
}

serial_visible() {
  local devices
  devices="$(adb_cmd devices 2>/dev/null)" || {
    echo "mobile-android-emulator-cleanup: adb devices probe failed" >&2
    return 2
  }
  printf '%s\n' "$devices" \
    | awk -v serial="$emulator_serial" '$1 == serial { found = 1 } END { exit !found }'
}

pid_alive() {
  [[ -n "$emulator_pid" ]] && kill -0 "$emulator_pid" >/dev/null 2>&1
}

deadline=$((SECONDS + CLEANUP_TIMEOUT_SECS))
kill_requested=0
while (( SECONDS < deadline )); do
  serial_status=0
  serial_visible || serial_status=$?
  if (( serial_status == 2 )); then
    exit 1
  elif (( serial_status == 0 )); then
    if (( kill_requested == 0 )); then
      observed_avd="$(adb_cmd -s "$emulator_serial" emu avd name 2>/dev/null \
        | tr -d '\r' | head -n 1 || true)"
      [[ "$observed_avd" == "$avd_name" ]] || {
        echo "mobile-android-emulator-cleanup: serial $emulator_serial belongs to '$observed_avd', not owned '$avd_name'" >&2
        exit 1
      }
      if adb_cmd -s "$emulator_serial" emu kill >/dev/null 2>&1; then
        kill_requested=1
      fi
    fi
  elif [[ "$launch_state" == "launched" ]] && ! pid_alive; then
    rm -f -- "$OWNER_FILE"
    echo "mobile-android-emulator-cleanup: owned emulator stopped"
    exit 0
  fi
  sleep 1
done

serial_status=0
serial_visible || serial_status=$?
if (( serial_status == 2 )); then
  exit 1
fi
if (( serial_status == 0 )) || pid_alive; then
  echo "mobile-android-emulator-cleanup: owned emulator did not stop within ${CLEANUP_TIMEOUT_SECS}s" >&2
  exit 1
fi

rm -f -- "$OWNER_FILE"
echo "mobile-android-emulator-cleanup: owned emulator stopped"
