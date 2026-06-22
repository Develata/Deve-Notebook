#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
source "$ROOT_DIR/scripts/lib/android-tools.sh"
SIGNING_REQUIRED="${DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED:-0}"
DEVICE_REQUIRED="${DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED:-0}"
SERIAL="${DEVE_MOBILE_ANDROID_SERIAL:-}"
ARTIFACT_KIND="${DEVE_MOBILE_ANDROID_RELEASE_ARTIFACT_KIND:-aab}"

# This gate validates Android signed-release and physical-device prerequisite
# shape only. It must not sign, install, upload, or open native authority paths.

fail() {
  echo "mobile-android-release-preflight-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

missing_signing=()
missing_device=()

diagnose_env() {
  local name="$1"
  [[ -n "${!name:-}" ]] || missing_signing+=("env $name")
}

diagnose_keystore() {
  local path="${DEVE_ANDROID_KEYSTORE_PATH:-}"
  local base64_value="${DEVE_ANDROID_KEYSTORE_BASE64:-}"

  if [[ -n "$path" ]]; then
    [[ -f "$path" || -f "$ROOT_DIR/$path" ]] && return 0
    missing_signing+=("keystore path from DEVE_ANDROID_KEYSTORE_PATH")
    return 0
  fi
  [[ -n "$base64_value" ]] && return 0
  missing_signing+=("keystore via DEVE_ANDROID_KEYSTORE_PATH or DEVE_ANDROID_KEYSTORE_BASE64")
}

diagnose_command_name() {
  local label="$1"
  shift
  local command_name
  for command_name in "$@"; do
    command -v "$command_name" >/dev/null 2>&1 && return 0
  done
  missing_device+=("$label")
}

adb_lines() {
  android_run_tool adb devices 2>/dev/null | tail -n +2 | sed '/^[[:space:]]*$/d' || true
}

physical_device_present() {
  local lines line serial state
  lines="$(adb_lines)"
  [[ -n "$lines" ]] || return 1

  while IFS= read -r line; do
    serial="$(awk '{print $1}' <<<"$line")"
    state="$(awk '{print $2}' <<<"$line")"
    [[ "$state" == "device" ]] || continue
    [[ "$serial" == emulator-* ]] && continue
    if [[ -n "$SERIAL" && "$serial" != "$SERIAL" ]]; then
      continue
    fi
    return 0
  done <<<"$lines"
  return 1
}

diagnose_physical_device() {
  android_tool_path adb >/dev/null 2>&1 || {
    missing_device+=("adb")
    return 0
  }
  physical_device_present && return 0
  if [[ -n "$SERIAL" ]]; then
    missing_device+=("physical Android device with serial $SERIAL")
  else
    missing_device+=("physical Android device (non-emulator adb target)")
  fi
}

run_deve_baseline "$ROOT_DIR" "mobile-android-release-preflight" "mobile-android-release-preflight-check"
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

DEVE_MOBILE_PACKAGE_TARGETS=android \
  DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=0 \
  run "$ROOT_DIR/scripts/check-mobile-platform-package-preflight.sh"

diagnose_keystore
diagnose_env "DEVE_ANDROID_KEY_ALIAS"
diagnose_env "DEVE_ANDROID_KEYSTORE_PASSWORD"
diagnose_env "DEVE_ANDROID_KEY_PASSWORD"
android_prepare_java_home >/dev/null 2>&1 || true
diagnose_command_name "keytool" keytool
diagnose_physical_device

echo "mobile-android-release-preflight-check: artifact_kind=$ARTIFACT_KIND"
echo "mobile-android-release-preflight-check: serial=${SERIAL:-<any physical adb target>}"

if ((${#missing_signing[@]} > 0)); then
  for item in "${missing_signing[@]}"; do
    echo "mobile-android-release-preflight-check: missing signing $item" >&2
  done
  if [[ "$SIGNING_REQUIRED" == "1" ]]; then
    fail "Android signed-release prerequisites are incomplete"
  fi
fi

if ((${#missing_device[@]} > 0)); then
  for item in "${missing_device[@]}"; do
    echo "mobile-android-release-preflight-check: missing physical-device $item" >&2
  done
  if [[ "$DEVICE_REQUIRED" == "1" ]]; then
    fail "Android physical-device prerequisites are incomplete"
  fi
fi

if ((${#missing_signing[@]} > 0 || ${#missing_device[@]} > 0)); then
  echo "mobile-android-release-preflight-check: signed release and physical-device smoke not executed"
  echo "mobile-android-release-preflight-check: set DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED=1 to require signing prerequisites"
  echo "mobile-android-release-preflight-check: set DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED=1 to require a physical adb target"
  echo "mobile-android-release-preflight-check: ok"
  exit 0
fi

echo "mobile-android-release-preflight-check: prerequisites present; signing/device smoke remains a separate explicit release step"
echo "mobile-android-release-preflight-check: ok"
