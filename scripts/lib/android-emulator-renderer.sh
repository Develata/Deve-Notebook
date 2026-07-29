#!/usr/bin/env bash
# Bounded proof of the renderer selected by the owned Android emulator.

ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES="${ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES:-1048576}"
ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE=""

android_emulator_renderer_verify() {
  local log_file="$1"
  local scan="" modes="" distinct_count=0

  ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE=""
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
  case "$modes" in
    "swiftshader swangle")
      ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="vulkan/gles=$modes"
      return 0
      ;;
    *)
      ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE="unapproved renderer selection: $modes"
      return 1
      ;;
  esac
}
