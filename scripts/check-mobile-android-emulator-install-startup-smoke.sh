#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-owner.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-capacity.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-boot-readiness.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-pin.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-renderer.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-feature-policy.sh"
REQUIRED="${DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED:-0}"
API_LEVEL="${DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL:-37.0}"
SYSTEM_TARGET="${DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET:-google_apis}"
ARCH="${DEVE_MOBILE_ANDROID_EMULATOR_ARCH:-x86_64}"
AVD_NAME="${DEVE_MOBILE_ANDROID_EMULATOR_AVD_NAME:-deve-mobile-smoke-api${API_LEVEL}-${SYSTEM_TARGET}-${ARCH}}"
DEVICE_PROFILE="${DEVE_MOBILE_ANDROID_EMULATOR_DEVICE:-pixel_2}"
BOOT_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_EMULATOR_BOOT_TIMEOUT_SECS:-900}"
ADB_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS:-120}"
LIFECYCLE_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS:-600}"
PACKAGE_TARGET="${DEVE_MOBILE_ANDROID_PACKAGE_TARGET:-x86_64}"
EMULATOR_PORT="${DEVE_MOBILE_ANDROID_EMULATOR_PORT:-5584}"
EMULATOR_SERIAL="emulator-$EMULATOR_PORT"
EMULATOR_RAM_MB="${DEVE_MOBILE_ANDROID_EMULATOR_RAM_MB:-4096}"
EMULATOR_PARTITION_MB="${DEVE_MOBILE_ANDROID_EMULATOR_PARTITION_MB:-4096}"
LOG_DIR="${DEVE_MOBILE_ANDROID_EMULATOR_LOG_DIR:-$ROOT_DIR/target/mobile-android-emulator-smoke}"
OWNER_FILE="$(android_emulator_owner_file "$LOG_DIR")" || exit 1
AVD_HOME="${DEVE_MOBILE_ANDROID_AVD_HOME:-$ROOT_DIR/target/mobile-android-avd}"
JOURNEY="${DEVE_MOBILE_ANDROID_EMULATOR_JOURNEY:-local}"
DIAGNOSTICS_PRINTED=0

run_deve_baseline "$ROOT_DIR" "mobile-android-emulator-install-startup-smoke" "mobile-android-emulator-install-startup-smoke-check"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
source "$ROOT_DIR/scripts/lib/android-emulator-diagnostics.sh"

# This gate owns only target-host emulator orchestration. It delegates package
# build and install/startup checks to the narrower Android shell gates.
fail() {
  echo "mobile-android-emulator-install-startup-smoke-check: $*" >&2
  print_emulator_diagnostics >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

require_command() {
  local command_name="$1"

  command -v "$command_name" >/dev/null 2>&1 || fail "$command_name is required for Android emulator smoke"
}

require_android_tool() {
  local command_name="$1"

  android_tool_path "$command_name" >/dev/null 2>&1 || fail "$command_name is required for Android emulator smoke"
}

sdkmanager_cmd() {
  android_prepare_java_home || fail "Java 17+ or Android Studio JBR is required for sdkmanager"
  android_run_tool sdkmanager "$@"
}

avdmanager_cmd() {
  android_prepare_java_home || fail "Java 17+ or Android Studio JBR is required for avdmanager"
  android_run_tool avdmanager "$@"
}

emulator_cmd() {
  [[ -n "${PINNED_EMULATOR_BIN:-}" ]] \
    || fail "pinned Android emulator was not resolved before emulator_cmd"
  "$PINNED_EMULATOR_BIN" "$@"
}

adb_cmd() {
  android_run_tool adb "$@"
}

cleanup() {
  local cleanup_status=0
  DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE="$OWNER_FILE" \
    bash "$ROOT_DIR/scripts/cleanup-mobile-android-emulator.sh" \
    || cleanup_status=$?
  if (( cleanup_status != 0 )) \
      && [[ -n "${EMULATOR_PID:-}" ]] \
      && jobs -pr | grep -Fx -- "$EMULATOR_PID" >/dev/null 2>&1; then
    kill "$EMULATOR_PID" >/dev/null 2>&1 || true
  fi
  return "$cleanup_status"
}

cleanup_on_exit() {
  local status=$?
  local cleanup_status=0
  trap - EXIT
  if (( status != 0 && DIAGNOSTICS_PRINTED == 0 )); then
    print_emulator_diagnostics >&2
  fi
  cleanup || cleanup_status=$?
  if (( status != 0 )); then
    exit "$status"
  fi
  exit "$cleanup_status"
}

write_emulator_owner() {
  local pid="${1:-}"
  local launch_state="reserved"
  local temporary="$OWNER_FILE.tmp.$$"
  [[ -z "$pid" ]] || launch_state="launched"
  mkdir -p "$(dirname "$OWNER_FILE")"
  printf 'launch_state=%s\nemulator_pid=%s\nemulator_serial=%s\navd_name=%s\n' \
    "$launch_state" "$pid" "$EMULATOR_SERIAL" "$AVD_NAME" >"$temporary"
  mv -f -- "$temporary" "$OWNER_FILE"
}

install_sdk_packages() {
  local sdk
  local system_image="system-images;android-$API_LEVEL;$SYSTEM_TARGET;$ARCH"
  local packages=(
    "platform-tools"
    "emulator"
    "platforms;android-$API_LEVEL"
    "$system_image"
  )
  local missing=()
  local package
  local attempt
  local installed=0

  sdk="$(android_sdk_root)" || fail "Android SDK root is unavailable"
  for package in "${packages[@]}"; do
    if android_sdk_package_complete "$sdk" "$package"; then
      echo "mobile-android-emulator-install-startup-smoke-check: reuse local SDK package $package"
    else
      missing+=("$package")
    fi
  done
  (( ${#missing[@]} > 0 )) || return 0

  yes | sdkmanager_cmd --licenses >/dev/null || true
  for attempt in 1 2 3; do
    echo "+ sdkmanager_cmd ${missing[*]} (attempt $attempt/3)"
    if sdkmanager_cmd "${missing[@]}"; then
      installed=1
      break
    fi
    (( attempt < 3 )) && sleep 2
  done
  (( installed == 1 )) \
    || fail "Android SDK package installation failed after 3 attempts: ${missing[*]}"

  for package in "${missing[@]}"; do
    android_sdk_package_complete "$sdk" "$package" \
      || fail "Android SDK package remained incomplete after sdkmanager: $package"
  done
}

android_sdk_package_complete() {
  local sdk="$1"
  local package="$2"
  local platform
  local api
  local target
  local arch

  case "$package" in
    platform-tools)
      [[ -f "$sdk/platform-tools/adb" || -f "$sdk/platform-tools/adb.exe" ]]
      ;;
    emulator)
      [[ -f "$sdk/emulator/emulator" || -f "$sdk/emulator/emulator.exe" ]]
      ;;
    platforms\;*)
      platform="${package#platforms;}"
      [[ -f "$sdk/platforms/$platform/source.properties" \
        && -f "$sdk/platforms/$platform/android.jar" ]]
      ;;
    system-images\;*)
      IFS=';' read -r _ api target arch <<<"$package"
      [[ -n "$api" && -n "$target" && -n "$arch" \
        && -f "$sdk/system-images/$api/$target/$arch/source.properties" \
        && -f "$sdk/system-images/$api/$target/$arch/system.img" \
        && -f "$sdk/system-images/$api/$target/$arch/ramdisk.img" ]]
      ;;
    *)
      return 1
      ;;
  esac
}

ensure_avd() {
  local system_image="system-images;android-$API_LEVEL;$SYSTEM_TARGET;$ARCH"

  if emulator_cmd -list-avds | grep -Fx -- "$AVD_NAME" >/dev/null; then
    ensure_avd_data_partition
    return 0
  fi

  if ! printf 'no\n' | avdmanager_cmd create avd \
      --force \
      --name "$AVD_NAME" \
      --package "$system_image" \
      --device "$DEVICE_PROFILE" >"$LOG_DIR/avdmanager.log" 2>&1; then
    fail "Android AVD creation failed: $AVD_NAME"
  fi

  emulator_cmd -list-avds | grep -Fx -- "$AVD_NAME" >/dev/null \
    || fail "Android AVD was not visible to emulator after creation: $AVD_NAME"
  ensure_avd_data_partition
}

# The emulator -partition-size flag is rejected (>2047 MB), observed on the
# retired android-36.1 image, so the bounded /data request is written into the owned
# AVD's config.ini instead; verify_emulator_data_capacity stays the proof.
ensure_avd_data_partition() {
  local config_file="$AVD_HOME/$AVD_NAME.avd/config.ini"
  local temporary="$config_file.tmp.$$"

  [[ -f "$config_file" ]] \
    || fail "Android AVD config is missing for data partition sizing: $config_file"
  grep -v '^disk\.dataPartition\.size=' "$config_file" >"$temporary" || true
  printf 'disk.dataPartition.size=%sM\n' "$EMULATOR_PARTITION_MB" >>"$temporary"
  mv -f -- "$temporary" "$config_file"
}

wait_for_boot() {
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECS))
  local boot_properties_ready=0

  android_emulator_wait_for_device_state \
      "$EMULATOR_SERIAL" "$deadline" ensure_emulator_process_alive \
    || fail "Android emulator did not reach adb device state within ${BOOT_TIMEOUT_SECS}s: $ANDROID_EMULATOR_BOOT_DEVICE_STATE_LAST_EVIDENCE"

  android_emulator_boot_avd_identity_matches \
      "$EMULATOR_SERIAL" "$AVD_NAME" "$deadline" \
    || fail "Android emulator AVD identity rejected within boot deadline: $ANDROID_EMULATOR_BOOT_AVD_IDENTITY_LAST_EVIDENCE"

  while (( SECONDS < deadline )); do
    ensure_emulator_process_alive
    if android_emulator_boot_properties_ready "$EMULATOR_SERIAL" "$deadline"; then
      boot_properties_ready=1
      break
    fi
    android_emulator_boot_poll_sleep "$deadline" 5 || break
  done

  (( boot_properties_ready == 1 )) \
    || fail "Android emulator did not satisfy boot-property readiness within ${BOOT_TIMEOUT_SECS}s: $ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE"

  android_emulator_wait_for_guest_services_stable \
      "$EMULATOR_SERIAL" "$deadline" ensure_emulator_process_alive \
    || fail "Android emulator guest services did not remain stable within ${BOOT_TIMEOUT_SECS}s: $ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE"
  echo "mobile-android-emulator-install-startup-smoke-check: boot readiness: $ANDROID_EMULATOR_BOOT_READINESS_LAST_EVIDENCE"
}

verify_sdk_package_reuse_contract() {
  local fixture
  fixture="$(mktemp -d)"

  mkdir -p \
    "$fixture/platform-tools" \
    "$fixture/emulator" \
    "$fixture/platforms/android-36.1" \
    "$fixture/system-images/android-36.1/google_apis/x86_64"
  touch \
    "$fixture/platform-tools/adb.exe" \
    "$fixture/emulator/emulator.exe" \
    "$fixture/platforms/android-36.1/source.properties" \
    "$fixture/platforms/android-36.1/android.jar" \
    "$fixture/system-images/android-36.1/google_apis/x86_64/source.properties" \
    "$fixture/system-images/android-36.1/google_apis/x86_64/system.img" \
    "$fixture/system-images/android-36.1/google_apis/x86_64/ramdisk.img"

  android_sdk_package_complete "$fixture" "platform-tools" \
    || fail "complete local platform-tools fixture must be reusable"
  android_sdk_package_complete "$fixture" "emulator" \
    || fail "complete local emulator fixture must be reusable"
  android_sdk_package_complete "$fixture" "platforms;android-36.1" \
    || fail "complete local platform fixture must be reusable"
  android_sdk_package_complete \
    "$fixture" \
    "system-images;android-36.1;google_apis;x86_64" \
    || fail "complete local system-image fixture must be reusable"
  rm "$fixture/system-images/android-36.1/google_apis/x86_64/system.img"
  if android_sdk_package_complete \
    "$fixture" \
    "system-images;android-36.1;google_apis;x86_64"; then
    fail "incomplete local system-image fixture must require sdkmanager repair"
  fi
  rm -rf "$fixture"
}

validate_emulator_port() {
  [[ "$EMULATOR_PORT" =~ ^[0-9]+$ ]] \
    || fail "DEVE_MOBILE_ANDROID_EMULATOR_PORT must be an even integer"
  (( EMULATOR_PORT >= 5554 && EMULATOR_PORT <= 5682 && EMULATOR_PORT % 2 == 0 )) \
    || fail "DEVE_MOBILE_ANDROID_EMULATOR_PORT must be an even port in 5554..5682"
}

validate_emulator_ram() {
  [[ "$EMULATOR_RAM_MB" =~ ^[0-9]+$ ]] \
    || fail "DEVE_MOBILE_ANDROID_EMULATOR_RAM_MB must be an integer"
  (( EMULATOR_RAM_MB >= 1536 && EMULATOR_RAM_MB <= 4096 )) \
    || fail "DEVE_MOBILE_ANDROID_EMULATOR_RAM_MB must be in 1536..4096"
}

validate_emulator_partition() {
  [[ "$EMULATOR_PARTITION_MB" =~ ^[0-9]+$ ]] \
    || fail "DEVE_MOBILE_ANDROID_EMULATOR_PARTITION_MB must be an integer"
  (( EMULATOR_PARTITION_MB >= 2048 && EMULATOR_PARTITION_MB <= 8192 )) \
    || fail "DEVE_MOBILE_ANDROID_EMULATOR_PARTITION_MB must be in 2048..8192"
}

verify_emulator_data_capacity() {
  local output parsed total_kib available_kib minimum_total_kib=0
  # Quoted remote command: Git Bash (MSYS) path-converts a bare /data arg
  # into a Windows host path on Windows target hosts.
  output="$(adb_cmd -s "$EMULATOR_SERIAL" shell "df -k /data" 2>&1)" \
    || fail "Android emulator /data capacity probe failed"
  printf '%s\n' "$output"
  parsed="$(printf '%s\n' "$output" | parse_android_emulator_data_capacity)" \
    || fail "Android emulator /data capacity probe returned an invalid row"
  read -r total_kib available_kib <<<"$parsed" \
    || fail "Android emulator /data capacity probe returned an invalid row"
  [[ "$total_kib" =~ ^[0-9]+$ && "$available_kib" =~ ^[0-9]+$ ]] \
    || fail "Android emulator /data capacity probe returned an invalid row"
  minimum_total_kib=$(( EMULATOR_PARTITION_MB * 1024 * 3 / 4 ))
  (( total_kib >= minimum_total_kib )) \
    || fail "Android emulator /data total capacity is below the requested partition floor"
  (( available_kib >= 1048576 )) \
    || fail "Android emulator /data has less than 1024 MiB available"
  echo "mobile-android-emulator-install-startup-smoke-check: data_total_kib=$total_kib data_available_kib=$available_kib"
}

validate_lifecycle_timeout() {
  [[ "$LIFECYCLE_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] \
    || fail "DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS must be a positive integer"
}

stop_android_gradle_daemon() {
  local gradle_root="$ROOT_DIR/apps/mobile/gen/android"
  if [[ -x "$gradle_root/gradlew" ]]; then
    (cd "$gradle_root" && ./gradlew --stop) >/dev/null 2>&1 || true
  fi
}

ensure_emulator_serial_available() {
  if adb_cmd devices | awk -v serial="$EMULATOR_SERIAL" '$1 == serial { found = 1 } END { exit !found }'; then
    fail "dedicated Android emulator serial is already in use: $EMULATOR_SERIAL"
  fi
}

ensure_emulator_process_alive() {
  if [[ -n "${EMULATOR_PID:-}" ]] && ! kill -0 "$EMULATOR_PID" >/dev/null 2>&1; then
    wait "$EMULATOR_PID" >/dev/null 2>&1 || true
    fail "owned Android emulator process exited unexpectedly"
  fi
}
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"
run node --test "$ROOT_DIR/scripts/webcrypto-capability.test.mjs"
run node --test "$ROOT_DIR/scripts/android-target-capability.test.mjs"
run node --test "$ROOT_DIR/apps/web/js/editor_lifecycle.test.mjs"
run node --test "$ROOT_DIR/scripts/android-webview-cdp.test.mjs" "$ROOT_DIR/scripts/android-app-process-observation.test.mjs" "$ROOT_DIR/scripts/android-logcat-observation.test.mjs" "$ROOT_DIR/scripts/smoke-mobile-android-remote-browser.test.mjs"
run node --test "$ROOT_DIR/scripts/mobile-webview-interaction.test.mjs"
run node --test "$ROOT_DIR/scripts/mobile-android-emulator-journey.test.mjs"
run node --test "$ROOT_DIR/scripts/websocket-delivery-gate.test.mjs"
run node --check "$ROOT_DIR/scripts/lib/android-webview-cdp.mjs"
run node --check "$ROOT_DIR/scripts/lib/android-webview-cdp-client.mjs"
run node --check "$ROOT_DIR/scripts/lib/mobile-source-control-interaction.mjs"
run bash "$ROOT_DIR/scripts/android-emulator-capacity.test.sh"
run bash "$ROOT_DIR/scripts/android-guest-service-readiness.test.sh"
run bash "$ROOT_DIR/scripts/android-emulator-boot-readiness.test.sh"
run bash "$ROOT_DIR/scripts/android-install-retry.test.sh"
run bash "$ROOT_DIR/scripts/android-startup-diagnostics.test.sh"
run bash "$ROOT_DIR/scripts/android-app-process-readiness.test.sh"
run bash "$ROOT_DIR/scripts/android-emulator-pin.test.sh"
run bash "$ROOT_DIR/scripts/android-emulator-renderer.test.sh"
run bash "$ROOT_DIR/scripts/android-emulator-feature-policy.test.sh"
verify_sdk_package_reuse_contract
run bash "$ROOT_DIR/scripts/android-emulator-cleanup.test.sh"
validate_emulator_port
validate_emulator_ram
validate_emulator_partition
validate_lifecycle_timeout

case "$JOURNEY" in
  local | remote) ;;
  *) fail "DEVE_MOBILE_ANDROID_EMULATOR_JOURNEY must be local or remote" ;;
esac

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-android-emulator-install-startup-smoke-check: emulator smoke not executed; set DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 on an Android target host"
  echo "mobile-android-emulator-install-startup-smoke-check: api=$API_LEVEL target=$SYSTEM_TARGET arch=$ARCH avd=$AVD_NAME partition_mb=$EMULATOR_PARTITION_MB journey=$JOURNEY"
  echo "mobile-android-emulator-install-startup-smoke-check: ok"
  exit 0
fi

require_command timeout
require_android_tool sdkmanager
require_android_tool avdmanager

mkdir -p "$LOG_DIR" "$AVD_HOME"
export ANDROID_AVD_HOME="$AVD_HOME"

install_sdk_packages
PINNED_EMULATOR_BIN="$(android_resolve_pinned_emulator)" \
  || fail "pinned Android emulator $ANDROID_EMULATOR_PIN_VERSION (build $ANDROID_EMULATOR_PIN_BUILD_ID) could not be resolved"
echo "mobile-android-emulator-install-startup-smoke-check: pinned emulator: $PINNED_EMULATOR_BIN"
require_android_tool adb
ensure_avd

# Build the exact package before reserving several GiB for the emulator. This
# keeps the target-host gate viable on the project's low-memory Windows host.
(
  export DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1
  export DEVE_MOBILE_ANDROID_PACKAGE_DEBUG=1
  export DEVE_MOBILE_ANDROID_PACKAGE_TARGET="$PACKAGE_TARGET"
  run "$ROOT_DIR/scripts/check-mobile-android-shell-package-build.sh"
)
stop_android_gradle_daemon

ensure_emulator_serial_available

if [[ -f "$OWNER_FILE" ]]; then
  cleanup || fail "stale owned Android emulator could not be cleaned"
fi
trap cleanup_on_exit EXIT
write_emulator_owner

FORMAL_FEATURE_POLICY="direct-memory-shared-slots"
android_emulator_feature_policy_configure "$FORMAL_FEATURE_POLICY" \
  || fail "$ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE"

emulator_cmd \
  -avd "$AVD_NAME" \
  -port "$EMULATOR_PORT" \
  -memory "$EMULATOR_RAM_MB" \
  -lowram \
  -no-window \
  -no-audio \
  -no-boot-anim \
  -gpu swangle \
  "${ANDROID_EMULATOR_FEATURE_ARGS[@]}" \
  -verbose \
  -no-snapshot \
  -no-snapshot-save \
  -wipe-data \
  >"$LOG_DIR/emulator.log" 2>&1 &
EMULATOR_PID="$!"
write_emulator_owner "$EMULATOR_PID"

android_emulator_feature_policy_wait \
    "$LOG_DIR/emulator.log" "$FORMAL_FEATURE_POLICY" 30 ensure_emulator_process_alive \
  || fail "Android emulator feature proof failed: $ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE"
echo "mobile-android-emulator-install-startup-smoke-check: gfxstream features: $ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE"

wait_for_boot
verify_emulator_data_capacity
echo "mobile-android-emulator-install-startup-smoke-check: emulator renderer selection:"
android_emulator_renderer_verify "$LOG_DIR/emulator.log" \
  || fail "Android emulator renderer proof failed: $ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE"
echo "mobile-android-emulator-install-startup-smoke-check: $ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE"

adb_cmd -s "$EMULATOR_SERIAL" shell input keyevent 82 >/dev/null 2>&1 || true

(
  export DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED=1
  export DEVE_MOBILE_ANDROID_SERIAL="$EMULATOR_SERIAL"
  export DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS="$ADB_TIMEOUT_SECS"
  export DEVE_MOBILE_ANDROID_INSTALL_SMOKE_UNINSTALL=0
  run "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
)

if [[ "$JOURNEY" == "local" ]]; then
  (
    export DEVE_MOBILE_ANDROID_LIFECYCLE_SMOKE_REQUIRED=1
    export DEVE_MOBILE_ANDROID_SERIAL="$EMULATOR_SERIAL"
    export DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS="$LIFECYCLE_TIMEOUT_SECS"
    run bash "$ROOT_DIR/scripts/smoke-mobile-android-lifecycle.sh"
  )
else
  (
    export DEVE_MOBILE_ANDROID_REMOTE_SMOKE_REQUIRED=1
    export DEVE_MOBILE_ANDROID_SERIAL="$EMULATOR_SERIAL"
    export DEVE_MOBILE_ANDROID_REMOTE_TIMEOUT_SECS="$LIFECYCLE_TIMEOUT_SECS"
    run bash "$ROOT_DIR/scripts/smoke-mobile-android-remote-browser.sh"
  )
fi

ensure_emulator_process_alive
android_emulator_feature_policy_observe "$LOG_DIR/emulator.log" "$FORMAL_FEATURE_POLICY" \
  || fail "Android emulator final feature proof failed: $ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE"
ensure_emulator_process_alive

echo "mobile-android-emulator-install-startup-smoke-check: serial=$EMULATOR_SERIAL partition_mb=$EMULATOR_PARTITION_MB journey=$JOURNEY log=${LOG_DIR#"$ROOT_DIR"/}/emulator.log"
echo "mobile-android-emulator-install-startup-smoke-check: ok"
