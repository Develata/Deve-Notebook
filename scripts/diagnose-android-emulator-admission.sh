#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-owner.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-boot-readiness.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-capacity.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-pin.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-renderer.sh"
source "$ROOT_DIR/scripts/lib/android-install-retry.sh"
source "$ROOT_DIR/scripts/lib/android-admission-diagnostic-result.sh"
source "$ROOT_DIR/scripts/lib/android-admission-emulator-lifecycle.sh"

REQUIRED="${DEVE_ANDROID_ADMISSION_DIAGNOSTIC_REQUIRED:-0}"
VARIANT_ID="${DEVE_ANDROID_ADMISSION_VARIANT_ID:-pinned-api37-swangle}"
EMULATOR_SOURCE="${DEVE_ANDROID_ADMISSION_EMULATOR_SOURCE:-pinned}"
API_LEVEL="${DEVE_ANDROID_ADMISSION_API_LEVEL:-37.0}"
GPU_MODE="${DEVE_ANDROID_ADMISSION_GPU_MODE:-swangle}"
SYSTEM_TARGET="${DEVE_ANDROID_ADMISSION_SYSTEM_TARGET:-google_apis}"
ARCHITECTURE="${DEVE_ANDROID_ADMISSION_ARCHITECTURE:-x86_64}"
REQUESTED_CYCLES="${DEVE_ANDROID_ADMISSION_CYCLES:-3}"
EXPECTED_HEAD="${DEVE_ANDROID_ADMISSION_HEAD_SHA:-}"
APK_PATH="${DEVE_ANDROID_ADMISSION_APK_PATH:-}"
RESULT_DIR="${DEVE_ANDROID_ADMISSION_RESULT_DIR:-$ROOT_DIR/target/android-emulator-admission/$VARIANT_ID}"
AVD_HOME="${DEVE_ANDROID_ADMISSION_AVD_HOME:-$ROOT_DIR/target/android-emulator-admission-avd/$VARIANT_ID}"
EMULATOR_PORT="${DEVE_ANDROID_ADMISSION_EMULATOR_PORT:-5584}"
EMULATOR_SERIAL="emulator-$EMULATOR_PORT"
EMULATOR_RAM_MB="${DEVE_ANDROID_ADMISSION_EMULATOR_RAM_MB:-4096}"
EMULATOR_PARTITION_MB="${DEVE_ANDROID_ADMISSION_EMULATOR_PARTITION_MB:-4096}"
BOOT_TIMEOUT_SECS="${DEVE_ANDROID_ADMISSION_BOOT_TIMEOUT_SECS:-900}"
POST_INSTALL_TIMEOUT_SECS="${DEVE_ANDROID_ADMISSION_POST_INSTALL_TIMEOUT_SECS:-90}"
ADB_TIMEOUT_SECS="${DEVE_ANDROID_ADMISSION_ADB_TIMEOUT_SECS:-120}"
ADB_KILL_AFTER_SECS=5
APP_ID="dev.deve.notebook.mobile"
AVD_NAME="deve-admission-${VARIANT_ID}-${ARCHITECTURE}"
RESULT_PATH="$RESULT_DIR/result.json"
CYCLE_RESULT_DIR="$RESULT_DIR/cycle-results"
RESULT_WRITTEN=0
CURRENT_PHASE="initialization"
HARNESS_ERROR=""
EMULATOR_BIN=""
EMULATOR_VERSION=""
EMULATOR_BUILD_ID=""
EMULATOR_PROBE_STATUS=""
SDK_EMULATOR_REVISION=""
SYSTEM_IMAGE_REVISION=""
APK_SHA256=""
ADB_BIN=""

log() {
  printf 'android-emulator-admission[%s]: %s\n' "$VARIANT_ID" "$*"
}

fail() {
  HARNESS_ERROR="$*"
  printf 'android-emulator-admission[%s]: %s\n' "$VARIANT_ID" "$*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 is required"
}

sdkmanager_cmd() {
  android_prepare_java_home || fail "Java 17+ is required for sdkmanager"
  android_run_tool sdkmanager "$@"
}

avdmanager_cmd() {
  android_prepare_java_home || fail "Java 17+ is required for avdmanager"
  android_run_tool avdmanager "$@"
}

read_package_revision() {
  local source_properties="$1"
  [[ -f "$source_properties" ]] || {
    printf 'unknown\n'
    return 0
  }
  sed -n 's/^Pkg\.Revision[[:space:]]*=[[:space:]]*//p' "$source_properties" \
    | head -n 1 \
    | tr -d '\r' \
    | cut -c 1-64
}

finish() {
  local status=$?
  local error="$HARNESS_ERROR"
  local cycle_dir
  trap - EXIT
  android_admission_bound_log_file "$RESULT_DIR/avdmanager.log" || status=1
  for cycle_dir in "$RESULT_DIR"/cycle-[0-9]*; do
    [[ -d "$cycle_dir" ]] || continue
    android_admission_bound_cycle_logs "$cycle_dir" || status=1
  done
  if ! android_admission_verify_variant_log_budget "$RESULT_DIR"; then
    status=1
    error="variant diagnostic output exceeded ${ANDROID_ADMISSION_VARIANT_LOG_BUDGET_BYTES} bytes"
  fi
  if (( RESULT_WRITTEN == 0 )); then
    [[ -n "$error" ]] || error="unexpected failure during $CURRENT_PHASE"
    error="$(printf '%s' "$error" | head -c 512)"
    android_admission_write_summary_result false false "$error" || status=1
  fi
  exit "$status"
}

validate_inputs() {
  [[ "$VARIANT_ID" =~ ^[a-z0-9][a-z0-9-]{0,63}$ ]] \
    || fail "variant id must match [a-z0-9-] and be at most 64 bytes"
  RESULT_PATH="$RESULT_DIR/$VARIANT_ID.json"
  case "$EMULATOR_SOURCE" in
    pinned | sdk) ;;
    *) fail "emulator source must be pinned or sdk" ;;
  esac
  case "$GPU_MODE" in
    swangle | software | swiftshader) ;;
    *) fail "GPU mode must be swangle, software, or swiftshader" ;;
  esac
  [[ "$API_LEVEL" =~ ^[0-9]{2}([.][0-9])?$ ]] || fail "API level is invalid"
  [[ "$SYSTEM_TARGET" =~ ^[a-z0-9_]+$ ]] || fail "system target is invalid"
  [[ "$ARCHITECTURE" == "x86_64" ]] || fail "diagnostic architecture must be x86_64"
  [[ "$REQUESTED_CYCLES" == "3" ]] || fail "cold-boot cycles must be exactly 3"
  [[ "$EMULATOR_PORT" =~ ^[0-9]+$ ]] \
    && (( EMULATOR_PORT >= 5554 && EMULATOR_PORT <= 5682 && EMULATOR_PORT % 2 == 0 )) \
    || fail "emulator port must be even and in 5554..5682"
  [[ "$EMULATOR_RAM_MB" =~ ^[0-9]+$ ]] \
    && (( EMULATOR_RAM_MB >= 1536 && EMULATOR_RAM_MB <= 4096 )) \
    || fail "emulator RAM must be in 1536..4096 MiB"
  [[ "$EMULATOR_PARTITION_MB" =~ ^[0-9]+$ ]] \
    && (( EMULATOR_PARTITION_MB >= 2048 && EMULATOR_PARTITION_MB <= 8192 )) \
    || fail "emulator partition must be in 2048..8192 MiB"
  [[ "$BOOT_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] || fail "boot timeout must be positive"
  [[ "$POST_INSTALL_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] \
    || fail "post-install timeout must be positive"
  [[ "$ADB_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] || fail "ADB timeout must be positive"
  [[ "$EXPECTED_HEAD" =~ ^[0-9a-f]{40}$ ]] || fail "exact lowercase HEAD SHA is required"
  [[ "$(git -C "$ROOT_DIR" rev-parse HEAD)" == "$EXPECTED_HEAD" ]] \
    || fail "checked-out HEAD does not match diagnostic HEAD"
  [[ -f "$APK_PATH" ]] || fail "exact diagnostic APK is missing: $APK_PATH"
  APK_PATH="$(cd "$(dirname "$APK_PATH")" && pwd)/$(basename "$APK_PATH")"
}

install_sdk_packages() {
  local system_image="system-images;android-$API_LEVEL;$SYSTEM_TARGET;$ARCHITECTURE"
  local -a packages=(
    "platform-tools"
    "emulator"
    "platforms;android-$API_LEVEL"
    "$system_image"
  )
  local attempt installed=0 sdk_log

  yes | sdkmanager_cmd --licenses >/dev/null || true
  for attempt in 1 2 3; do
    log "sdkmanager attempt $attempt/3 for API $API_LEVEL"
    sdk_log="$RESULT_DIR/sdkmanager-attempt-$attempt.log"
    if sdkmanager_cmd "${packages[@]}" >"$sdk_log" 2>&1; then
      installed=1
    fi
    android_admission_bound_log_file "$sdk_log"
    head -c "$ANDROID_ADMISSION_LOG_FILE_BUDGET_BYTES" "$sdk_log"
    (( installed == 0 )) || break
    (( attempt == 3 )) || sleep 2
  done
  (( installed == 1 )) || fail "SDK package installation failed after 3 attempts"
  local sdk_root
  sdk_root="$(android_sdk_root)" || fail "Android SDK root is unavailable"
  SDK_EMULATOR_REVISION="$(read_package_revision "$sdk_root/emulator/source.properties")"
  SYSTEM_IMAGE_REVISION="$(read_package_revision \
    "$sdk_root/system-images/android-$API_LEVEL/$SYSTEM_TARGET/$ARCHITECTURE/source.properties")"
}

capture_emulator_identity() {
  local probe_dir probe_file bytes banner
  local -a probe_status=()
  probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/deve-android-admission-identity.XXXXXX")"
  probe_file="$probe_dir/emulator-version.log"
  {
    timeout --signal=TERM --kill-after=5s 15s "$EMULATOR_BIN" -version 2>&1 \
      | head -c 65537 >"$probe_file"
    probe_status=("${PIPESTATUS[@]}")
  } || :
  EMULATOR_PROBE_STATUS="${probe_status[0]:-1}"
  bytes="$(wc -c <"$probe_file" | tr -d '[:space:]')"
  (( bytes <= 65536 )) || {
    rm -rf -- "$probe_dir"
    fail "emulator identity output exceeded 65536 bytes"
  }
  banner="$(tr -d '\r' <"$probe_file" \
    | grep -aE -m 1 '^[[:space:]]*Android emulator version [0-9]+([.][0-9]+){3} [(]build_id [0-9]+[)]([[:space:]]|$)' \
    || true)"
  rm -rf -- "$probe_dir"
  [[ "$banner" =~ Android[[:space:]]emulator[[:space:]]version[[:space:]]([0-9]+([.][0-9]+){3})[[:space:]]\(build_id[[:space:]]([0-9]+)\) ]] \
    || fail "emulator identity did not contain one canonical version/build banner"
  EMULATOR_VERSION="${BASH_REMATCH[1]}"
  EMULATOR_BUILD_ID="${BASH_REMATCH[3]}"
}

resolve_emulator() {
  case "$EMULATOR_SOURCE" in
    pinned)
      EMULATOR_BIN="$(android_resolve_pinned_emulator)" \
        || fail "checksum-pinned emulator could not be resolved"
      android_emulator_pin_matches "$EMULATOR_BIN" \
        || fail "resolved pinned emulator identity drifted: $ANDROID_EMULATOR_PIN_LAST_PROBE"
      ;;
    sdk)
      EMULATOR_BIN="$(android_tool_path emulator)" \
        || fail "SDK emulator binary is unavailable"
      ;;
  esac
  capture_emulator_identity
  log "emulator_source=$EMULATOR_SOURCE version=$EMULATOR_VERSION build_id=$EMULATOR_BUILD_ID probe_exit=$EMULATOR_PROBE_STATUS"
}

ensure_avd() {
  local system_image="system-images;android-$API_LEVEL;$SYSTEM_TARGET;$ARCHITECTURE"
  local config_file temporary
  mkdir -p "$AVD_HOME"
  export ANDROID_AVD_HOME="$AVD_HOME"
  if ! printf 'no\n' | avdmanager_cmd create avd \
      --force \
      --name "$AVD_NAME" \
      --package "$system_image" \
      --device pixel_2 >"$RESULT_DIR/avdmanager.log" 2>&1; then
    android_admission_bound_log_file "$RESULT_DIR/avdmanager.log"
    fail "AVD creation failed"
  fi
  android_admission_bound_log_file "$RESULT_DIR/avdmanager.log"
  "$EMULATOR_BIN" -list-avds | grep -Fx -- "$AVD_NAME" >/dev/null \
    || fail "created AVD is not visible to selected emulator"
  config_file="$AVD_HOME/$AVD_NAME.avd/config.ini"
  [[ -f "$config_file" ]] || fail "AVD config.ini is missing"
  temporary="$config_file.tmp.$$"
  grep -v '^disk\.dataPartition\.size=' "$config_file" >"$temporary" || true
  printf 'disk.dataPartition.size=%sM\n' "$EMULATOR_PARTITION_MB" >>"$temporary"
  mv -f -- "$temporary" "$config_file"
}

adb_with_timeout() {
  local timeout_secs="$1"
  shift
  timeout --signal=TERM --kill-after="${ADB_KILL_AFTER_SECS}s" \
    "${timeout_secs}s" "$ADB_BIN" "$@"
}

ensure_cycle_process_alive() {
  if [[ -n "${EMULATOR_PID:-}" ]] && kill -0 "$EMULATOR_PID" >/dev/null 2>&1; then
    return 0
  fi
  echo "owned Android emulator process exited" >&2
  return 1
}

wait_for_cycle_boot() {
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECS))
  android_emulator_wait_for_device_state \
    "$EMULATOR_SERIAL" "$deadline" ensure_cycle_process_alive
  android_emulator_boot_avd_identity_matches \
    "$EMULATOR_SERIAL" "$AVD_NAME" "$deadline"
  while (( SECONDS < deadline )); do
    ensure_cycle_process_alive
    if android_emulator_boot_properties_ready "$EMULATOR_SERIAL" "$deadline"; then
      break
    fi
    android_emulator_boot_poll_sleep "$deadline" 5
  done
  android_emulator_boot_properties_ready "$EMULATOR_SERIAL" "$deadline"
  android_emulator_wait_for_guest_services_stable \
    "$EMULATOR_SERIAL" "$deadline" ensure_cycle_process_alive
}

verify_data_capacity() {
  local output parsed total_kib available_kib minimum_total_kib
  output="$(adb_with_timeout 15 -s "$EMULATOR_SERIAL" shell "df -k /data" 2>&1)"
  parsed="$(printf '%s\n' "$output" | parse_android_emulator_data_capacity)"
  read -r total_kib available_kib <<<"$parsed"
  minimum_total_kib=$((EMULATOR_PARTITION_MB * 1024 * 3 / 4))
  (( total_kib >= minimum_total_kib && available_kib >= 1048576 ))
}

system_server_pid() {
  local output
  output="$(adb_with_timeout 10 -s "$EMULATOR_SERIAL" shell pidof system_server 2>/dev/null \
    | tr -d '\r' | head -n 1)"
  [[ "$output" =~ ^[1-9][0-9]*$ ]] || return 1
  printf '%s\n' "$output"
}

run_cycle() (
  set -euo pipefail
  # Implicit errexit unwinds function locals before the subshell EXIT trap.
  # Keep finalizer context private to this already-isolated subshell instead.
  cycle="$1"
  cycle_dir="$RESULT_DIR/cycle-$cycle"
  cycle_log="$cycle_dir/cycle.log"
  owner_file=""
  started_at="" completed_at="" status=0 primary_status=0 cleanup_status=0 failure_class=""
  system_pid_before="" system_pid_after="" renderer_pair=""
  cycle_phase="launch" emulator_pid="" temporary=""

  mkdir -p "$cycle_dir"
  owner_file="$(DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE= \
    android_emulator_owner_file "$cycle_dir")"
  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  cycle_finish() {
    primary_status=$?
    trap - EXIT INT TERM
    status="$primary_status"
    if (( status != 0 )); then
      android_admission_capture_cycle_diagnostics "$cycle_dir/diagnostics" || true
    fi
    android_admission_cleanup_emulator "$owner_file" "$cycle_dir" "$emulator_pid" \
      || cleanup_status=$?
    if (( primary_status == 0 && cleanup_status != 0 )); then
      status=1
      cycle_phase="cleanup"
    fi
    if android_emulator_renderer_observe "$cycle_dir/emulator.log"; then
      renderer_pair="$ANDROID_EMULATOR_RENDERER_LAST_MODE"
    else
      renderer_pair=""
      if (( status == 0 )); then
        status=1
        cycle_phase="renderer-finalization"
      fi
      echo "final renderer observation failed: $ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" >&2
    fi
    completed_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    if (( status != 0 )); then
      if (( primary_status == 0 && cleanup_status != 0 )); then
        failure_class="cleanup_failure"
      else
        failure_class="$(android_admission_classify_cycle_failure "$cycle_log" "$cycle_phase")"
      fi
    fi
    temporary="$CYCLE_RESULT_DIR/$VARIANT_ID-cycle-$cycle.json.tmp.$$"
    jq -n \
      --argjson cycle "$cycle" \
      --arg outcome "$([[ $status -eq 0 ]] && printf passed || printf failed)" \
      --arg phase "$([[ $status -eq 0 ]] && printf complete || printf '%s' "$cycle_phase")" \
      --argjson exitStatus "$status" \
      --argjson cleanupStatus "$cleanup_status" \
      --arg failureClass "$failure_class" \
      --arg startedAt "$started_at" \
      --arg completedAt "$completed_at" \
      --arg systemServerPidBefore "$system_pid_before" \
      --arg systemServerPidAfter "$system_pid_after" \
      --arg rendererPair "$renderer_pair" \
      '{
        cycle: $cycle,
        outcome: $outcome,
        phase: $phase,
        exitStatus: $exitStatus,
        cleanupStatus: $cleanupStatus,
        failureClass: (if $failureClass == "" then null else $failureClass end),
        startedAt: $startedAt,
        completedAt: $completedAt,
        systemServerPidBefore: (if $systemServerPidBefore == "" then null else $systemServerPidBefore end),
        systemServerPidAfter: (if $systemServerPidAfter == "" then null else $systemServerPidAfter end),
        rendererPair: (if $rendererPair == "" then null else $rendererPair end)
      }' >"$temporary"
    mv -f -- "$temporary" "$CYCLE_RESULT_DIR/$VARIANT_ID-cycle-$cycle.json"
    exit "$status"
  }
  trap cycle_finish EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  if adb_with_timeout 10 devices | awk -v serial="$EMULATOR_SERIAL" \
      '$1 == serial { found = 1 } END { exit !found }'; then
    echo "dedicated emulator serial is already in use: $EMULATOR_SERIAL" >&2
    exit 1
  fi

  android_admission_write_emulator_owner "$owner_file"
  "$EMULATOR_BIN" \
    -avd "$AVD_NAME" \
    -port "$EMULATOR_PORT" \
    -memory "$EMULATOR_RAM_MB" \
    -lowram \
    -no-window \
    -no-audio \
    -no-boot-anim \
    -gpu "$GPU_MODE" \
    -verbose \
    -no-snapshot \
    -no-snapshot-save \
    -wipe-data \
    >"$cycle_dir/emulator.log" 2>&1 &
  emulator_pid="$!"
  EMULATOR_PID="$emulator_pid"
  export EMULATOR_PID
  android_admission_write_emulator_owner "$owner_file" "$emulator_pid"

  cycle_phase="renderer-admission"
  android_emulator_renderer_wait \
    "$cycle_dir/emulator.log" 30 ensure_cycle_process_alive || {
    echo "Android emulator renderer observation failed: $ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" >&2
    exit 1
  }
  renderer_pair="$ANDROID_EMULATOR_RENDERER_LAST_MODE"
  cycle_phase="boot-admission"
  wait_for_cycle_boot
  printf '%s\n' "$ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE" \
    >"$cycle_dir/boot-readiness.txt"
  verify_data_capacity
  system_pid_before="$(system_server_pid)"

  cycle_phase="install"
  ANDROID_INSTALL_RETRY_LOG_PREFIX="android-emulator-admission[$VARIANT_ID cycle=$cycle]"
  export ANDROID_INSTALL_RETRY_LOG_PREFIX
  install_apk

  cycle_phase="post-install-admission"
  android_emulator_wait_for_guest_services_stable \
    "$EMULATOR_SERIAL" \
    "$((SECONDS + POST_INSTALL_TIMEOUT_SECS))" \
    ensure_cycle_process_alive
  system_pid_after="$(system_server_pid)"
  [[ "$system_pid_before" == "$system_pid_after" ]] || {
    echo "system_server PID changed: before=$system_pid_before after=$system_pid_after" >&2
    exit 1
  }
  cycle_phase="complete"
)

main() {
  local cycle cycle_status passed=0
  CURRENT_PHASE="input-validation"
  validate_inputs
  require_command jq
  require_command timeout
  require_command sha256sum
  mkdir -p "$RESULT_DIR" "$CYCLE_RESULT_DIR" "$AVD_HOME"
  APK_SHA256="$(sha256sum "$APK_PATH" | awk '{print $1}')"

  if [[ "$REQUIRED" != "1" ]]; then
    log "diagnostic not executed; set DEVE_ANDROID_ADMISSION_DIAGNOSTIC_REQUIRED=1"
    android_admission_write_summary_result false false "diagnostic execution was not required"
    RESULT_WRITTEN=1
    return 0
  fi

  CURRENT_PHASE="sdk-install"
  install_sdk_packages
  ADB_BIN="$(android_tool_path adb)" || fail "adb is unavailable after SDK installation"
  CURRENT_PHASE="emulator-resolution"
  resolve_emulator
  CURRENT_PHASE="avd-creation"
  ensure_avd

  CURRENT_PHASE="cold-boot-cycles"
  for ((cycle = 1; cycle <= REQUESTED_CYCLES; cycle += 1)); do
    log "starting cold-boot cycle $cycle/$REQUESTED_CYCLES"
    mkdir -p "$RESULT_DIR/cycle-$cycle"
    # A Bash function invoked directly by `if` inherits an ignored errexit
    # context, even when the function enables `set -e` itself. Invoke the
    # cycle as a simple command while the caller temporarily tolerates its
    # status so every failing adb/admission command remains fail-closed.
    set +e
    run_cycle "$cycle" >"$RESULT_DIR/cycle-$cycle/cycle.log" 2>&1
    cycle_status=$?
    set -e
    if (( cycle_status == 0 )); then
      passed=$((passed + 1))
    fi
    android_admission_bound_cycle_logs "$RESULT_DIR/cycle-$cycle"
    head -c "$ANDROID_ADMISSION_LOG_FILE_BUDGET_BYTES" \
      "$RESULT_DIR/cycle-$cycle/cycle.log"
  done
  android_admission_verify_variant_log_budget "$RESULT_DIR" \
    || fail "variant diagnostic output exceeded ${ANDROID_ADMISSION_VARIANT_LOG_BUDGET_BYTES} bytes"

  if (( passed == REQUESTED_CYCLES )); then
    android_admission_write_summary_result true true ""
    RESULT_WRITTEN=1
    log "stable across $passed/$REQUESTED_CYCLES cold boots"
    return 0
  fi
  android_admission_write_summary_result true false ""
  RESULT_WRITTEN=1
  log "unstable: passed $passed/$REQUESTED_CYCLES cold boots"
  return 1
}

trap finish EXIT
main
