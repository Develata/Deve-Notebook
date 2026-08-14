#!/usr/bin/env bash

# Cohesive, read-only admission contracts for the targeted Android emulator
# host. The owning orchestrator supplies `fail` and the SDK package probe.

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

validate_lifecycle_timeout() {
  [[ "$LIFECYCLE_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] \
    || fail "DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS must be a positive integer"
}
