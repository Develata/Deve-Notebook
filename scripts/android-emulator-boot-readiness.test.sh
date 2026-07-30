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

android_emulator_boot_now() {
  printf '%s\n' "$clock"
}

sleep() {
  printf '%s\n' "$1" >"$slept"
}

android_emulator_boot_probe() {
  local timeout_secs="$1"
  shift
  printf '%s\t%s\n' "$timeout_secs" "$*" >>"$operations"

  case "$*" in
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
