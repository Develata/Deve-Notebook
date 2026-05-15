#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED:-0}"
API_LEVEL="${DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL:-35}"
SYSTEM_TARGET="${DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET:-google_apis}"
ARCH="${DEVE_MOBILE_ANDROID_EMULATOR_ARCH:-x86_64}"
AVD_NAME="${DEVE_MOBILE_ANDROID_EMULATOR_AVD_NAME:-deve-mobile-smoke}"
DEVICE_PROFILE="${DEVE_MOBILE_ANDROID_EMULATOR_DEVICE:-pixel_6}"
BOOT_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_EMULATOR_BOOT_TIMEOUT_SECS:-600}"
ADB_TIMEOUT_SECS="${DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS:-120}"
PACKAGE_TARGET="${DEVE_MOBILE_ANDROID_PACKAGE_TARGET:-x86_64}"
LOG_DIR="${DEVE_MOBILE_ANDROID_EMULATOR_LOG_DIR:-$ROOT_DIR/target/mobile-android-emulator-smoke}"

# This gate owns only target-host emulator orchestration. It delegates package
# build and install/startup checks to the narrower Android shell gates.

fail() {
  echo "mobile-android-emulator-install-startup-smoke-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

assert_positive_integer() {
  local name="$1"
  local value="$2"

  [[ "$value" =~ ^[1-9][0-9]*$ ]] || fail "$name must be a positive integer"
}

require_command() {
  local command_name="$1"

  command -v "$command_name" >/dev/null 2>&1 || fail "$command_name is required for Android emulator smoke"
}

sdkmanager_cmd() {
  require_command sdkmanager
  sdkmanager "$@"
}

avdmanager_cmd() {
  require_command avdmanager
  avdmanager "$@"
}

emulator_cmd() {
  require_command emulator
  emulator "$@"
}

adb_cmd() {
  require_command adb
  adb "$@"
}

cleanup() {
  if [[ -n "${EMULATOR_SERIAL:-}" ]]; then
    adb -s "$EMULATOR_SERIAL" emu kill >/dev/null 2>&1 || true
  fi
  if [[ -n "${EMULATOR_PID:-}" ]]; then
    kill "$EMULATOR_PID" >/dev/null 2>&1 || true
  fi
}

install_sdk_packages() {
  local system_image="system-images;android-$API_LEVEL;$SYSTEM_TARGET;$ARCH"

  yes | sdkmanager_cmd --licenses >/dev/null || true
  run sdkmanager_cmd \
    "platform-tools" \
    "emulator" \
    "platforms;android-$API_LEVEL" \
    "$system_image"
}

ensure_avd() {
  local system_image="system-images;android-$API_LEVEL;$SYSTEM_TARGET;$ARCH"

  if emulator_cmd -list-avds | grep -Fx -- "$AVD_NAME" >/dev/null; then
    return 0
  fi

  printf 'no\n' | avdmanager_cmd create avd \
    --force \
    --name "$AVD_NAME" \
    --package "$system_image" \
    --device "$DEVICE_PROFILE" >/dev/null
}

wait_for_boot() {
  local deadline=$((SECONDS + BOOT_TIMEOUT_SECS))
  local booted=""

  while (( SECONDS < deadline )); do
    EMULATOR_SERIAL="$(first_emulator_serial)"
    if [[ -n "$EMULATOR_SERIAL" ]]; then
      break
    fi
    sleep 2
  done

  [[ -n "${EMULATOR_SERIAL:-}" ]] \
    || fail "Android emulator device did not appear within ${BOOT_TIMEOUT_SECS}s"

  while (( SECONDS < deadline )); do
    booted="$(adb_cmd -s "$EMULATOR_SERIAL" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)"
    if [[ "$booted" == "1" ]]; then
      return 0
    fi
    sleep 5
  done

  fail "Android emulator did not finish booting within ${BOOT_TIMEOUT_SECS}s"
}

first_emulator_serial() {
  adb_cmd devices | awk '$1 ~ /^emulator-/ && $2 == "device" { print $1; exit }'
}

assert_positive_integer "DEVE_MOBILE_ANDROID_EMULATOR_BOOT_TIMEOUT_SECS" "$BOOT_TIMEOUT_SECS"
assert_positive_integer "DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS" "$ADB_TIMEOUT_SECS"

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-android-emulator-install-startup-smoke-check: emulator smoke not executed; set DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 on an Android target host"
  echo "mobile-android-emulator-install-startup-smoke-check: api=$API_LEVEL target=$SYSTEM_TARGET arch=$ARCH avd=$AVD_NAME"
  echo "mobile-android-emulator-install-startup-smoke-check: ok"
  exit 0
fi

require_command timeout
require_command sdkmanager
require_command avdmanager
require_command emulator
require_command adb

mkdir -p "$LOG_DIR"
install_sdk_packages
ensure_avd

trap cleanup EXIT

emulator_cmd \
  -avd "$AVD_NAME" \
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
  export DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1
  export DEVE_MOBILE_ANDROID_PACKAGE_DEBUG=1
  export DEVE_MOBILE_ANDROID_PACKAGE_TARGET="$PACKAGE_TARGET"
  run "$ROOT_DIR/scripts/check-mobile-android-shell-package-build.sh"
)

(
  export DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED=1
  export DEVE_MOBILE_ANDROID_SERIAL="$EMULATOR_SERIAL"
  export DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS="$ADB_TIMEOUT_SECS"
  run "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
)

echo "mobile-android-emulator-install-startup-smoke-check: serial=$EMULATOR_SERIAL log=${LOG_DIR#"$ROOT_DIR"/}/emulator.log"
echo "mobile-android-emulator-install-startup-smoke-check: ok"
