#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-boot-readiness.sh"

temporary="$(mktemp -d)"
cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT

mode=""
operations="$temporary/operations"
clock=0
slept="$temporary/slept"
device_probe_count="$temporary/device-probe-count"

android_emulator_boot_now() {
  printf '%s\n' "$clock"
}

sleep() {
  printf '%s\n' "$1" >"$slept"
  clock=$((clock + $1))
}

android_emulator_boot_probe() {
  local timeout_secs="$1"
  shift
  printf '%s\t%s\n' "$timeout_secs" "$*" >>"$operations"

  case "$*" in
    "devices")
      local count=0
      if [[ -f "$device_probe_count" ]]; then
        count="$(cat "$device_probe_count")"
      fi
      count=$((count + 1))
      printf '%s\n' "$count" >"$device_probe_count"
      printf '%s\n' "List of devices attached"
      case "$mode" in
        device-offline-then-ready)
          if (( count == 1 )); then
            printf '%s\n' "emulator-5584 offline"
          else
            printf '%s\n' "emulator-5584 device product:sdk_gphone_x86_64"
          fi
          ;;
        device-always-offline)
          printf '%s\n' "emulator-5584 offline"
          ;;
        device-line-then-timeout)
          printf '%s\n' "emulator-5584 device product:sdk_gphone_x86_64"
          return 124
          ;;
        avd-line-then-timeout)
          return 97
          ;;
        device-missing) ;;
        *) return 97 ;;
      esac
      ;;
    "-s emulator-5584 emu avd name")
      [[ "$mode" == "avd-line-then-timeout" ]] || return 96
      printf '%s\n' "deve-mobile-smoke-api37.0-google_apis-x86_64"
      return 124
      ;;
    "-s emulator-5584 shell getprop sys.boot_completed")
      [[ "$mode" == "boot-property-line-then-timeout" ]] || return 95
      printf '%s\n' "1"
      return 124
      ;;
    "-s emulator-5584 shell getprop dev.bootcomplete")
      [[ "$mode" == "boot-property-line-then-timeout" ]] || return 94
      printf '%s\n' "1"
      ;;
    "-s emulator-5584 shell cmd package list packages")
      printf '%s\n' "package:android"
      ;;
    *) return 99 ;;
  esac
}

expect_device_ready_after_offline() {
  : >"$operations"
  rm -f -- "$device_probe_count"
  mode=device-offline-then-ready
  clock=0
  android_emulator_wait_for_device_state "emulator-5584" 6
  [[ "$ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE" == "state=device" ]]
  [[ "$(cat "$device_probe_count")" == "2" ]]
  [[ "$(cat "$slept")" == "2" ]]
}

expect_device_deadline_rejects_offline() {
  : >"$operations"
  rm -f -- "$device_probe_count"
  mode=device-always-offline
  clock=0
  if android_emulator_wait_for_device_state "emulator-5584" 4; then
    echo "android-emulator-boot-readiness.test: offline device escaped deadline" >&2
    return 1
  fi
  [[ "$ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE" == "state=offline" ]]
  [[ "$(cat "$device_probe_count")" == "1" ]]
}

expect_process_guard_failure_stops_before_probe() {
  : >"$operations"
  rm -f -- "$device_probe_count"
  mode=device-offline-then-ready
  clock=0
  reject_dead_emulator() {
    return 23
  }
  local status=0
  android_emulator_wait_for_device_state \
    "emulator-5584" 6 reject_dead_emulator || status=$?
  [[ "$status" == "23" ]]
  [[ "$ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE" \
    == "process-guard=failed status=23" ]]
  [[ ! -e "$device_probe_count" ]]
}

expect_timed_out_probe_cannot_admit_device_line() {
  : >"$operations"
  rm -f -- "$device_probe_count"
  mode=device-line-then-timeout
  clock=0
  if android_emulator_wait_for_device_state "emulator-5584" 4; then
    echo "android-emulator-boot-readiness.test: timed-out device line was admitted" >&2
    return 1
  fi
  [[ "$ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE" \
    == "probe=failed status=124 last_state=not-observed" ]]
  [[ "$(cat "$device_probe_count")" == "1" ]]
}

expect_timed_out_capture_preserves_nonzero_status() {
  : >"$operations"
  mode=avd-line-then-timeout
  clock=0
  local status=0
  android_emulator_boot_capture_until 10 128 \
    -s emulator-5584 emu avd name || status=$?
  [[ "$status" == "124" ]]
  [[ "$ANDROID_EMULATOR_BOOT_CAPTURE_STATUS" == "124" ]]
  [[ "$ANDROID_EMULATOR_BOOT_CAPTURE_OUTPUT" \
    == "deve-mobile-smoke-api37.0-google_apis-x86_64" ]]
  if android_emulator_boot_avd_identity_matches \
      emulator-5584 deve-mobile-smoke-api37.0-google_apis-x86_64 10; then
    echo "android-emulator-boot-readiness.test: timed-out AVD identity was admitted" >&2
    return 1
  fi
  [[ "$ANDROID_EMULATOR_BOOT_AVD_IDENTITY_LAST_EVIDENCE" \
    == "probe=failed status=124" ]]
}

expect_timed_out_boot_property_cannot_signal_ready() {
  : >"$operations"
  mode=boot-property-line-then-timeout
  clock=0
  if android_emulator_boot_properties_ready emulator-5584 10; then
    echo "android-emulator-boot-readiness.test: timed-out boot property was admitted" >&2
    return 1
  fi
  [[ "$ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE" \
    == "boot-properties=unavailable sys_status=124 dev_status=0" ]]
}

android_boot_properties_complete "1" ""
android_boot_properties_complete "" "1"
if android_boot_properties_complete "" ""; then
  echo "android-emulator-boot-readiness.test: missing boot properties passed" >&2
  exit 1
fi
mode=ready-one
: >"$operations"
clock=97
android_emulator_boot_probe_until 100 \
  -s emulator-5584 shell cmd package list packages >/dev/null
[[ "$(cut -f 1 "$operations")" == "1" ]]
clock=98
if android_emulator_boot_probe_until 100 -s emulator-5584 shell cmd package list packages; then
  echo "android-emulator-boot-readiness.test: kill grace escaped the absolute deadline" >&2
  exit 1
fi
clock=98
android_emulator_boot_poll_sleep 100 5
[[ "$(cat "$slept")" == "2" ]]
clock=100
if android_emulator_boot_poll_sleep 100 5; then
  echo "android-emulator-boot-readiness.test: expired deadline admitted poll sleep" >&2
  exit 1
fi
clock=0

expect_device_ready_after_offline
expect_device_deadline_rejects_offline
expect_process_guard_failure_stops_before_probe
expect_timed_out_probe_cannot_admit_device_line
expect_timed_out_capture_preserves_nonzero_status
expect_timed_out_boot_property_cannot_signal_ready

echo "android-emulator-boot-readiness.test: ok"
