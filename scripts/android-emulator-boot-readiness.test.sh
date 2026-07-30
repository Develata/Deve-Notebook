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
      case "$mode" in
        package-missing)
          printf '%s\n' "cmd: Can't find service: package"
          return 20
          ;;
        package-mixed)
          printf '%s\n' "cmd: Can't find service: package" "error: device offline"
          return 20
          ;;
      esac
      printf '%s\n' "package:android"
      ;;
    "-s emulator-5584 shell settings get global device_provisioned")
      case "$mode" in
        ready-zero) printf '%s\r\n' "0" ;;
        ready-one) printf '%s\n' "1" ;;
        settings-missing)
          printf '%s\n' "cmd: Can't find service: settings"
          return 20
          ;;
        settings-provider-uninstalled)
          printf '%s\n' \
            "java.lang.IllegalStateException: Cannot access system provider: 'settings' before system providers are installed!"
          return 1
          ;;
        settings-timeout) return 124 ;;
        settings-null) printf '%s\n' "null" ;;
        settings-mixed) printf '%s\n' "1" "unexpected" ;;
        *) return 98 ;;
      esac
      ;;
    *) return 99 ;;
  esac
}

expect_ready() {
  local case_mode="$1"
  local expected_value="$2"
  : >"$operations"
  mode="$case_mode"
  android_emulator_guest_services_ready "emulator-5584" 20
  [[ "$ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE" \
    == "package-manager=ready settings-provider=ready device_provisioned=$expected_value" ]]
  [[ "$(wc -l <"$operations" | tr -d ' ')" == "2" ]]
  awk -F '\t' '$1 != 10 { exit 1 }' "$operations"
}

expect_rejected() {
  local case_mode="$1"
  local expected_operations="$2"
  local expected_reason="$3"
  : >"$operations"
  mode="$case_mode"
  if android_emulator_guest_services_ready "emulator-5584" 20; then
    echo "android-emulator-boot-readiness.test: $case_mode unexpectedly passed" >&2
    return 1
  fi
  [[ "$(wc -l <"$operations" | tr -d ' ')" == "$expected_operations" ]]
  [[ "$ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE" == "$expected_reason" ]]
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
expect_ready ready-zero 0
expect_ready ready-one 1
expect_rejected package-missing 1 "package-manager=unavailable status=20"
expect_rejected package-mixed 1 "package-manager=unavailable status=20"
expect_rejected settings-missing 2 "settings-provider=unavailable status=20"
expect_rejected settings-provider-uninstalled 2 "settings-provider=unavailable status=1"
expect_rejected settings-timeout 2 "settings-provider=unavailable status=124"
expect_rejected settings-null 2 "settings-provider=invalid device_provisioned=scalar"
expect_rejected settings-mixed 2 "settings-provider=invalid device_provisioned=multiline"

echo "android-emulator-boot-readiness.test: ok"
