#!/usr/bin/env bash

android_normalize_owner_path() {
  local value="$1"
  if command -v cygpath >/dev/null 2>&1 && [[ "$value" =~ ^[A-Za-z]:[\\/] ]]; then
    cygpath -u "$value"
  else
    printf '%s\n' "$value"
  fi
}

android_emulator_owner_file() {
  local fallback_state="$1"
  local state_raw="${DEVE_ACCEPTANCE_PRODUCER_STATE_DIR:-$fallback_state}"
  local state
  local expected
  local override
  [[ -n "$state_raw" ]] || {
    echo "Android emulator owner state directory is empty" >&2
    return 1
  }
  state="$(android_normalize_owner_path "$state_raw")" || return 1
  expected="${state%/}/android-emulator-owner.txt"
  if [[ -n "${DEVE_ACCEPTANCE_PRODUCER_STATE_DIR:-}" ]]; then
    if [[ -n "${DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE:-}" ]]; then
      override="$(android_normalize_owner_path "$DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE")" \
        || return 1
      [[ "$override" == "$expected" ]] || {
        echo "runner-owned Android emulator owner file may not escape its state directory" >&2
        return 1
      }
    fi
    printf '%s\n' "$expected"
    return 0
  fi
  android_normalize_owner_path "${DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE:-$expected}"
}
