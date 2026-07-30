#!/usr/bin/env bash
# Read-only Android guest-service admission used after a boot-complete property
# appears and before any APK install. Every probe is independently time
# bounded; this library only accepts a package manager response plus a
# canonical settings-provider value.

if [[ -n "${ANDROID_EMULATOR_BOOT_READINESS_LOADED:-}" ]]; then
  return 0
fi
ANDROID_EMULATOR_BOOT_READINESS_LOADED=1

readonly ANDROID_EMULATOR_BOOT_PROBE_MAX_SECS=10
readonly ANDROID_EMULATOR_BOOT_PROBE_KILL_AFTER_SECS=2
ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="not-probed"

android_emulator_boot_probe() {
  local timeout_secs="$1"
  shift
  timeout --kill-after="${ANDROID_EMULATOR_BOOT_PROBE_KILL_AFTER_SECS}s" \
    "${timeout_secs}s" "$(android_tool_path adb)" "$@"
}

android_emulator_boot_now() {
  printf '%s\n' "$SECONDS"
}

android_emulator_boot_probe_until() {
  local deadline="$1"
  local now remaining timeout_secs
  shift

  now="$(android_emulator_boot_now)"
  remaining=$((deadline - now))
  (( remaining > ANDROID_EMULATOR_BOOT_PROBE_KILL_AFTER_SECS )) || return 124
  timeout_secs=$((remaining - ANDROID_EMULATOR_BOOT_PROBE_KILL_AFTER_SECS))
  (( timeout_secs <= ANDROID_EMULATOR_BOOT_PROBE_MAX_SECS )) \
    || timeout_secs="$ANDROID_EMULATOR_BOOT_PROBE_MAX_SECS"
  android_emulator_boot_probe "$timeout_secs" "$@"
}

android_emulator_boot_poll_sleep() {
  local deadline="$1"
  local interval="$2"
  local now remaining

  now="$(android_emulator_boot_now)"
  remaining=$((deadline - now))
  (( remaining > 0 )) || return 1
  (( interval <= remaining )) || interval="$remaining"
  sleep "$interval"
}

android_emulator_guest_services_ready() {
  local serial="$1"
  local deadline="$2"
  local output status

  if android_emulator_boot_probe_until "$deadline" \
      -s "$serial" shell cmd package list packages >/dev/null 2>&1; then
    :
  else
    status=$?
    ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="package-manager=unavailable status=$status"
    return 1
  fi

  if output="$(android_emulator_boot_probe_until "$deadline" \
      -s "$serial" shell settings get global device_provisioned 2>&1 | head -c 128)"; then
    :
  else
    status=$?
    ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="settings-provider=unavailable status=$status"
    return 1
  fi
  output="${output//$'\r'/}"

  case "$output" in
    0 | 1)
      ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="package-manager=ready settings-provider=ready device_provisioned=$output"
      return 0
      ;;
    *$'\n'*)
      ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="settings-provider=invalid device_provisioned=multiline"
      ;;
    "")
      ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="settings-provider=invalid device_provisioned=empty"
      ;;
    *)
      ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="settings-provider=invalid device_provisioned=scalar"
      ;;
  esac
  return 1
}
