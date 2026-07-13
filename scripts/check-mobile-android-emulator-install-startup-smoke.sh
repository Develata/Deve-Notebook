#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
REQUIRED="${DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED:-0}"
API_LEVEL="${DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL:-37.1}"
SYSTEM_TARGET="${DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET:-google_apis_ps16k}"
ARCH="${DEVE_MOBILE_ANDROID_EMULATOR_ARCH:-x86_64}"
AVD_NAME="${DEVE_MOBILE_ANDROID_EMULATOR_AVD_NAME:-deve-mobile-smoke-api${API_LEVEL}-${SYSTEM_TARGET}-${ARCH}}"
DEVICE_PROFILE="${DEVE_MOBILE_ANDROID_EMULATOR_DEVICE:-pixel_2}"
BOOT_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_EMULATOR_BOOT_TIMEOUT_SECS:-900}"
ADB_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS:-120}"
LIFECYCLE_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS:-600}"
PACKAGE_TARGET="${DEVE_MOBILE_ANDROID_PACKAGE_TARGET:-x86_64}"
EMULATOR_PORT="${DEVE_MOBILE_ANDROID_EMULATOR_PORT:-5584}"
EMULATOR_SERIAL="emulator-$EMULATOR_PORT"
EMULATOR_RAM_MB="${DEVE_MOBILE_ANDROID_EMULATOR_RAM_MB:-3072}"
LOG_DIR="${DEVE_MOBILE_ANDROID_EMULATOR_LOG_DIR:-$ROOT_DIR/target/mobile-android-emulator-smoke}"
AVD_HOME="${DEVE_MOBILE_ANDROID_AVD_HOME:-$ROOT_DIR/target/mobile-android-avd}"

run_deve_baseline "$ROOT_DIR" "mobile-android-emulator-install-startup-smoke" "mobile-android-emulator-install-startup-smoke-check"
source "$ROOT_DIR/scripts/lib/android-tools.sh"

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
  android_run_tool emulator "$@"
}

adb_cmd() {
  android_run_tool adb "$@"
}

cleanup() {
  if [[ -n "${EMULATOR_SERIAL:-}" ]]; then
    adb_cmd -s "$EMULATOR_SERIAL" emu kill >/dev/null 2>&1 || true
  fi
  if [[ -n "${EMULATOR_PID:-}" ]]; then
    kill "$EMULATOR_PID" >/dev/null 2>&1 || true
  fi
}

print_emulator_diagnostics() {
  if command -v adb >/dev/null 2>&1; then
    echo "mobile-android-emulator-install-startup-smoke-check: adb devices:"
    adb devices 2>&1 || true
  elif android_tool_path adb >/dev/null 2>&1; then
    echo "mobile-android-emulator-install-startup-smoke-check: adb devices:"
    android_run_tool adb devices 2>&1 || true
  fi
  if command -v emulator >/dev/null 2>&1 || android_tool_path emulator >/dev/null 2>&1; then
    echo "mobile-android-emulator-install-startup-smoke-check: emulator AVD list:"
    android_run_tool emulator -list-avds 2>&1 || true
  fi
  if [[ -f "$LOG_DIR/avdmanager.log" ]]; then
    echo "mobile-android-emulator-install-startup-smoke-check: avdmanager.log tail:"
    tail -n 120 "$LOG_DIR/avdmanager.log" || true
  fi
  if [[ -f "$LOG_DIR/emulator.log" ]]; then
    echo "mobile-android-emulator-install-startup-smoke-check: emulator.log tail:"
    tail -n 120 "$LOG_DIR/emulator.log" || true
  fi
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
}

wait_for_boot() {
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECS))
  local sys_boot_completed=""
  local dev_boot_complete=""
  local remaining=0

  while (( SECONDS < deadline )); do
    ensure_emulator_process_alive
    if adb_cmd devices | awk -v serial="$EMULATOR_SERIAL" '$1 == serial { found = 1 } END { exit !found }'; then
      break
    fi
    sleep 2
  done

  [[ -n "${EMULATOR_SERIAL:-}" ]] \
    || fail "Android emulator serial did not appear within ${BOOT_TIMEOUT_SECS}s"

  remaining=$((deadline - SECONDS))
  (( remaining > 0 )) || fail "Android emulator serial appeared after boot deadline"
  timeout "$remaining" "$(android_tool_path adb)" -s "$EMULATOR_SERIAL" wait-for-device \
    || fail "Android emulator did not reach adb device state within ${BOOT_TIMEOUT_SECS}s"

  local observed_avd
  observed_avd="$(adb_cmd -s "$EMULATOR_SERIAL" emu avd name 2>/dev/null | tr -d '\r' | head -n 1 || true)"
  [[ "$observed_avd" == "$AVD_NAME" ]] \
    || fail "Android emulator serial $EMULATOR_SERIAL belongs to '$observed_avd', expected '$AVD_NAME'"

  while (( SECONDS < deadline )); do
    ensure_emulator_process_alive
    sys_boot_completed="$(adb_cmd -s "$EMULATOR_SERIAL" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)"
    dev_boot_complete="$(adb_cmd -s "$EMULATOR_SERIAL" shell getprop dev.bootcomplete 2>/dev/null | tr -d '\r' || true)"
    if android_boot_properties_complete "$sys_boot_completed" "$dev_boot_complete" \
        && adb_cmd -s "$EMULATOR_SERIAL" shell cmd package list packages >/dev/null 2>&1; then
      return 0
    fi
    sleep 5
  done

  fail "Android emulator did not finish booting within ${BOOT_TIMEOUT_SECS}s"
}

verify_boot_completion_contract() {
  android_boot_properties_complete "1" "" \
    || fail "sys.boot_completed=1 must satisfy the emulator boot property gate"
  android_boot_properties_complete "" "1" \
    || fail "dev.bootcomplete=1 must satisfy the emulator boot property gate"
  if android_boot_properties_complete "" ""; then
    fail "missing Android boot completion properties must fail closed"
  fi
}

verify_sdk_package_reuse_contract() {
  local fixture
  fixture="$(mktemp -d)"

  mkdir -p \
    "$fixture/platform-tools" \
    "$fixture/emulator" \
    "$fixture/platforms/android-37.1" \
    "$fixture/system-images/android-37.1/google_apis_ps16k/x86_64"
  touch \
    "$fixture/platform-tools/adb.exe" \
    "$fixture/emulator/emulator.exe" \
    "$fixture/platforms/android-37.1/source.properties" \
    "$fixture/platforms/android-37.1/android.jar" \
    "$fixture/system-images/android-37.1/google_apis_ps16k/x86_64/source.properties" \
    "$fixture/system-images/android-37.1/google_apis_ps16k/x86_64/system.img" \
    "$fixture/system-images/android-37.1/google_apis_ps16k/x86_64/ramdisk.img"

  android_sdk_package_complete "$fixture" "platform-tools" \
    || fail "complete local platform-tools fixture must be reusable"
  android_sdk_package_complete "$fixture" "emulator" \
    || fail "complete local emulator fixture must be reusable"
  android_sdk_package_complete "$fixture" "platforms;android-37.1" \
    || fail "complete local platform fixture must be reusable"
  android_sdk_package_complete \
    "$fixture" \
    "system-images;android-37.1;google_apis_ps16k;x86_64" \
    || fail "complete local system-image fixture must be reusable"
  rm "$fixture/system-images/android-37.1/google_apis_ps16k/x86_64/system.img"
  if android_sdk_package_complete \
    "$fixture" \
    "system-images;android-37.1;google_apis_ps16k;x86_64"; then
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
    fail "Android emulator process exited before boot completed"
  fi
}

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"
run node --test "$ROOT_DIR/scripts/webcrypto-capability.test.mjs"
run node --test "$ROOT_DIR/apps/web/js/editor_lifecycle.test.mjs"
run node --test "$ROOT_DIR/scripts/mobile-webview-interaction.test.mjs"
run node --test "$ROOT_DIR/scripts/websocket-delivery-gate.test.mjs"
run node --check "$ROOT_DIR/scripts/lib/android-webview-cdp.mjs"
run node --check "$ROOT_DIR/scripts/lib/mobile-source-control-interaction.mjs"
verify_boot_completion_contract
verify_sdk_package_reuse_contract
validate_emulator_port
validate_emulator_ram
validate_lifecycle_timeout

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-android-emulator-install-startup-smoke-check: emulator smoke not executed; set DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 on an Android target host"
  echo "mobile-android-emulator-install-startup-smoke-check: api=$API_LEVEL target=$SYSTEM_TARGET arch=$ARCH avd=$AVD_NAME"
  echo "mobile-android-emulator-install-startup-smoke-check: ok"
  exit 0
fi

require_command timeout
require_android_tool sdkmanager
require_android_tool avdmanager
require_android_tool emulator
require_android_tool adb

mkdir -p "$LOG_DIR" "$AVD_HOME"
export ANDROID_AVD_HOME="$AVD_HOME"

install_sdk_packages
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

trap cleanup EXIT

emulator_cmd \
  -avd "$AVD_NAME" \
  -port "$EMULATOR_PORT" \
  -memory "$EMULATOR_RAM_MB" \
  -lowram \
  -no-window \
  -no-audio \
  -no-boot-anim \
  -gpu swiftshader_indirect \
  -no-snapshot \
  -no-snapshot-save \
  -wipe-data \
  >"$LOG_DIR/emulator.log" 2>&1 &
EMULATOR_PID="$!"

wait_for_boot

adb_cmd -s "$EMULATOR_SERIAL" shell input keyevent 82 >/dev/null 2>&1 || true

(
  export DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED=1
  export DEVE_MOBILE_ANDROID_SERIAL="$EMULATOR_SERIAL"
  export DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS="$ADB_TIMEOUT_SECS"
  export DEVE_MOBILE_ANDROID_INSTALL_SMOKE_UNINSTALL=0
  run "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
)

(
  export DEVE_MOBILE_ANDROID_LIFECYCLE_SMOKE_REQUIRED=1
  export DEVE_MOBILE_ANDROID_SERIAL="$EMULATOR_SERIAL"
  export DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS="$LIFECYCLE_TIMEOUT_SECS"
  run "$ROOT_DIR/scripts/smoke-mobile-android-lifecycle.sh"
)

echo "mobile-android-emulator-install-startup-smoke-check: serial=$EMULATOR_SERIAL log=${LOG_DIR#"$ROOT_DIR"/}/emulator.log"
echo "mobile-android-emulator-install-startup-smoke-check: ok"
