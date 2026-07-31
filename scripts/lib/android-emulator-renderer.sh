#!/usr/bin/env bash
# Bounded proof of the renderer selected by the owned Android emulator.

ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES="${ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES:-1048576}"
ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE=""
ANDROID_EMULATOR_RENDERER_LAST_MODE=""

android_emulator_renderer_observe() {
  local log_file="$1"
  local scan="" modes="" distinct_count=0

  ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE=""
  ANDROID_EMULATOR_RENDERER_LAST_MODE=""
  if ! [[ "$ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES" =~ ^[1-9][0-9]*$ ]] \
      || (( ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES < 1024 || ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES > 4194304 )); then
    ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="invalid renderer log read bound"
    return 1
  fi
  [[ -f "$log_file" ]] \
    || { ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="renderer log is missing"; return 1; }

  if ! scan="$(
    head -c "$ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES" "$log_file" \
      | awk '
          {
            remaining = $0
            if (remaining ~ /swiftshader_indirect/) {
              legacy = 1
            }
            while (match(remaining, /vulkan_mode_selected:[^[:space:]]+[[:space:]]+gles_mode_selected:[^[:space:]]+/)) {
              match_start = RSTART
              match_length = RLENGTH
              pair = substr(remaining, match_start, match_length)
              sub(/^vulkan_mode_selected:/, "", pair)
              sub(/[[:space:]]+gles_mode_selected:/, " ", pair)
              print "pair:" pair
              remaining = substr(remaining, match_start + match_length)
            }
          }
          END {
            if (legacy) {
              print "legacy"
            }
          }
        '
  )"; then
    ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="renderer log bounded scan failed"
    return 1
  fi

  if printf '%s\n' "$scan" | grep -qx 'legacy'; then
    ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="legacy swiftshader_indirect selected"
    return 1
  fi

  modes="$(printf '%s\n' "$scan" | sed -n 's/^pair://p' | sort -u)"
  distinct_count="$(printf '%s\n' "$modes" | sed '/^$/d' | wc -l | tr -d '[:space:]')"
  [[ "$distinct_count" != "0" ]] \
    || { ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="renderer selection is missing within bounded log prefix"; return 1; }
  [[ "$distinct_count" == "1" ]] \
    || { ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="conflicting renderer selections: $(printf '%s' "$modes" | tr '\n' '|')"; return 1; }
  ANDROID_EMULATOR_RENDERER_LAST_MODE="$modes"
  ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="vulkan/gles=$modes"
}

android_emulator_renderer_wait() {
  local log_file="$1"
  local timeout_secs="$2"
  local process_guard="$3"
  local deadline
  if ! [[ "$timeout_secs" =~ ^[1-9][0-9]*$ ]] || (( timeout_secs > 60 )); then
    ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="invalid renderer observation timeout"
    return 1
  fi
  deadline=$((SECONDS + timeout_secs))
  while (( SECONDS < deadline )); do
    android_emulator_renderer_observe "$log_file" && return 0
    case "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" in
      "renderer selection is missing"* | "renderer log is missing") ;;
      *) return 1 ;;
    esac
    "$process_guard" || return 1
    sleep 1
  done
  ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="renderer observation timed out"
  return 1
}

android_emulator_renderer_verify() {
  local log_file="$1"
  android_emulator_renderer_observe "$log_file" || return 1
  case "$ANDROID_EMULATOR_RENDERER_LAST_MODE" in
    "swiftshader swangle")
      return 0
      ;;
    *)
      ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="unapproved renderer selection: $ANDROID_EMULATOR_RENDERER_LAST_MODE"
      return 1
      ;;
  esac
}
