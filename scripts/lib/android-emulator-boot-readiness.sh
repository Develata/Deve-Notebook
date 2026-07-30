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
ANDROID_EMULATOR_BOOT_CAPTURE_OUTPUT=""
ANDROID_EMULATOR_BOOT_CAPTURE_STATUS="not-probed"
ANDROID_EMULATOR_BOOT_AVD_IDENTITY_LAST_EVIDENCE="not-probed"
ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE="not-probed"
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

android_emulator_boot_capture_until() {
  local deadline="$1"
  local max_bytes="$2"
  local output status
  shift 2

  if output="$(android_emulator_boot_probe_until "$deadline" "$@" \
      | head -c "$max_bytes")"; then
    status=0
  else
    status=$?
  fi
  ANDROID_EMULATOR_BOOT_CAPTURE_OUTPUT="$output"
  ANDROID_EMULATOR_BOOT_CAPTURE_STATUS="$status"
  return "$status"
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

android_emulator_adb_state_from_devices() {
  local serial="$1"
  awk -v serial="$serial" '
    $1 == serial {
      print (NF >= 2 ? $2 : "invalid")
      found = 1
      exit
    }
    END {
      if (!found) {
        print "missing"
      }
    }
  '
}

android_emulator_wait_for_device_state() {
  local serial="$1"
  local deadline="$2"
  local process_guard="${3:-:}"
  local devices_output guard_status probe_status state now remaining
  local last_state="not-observed"

  while :; do
    now="$(android_emulator_boot_now)"
    (( now < deadline )) || return 1
    remaining=$((deadline - now))
    (( remaining > ANDROID_EMULATOR_BOOT_PROBE_KILL_AFTER_SECS )) || return 1
    if "$process_guard"; then
      :
    else
      guard_status=$?
      ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE="process-guard=failed status=$guard_status"
      return "$guard_status"
    fi

    if android_emulator_boot_capture_until "$deadline" 4096 devices 2>/dev/null; then
      probe_status=0
    else
      probe_status=$?
    fi
    devices_output="$ANDROID_EMULATOR_BOOT_CAPTURE_OUTPUT"
    if (( probe_status != 0 )); then
      ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE="probe=failed status=$probe_status last_state=$last_state"
      android_emulator_boot_poll_sleep "$deadline" 2 || return 1
      continue
    fi

    state="$(printf '%s\n' "$devices_output" \
      | android_emulator_adb_state_from_devices "$serial" \
      | head -c 64)"
    last_state="$state"
    case "$state" in
      device)
        ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE="state=device"
        return 0
        ;;
      missing | offline | unauthorized | recovery | bootloader | sideload)
        ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE="state=$state"
        ;;
      *)
        ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE="state=invalid"
        ;;
    esac

    android_emulator_boot_poll_sleep "$deadline" 2 || return 1
  done
}

android_emulator_boot_avd_identity_matches() {
  local serial="$1"
  local expected_avd="$2"
  local deadline="$3"
  local observed_avd status=0

  if android_emulator_boot_capture_until "$deadline" 128 \
      -s "$serial" emu avd name 2>/dev/null; then
    status=0
  else
    status=$?
  fi
  if (( status != 0 )); then
    ANDROID_EMULATOR_BOOT_AVD_IDENTITY_LAST_EVIDENCE="probe=failed status=$status"
    return 1
  fi

  observed_avd="$(printf '%s\n' "$ANDROID_EMULATOR_BOOT_CAPTURE_OUTPUT" \
    | tr -d '\r' | head -n 1)"
  ANDROID_EMULATOR_BOOT_AVD_IDENTITY_LAST_EVIDENCE="observed=$observed_avd expected=$expected_avd"
  [[ "$observed_avd" == "$expected_avd" ]]
}

android_emulator_boot_properties_ready() {
  local serial="$1"
  local deadline="$2"
  local sys_boot_completed="" sys_boot_status=0
  local dev_boot_complete="" dev_boot_status=0

  if android_emulator_boot_capture_until "$deadline" 16 \
      -s "$serial" shell getprop sys.boot_completed 2>/dev/null; then
    sys_boot_status=0
  else
    sys_boot_status=$?
  fi
  sys_boot_completed="${ANDROID_EMULATOR_BOOT_CAPTURE_OUTPUT//$'\r'/}"
  if android_emulator_boot_capture_until "$deadline" 16 \
      -s "$serial" shell getprop dev.bootcomplete 2>/dev/null; then
    dev_boot_status=0
  else
    dev_boot_status=$?
  fi
  dev_boot_complete="${ANDROID_EMULATOR_BOOT_CAPTURE_OUTPUT//$'\r'/}"

  if (( sys_boot_status != 0 || dev_boot_status != 0 )); then
    ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="boot-properties=unavailable sys_status=$sys_boot_status dev_status=$dev_boot_status"
    return 1
  fi
  if android_boot_properties_complete "$sys_boot_completed" "$dev_boot_complete"; then
    return 0
  fi
  ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE="boot-properties=not-ready sys=${sys_boot_completed:-missing} dev=${dev_boot_complete:-missing}"
  return 1
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
