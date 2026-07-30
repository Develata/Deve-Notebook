#!/usr/bin/env bash
# Result and failure-diagnostic boundary for the manual Android emulator
# admission worker. The including script owns lifecycle decisions and provides
# the documented globals; this module only writes bounded observations.
#
# Summary globals:
# RESULT_DIR, RESULT_PATH, CYCLE_RESULT_DIR, EXPECTED_HEAD, VARIANT_ID,
# EMULATOR_SOURCE, EMULATOR_VERSION, EMULATOR_BUILD_ID,
# EMULATOR_PROBE_STATUS, SDK_EMULATOR_REVISION, API_LEVEL, SYSTEM_TARGET,
# SYSTEM_IMAGE_REVISION, ARCHITECTURE, APK_SHA256, REQUESTED_CYCLES.
#
# Diagnostic globals:
# ADB_BIN, EMULATOR_SERIAL.

ANDROID_ADMISSION_LOG_FILE_BUDGET_BYTES=131072
ANDROID_ADMISSION_VARIANT_LOG_BUDGET_BYTES=4194304

android_admission_bound_log_file() {
  local log_file="$1"
  local budget="${2:-$ANDROID_ADMISSION_LOG_FILE_BUDGET_BYTES}"
  local original_bytes half temporary
  [[ -f "$log_file" ]] || return 0
  original_bytes="$(wc -c <"$log_file" | tr -d '[:space:]')"
  (( original_bytes > budget )) || return 0
  half=$((budget / 2 - 64))
  temporary="$log_file.tmp.$$"
  {
    head -c "$half" "$log_file"
    printf '\n--- deve diagnostic log truncated: original_bytes=%s ---\n' "$original_bytes"
    tail -c "$half" "$log_file"
  } >"$temporary"
  head -c "$budget" "$temporary" >"$temporary.bounded"
  mv -f -- "$temporary.bounded" "$log_file"
  rm -f -- "$temporary"
}

android_admission_bound_cycle_logs() {
  local cycle_dir="$1"
  android_admission_bound_log_file "$cycle_dir/cycle.log"
  android_admission_bound_log_file "$cycle_dir/emulator.log"
}

android_admission_verify_variant_log_budget() {
  local result_dir="$1"
  local observed
  [[ -d "$result_dir" ]] || return 0
  observed="$(find "$result_dir" -type f \
    ! -path '*/cycle-results/*' \
    ! -path "$result_dir/*.json" \
    -printf '%s\n' \
    | awk '{ total += $1 } END { print total + 0 }')"
  (( observed <= ANDROID_ADMISSION_VARIANT_LOG_BUDGET_BYTES ))
}

android_admission_write_summary_result() {
  local complete="$1"
  local stable="$2"
  local harness_error="$3"
  local cycles_json="[]"
  local temporary="$RESULT_PATH.tmp.$$"
  local -a cycle_files=()

  mkdir -p "$RESULT_DIR" "$CYCLE_RESULT_DIR"
  shopt -s nullglob
  cycle_files=("$CYCLE_RESULT_DIR"/*.json)
  shopt -u nullglob
  if (( ${#cycle_files[@]} > 0 )); then
    cycles_json="$(jq -s 'sort_by(.cycle)' "${cycle_files[@]}")"
  fi
  jq -n \
    --argjson complete "$complete" \
    --argjson stable "$stable" \
    --arg harnessError "$harness_error" \
    --arg headSha "$EXPECTED_HEAD" \
    --arg variantId "$VARIANT_ID" \
    --arg emulatorSource "$EMULATOR_SOURCE" \
    --arg emulatorVersion "$EMULATOR_VERSION" \
    --arg emulatorBuildId "$EMULATOR_BUILD_ID" \
    --arg emulatorProbeStatus "$EMULATOR_PROBE_STATUS" \
    --arg sdkEmulatorRevision "$SDK_EMULATOR_REVISION" \
    --arg apiLevel "$API_LEVEL" \
    --arg systemTarget "$SYSTEM_TARGET" \
    --arg systemImageRevision "$SYSTEM_IMAGE_REVISION" \
    --arg architecture "$ARCHITECTURE" \
    --arg apkSha256 "$APK_SHA256" \
    --argjson requestedCycles "$REQUESTED_CYCLES" \
    --argjson cycles "$cycles_json" \
    '{
      schemaVersion: 1,
      kind: "android-emulator-admission-diagnostic",
      complete: $complete,
      headSha: $headSha,
      variantId: $variantId,
      emulatorSource: $emulatorSource,
      emulatorVersion: $emulatorVersion,
      emulatorBuildId: $emulatorBuildId,
      emulatorProbeStatus: $emulatorProbeStatus,
      sdkEmulatorRevision: $sdkEmulatorRevision,
      apiLevel: $apiLevel,
      systemTarget: $systemTarget,
      systemImageRevision: $systemImageRevision,
      architecture: $architecture,
      apkSha256: $apkSha256,
      requestedCycles: $requestedCycles,
      stable: $stable,
      harnessError: (if $harnessError == "" then null else $harnessError end),
      cycles: $cycles
    }' >"$temporary"
  mv -f -- "$temporary" "$RESULT_PATH"
}

android_admission_capture_bounded_command() {
  local output_file="$1"
  shift
  local status=0
  set +e
  timeout --signal=TERM --kill-after=2s 10s "$@" 2>&1 \
    | head -c 65536 >"$output_file"
  status="${PIPESTATUS[0]}"
  set -e
  printf '%s\n' "$status" >"$output_file.status"
}

android_admission_capture_cycle_diagnostics() {
  local diagnostic_dir="$1"
  mkdir -p "$diagnostic_dir"
  android_admission_capture_bounded_command \
    "$diagnostic_dir/adb-devices.log" "$ADB_BIN" devices
  android_admission_capture_bounded_command \
    "$diagnostic_dir/boot-properties.log" \
    "$ADB_BIN" -s "$EMULATOR_SERIAL" shell sh -c \
    'printf "sys="; getprop sys.boot_completed; printf "dev="; getprop dev.bootcomplete'
  android_admission_capture_bounded_command \
    "$diagnostic_dir/system-server.log" \
    "$ADB_BIN" -s "$EMULATOR_SERIAL" shell pidof system_server
  android_admission_capture_bounded_command \
    "$diagnostic_dir/service-list.log" \
    "$ADB_BIN" -s "$EMULATOR_SERIAL" shell service list
  android_admission_capture_bounded_command \
    "$diagnostic_dir/activity-services.log" \
    "$ADB_BIN" -s "$EMULATOR_SERIAL" shell dumpsys activity services
  android_admission_capture_bounded_command \
    "$diagnostic_dir/system-crash-logcat.log" \
    "$ADB_BIN" -s "$EMULATOR_SERIAL" logcat -b system -b crash -d -v threadtime
  android_admission_capture_bounded_command \
    "$diagnostic_dir/guest-meminfo.log" \
    "$ADB_BIN" -s "$EMULATOR_SERIAL" shell cat /proc/meminfo
  android_admission_capture_bounded_command \
    "$diagnostic_dir/guest-dmesg.log" \
    "$ADB_BIN" -s "$EMULATOR_SERIAL" shell dmesg
  android_admission_capture_bounded_command \
    "$diagnostic_dir/host-dmesg.log" dmesg
}

android_admission_classify_cycle_failure() {
  local log_file="$1"
  local phase="$2"
  if grep -Fq 'Failure calling service package: Broken pipe (32)' "$log_file"; then
    printf 'binder_epipe\n'
  elif grep -Fq "Can't find service: package" "$log_file"; then
    printf 'package_service_missing\n'
  elif grep -Eq "Can't find service: settings|Cannot access system provider: 'settings'" "$log_file"; then
    printf 'settings_service_unavailable\n'
  elif grep -Fq 'emulator process exited' "$log_file"; then
    printf 'emulator_process_exit\n'
  elif [[ "$phase" == boot* ]]; then
    printf 'boot_or_guest_admission\n'
  elif [[ "$phase" == install* ]]; then
    printf 'install_unclassified\n'
  elif [[ "$phase" == post-install* ]]; then
    printf 'post_install_instability\n'
  elif [[ "$phase" == cleanup* ]]; then
    printf 'cleanup_failure\n'
  else
    printf 'harness_or_unknown\n'
  fi
}
