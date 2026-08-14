#!/usr/bin/env bash

# Owns a reversible, exact Android test-IME selection. The caller supplies an
# adb function so normal work can use its global deadline while EXIT recovery
# retains a separate bounded transport path.

ANDROID_IME_TEST_ORIGINAL_SERVICE=""
ANDROID_IME_TEST_SELECTED_SERVICE=""
ANDROID_IME_TEST_RESTORE_REQUIRED=0

android_ime_test_read_default() {
  local adb_function="$1"
  "$adb_function" shell settings get secure default_input_method | tr -d '\r'
}

android_ime_test_begin() {
  local adb_function="$1"
  local service="$2"
  local current=""

  [[ "$service" =~ ^[A-Za-z0-9._]+/[A-Za-z0-9._$]+$ ]] || {
    echo "android-ime-test-session: invalid exact IME component" >&2
    return 1
  }
  ANDROID_IME_TEST_ORIGINAL_SERVICE="$(android_ime_test_read_default "$adb_function")" || return 1
  [[ -n "$ANDROID_IME_TEST_ORIGINAL_SERVICE" \
    && "$ANDROID_IME_TEST_ORIGINAL_SERVICE" != "null" ]] || {
    echo "android-ime-test-session: default input method is unavailable" >&2
    return 1
  }
  "$adb_function" shell ime list -s | tr -d '\r' | grep -Fx "$service" >/dev/null || {
    echo "android-ime-test-session: configured IME is not installed/enabled: $service" >&2
    return 1
  }

  ANDROID_IME_TEST_SELECTED_SERVICE="$service"
  if [[ "$ANDROID_IME_TEST_ORIGINAL_SERVICE" == "$service" ]]; then
    return 0
  fi

  # The device mutation may commit even when the host loses the response.
  # Enter restore-required before issuing it so every ambiguous exit recovers.
  ANDROID_IME_TEST_RESTORE_REQUIRED=1
  "$adb_function" shell ime set "$service" >/dev/null || return 1
  current="$(android_ime_test_read_default "$adb_function")" || return 1
  [[ "$current" == "$service" ]] || {
    echo "android-ime-test-session: selected IME verification mismatch" >&2
    return 1
  }
}

android_ime_test_restore() {
  local adb_function="$1"
  local attempt current="unavailable"
  [[ "$ANDROID_IME_TEST_RESTORE_REQUIRED" == "1" ]] || return 0
  [[ -n "$ANDROID_IME_TEST_ORIGINAL_SERVICE" ]] || {
    echo "android-ime-test-session: recovery blocker: original IME is unknown" >&2
    return 1
  }

  for attempt in 1 2; do
    "$adb_function" shell ime set "$ANDROID_IME_TEST_ORIGINAL_SERVICE" >/dev/null 2>&1 || true
    current="$(android_ime_test_read_default "$adb_function" 2>/dev/null)" || current="unavailable"
    if [[ "$current" == "$ANDROID_IME_TEST_ORIGINAL_SERVICE" ]]; then
      ANDROID_IME_TEST_RESTORE_REQUIRED=0
      echo "android-ime-test-session: original_ime_restored=true"
      return 0
    fi
  done

  echo "android-ime-test-session: recovery blocker: original IME restore verification failed; expected=$ANDROID_IME_TEST_ORIGINAL_SERVICE actual=$current" >&2
  return 1
}
