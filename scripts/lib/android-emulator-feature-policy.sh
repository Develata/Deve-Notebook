#!/usr/bin/env bash
# Exact gfxstream memory-feature policy for Android emulator admission.
# Requested flags are not evidence: every cycle must parse the two effective
# feature states from the bounded emulator log and match its declared policy.

ANDROID_EMULATOR_FEATURE_POLICY_LOG_SCAN_BYTES=262144
ANDROID_EMULATOR_FEATURE_ARGS=()
ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR=""
ANDROID_EMULATOR_FEATURE_POLICY_LAST_PAIR=""
ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE=""

android_emulator_feature_policy_configure() {
  local policy="$1"
  ANDROID_EMULATOR_FEATURE_ARGS=()
  case "$policy" in
    default)
      ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR="0/0"
      ;;
    direct-memory)
      ANDROID_EMULATOR_FEATURE_ARGS=(-feature GLDirectMem)
      ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR="1/0"
      ;;
    direct-memory-shared-slots)
      ANDROID_EMULATOR_FEATURE_ARGS=(
        -feature GLDirectMem
        -feature HasSharedSlotsHostMemoryAllocator
      )
      ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR="1/1"
      ;;
    *)
      ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="unknown gfxstream feature policy: $policy"
      return 1
      ;;
  esac
}

android_emulator_feature_policy_observe() {
  local log_file="$1"
  local policy="$2"
  local scan direct_values shared_values direct_count shared_count pair expected
  ANDROID_EMULATOR_FEATURE_POLICY_LAST_PAIR=""
  ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE=""
  [[ -f "$log_file" ]] || {
    ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="emulator log is missing"
    return 1
  }
  scan="$(head -c "$ANDROID_EMULATOR_FEATURE_POLICY_LOG_SCAN_BYTES" "$log_file")"
  direct_values="$(printf '%s\n' "$scan" \
    | sed -n 's/.*gfxstreamFeature:GlDirectMem = \([01]\).*/\1/p' \
    | sort -u)"
  shared_values="$(printf '%s\n' "$scan" \
    | sed -n 's/.*gfxstreamFeature:HasSharedSlotsHostMemoryAllocator = \([01]\).*/\1/p' \
    | sort -u)"
  direct_count="$(printf '%s\n' "$direct_values" | sed '/^$/d' | wc -l | tr -d '[:space:]')"
  shared_count="$(printf '%s\n' "$shared_values" | sed '/^$/d' | wc -l | tr -d '[:space:]')"
  [[ "$direct_count" != 0 && "$shared_count" != 0 ]] || {
    ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="gfxstream feature observation is missing"
    return 1
  }
  [[ "$direct_count" == 1 && "$shared_count" == 1 ]] || {
    ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="gfxstream feature observation is conflicting"
    return 1
  }
  pair="$direct_values/$shared_values"
  android_emulator_feature_policy_configure "$policy" || return 1
  expected="$ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR"
  [[ "$pair" == "$expected" ]] || {
    ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="gfxstream feature policy $policy expected $expected, observed $pair"
    return 1
  }
  ANDROID_EMULATOR_FEATURE_POLICY_LAST_PAIR="$pair"
  ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="GlDirectMem=${pair%/*} HasSharedSlotsHostMemoryAllocator=${pair#*/}"
}

android_emulator_feature_policy_wait() {
  local log_file="$1"
  local policy="$2"
  local timeout_secs="$3"
  local process_guard="$4"
  local deadline last_evidence=""
  [[ "$timeout_secs" =~ ^[1-9][0-9]*$ ]] && (( timeout_secs <= 60 )) || {
    ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="invalid feature observation timeout"
    return 1
  }
  deadline=$((SECONDS + timeout_secs))
  while (( SECONDS < deadline )); do
    if android_emulator_feature_policy_observe "$log_file" "$policy"; then
      return 0
    fi
    last_evidence="$ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE"
    case "$last_evidence" in
      "emulator log is missing" | "gfxstream feature observation is missing") ;;
      *) return 1 ;;
    esac
    "$process_guard" || {
      ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="emulator process exited before feature observation"
      return 1
    }
    sleep 1
  done
  ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE="feature observation timed out: $last_evidence"
  return 1
}
