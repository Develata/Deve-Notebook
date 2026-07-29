#!/usr/bin/env bash
# Fail-closed resolution of the pinned Android emulator binary.
#
# The generic sdkmanager "emulator" package is a floating input: its version
# moves underneath the pinned system image, and emulator-host/guest protocol
# skew is the root-cause category of the API 37.0 surfaceflinger abort the
# release gate guards against. Resolution never mutates the shared SDK:
#   1. DEVE_MOBILE_ANDROID_EMULATOR_BIN override (must still match the pin);
#   2. the SDK-installed emulator when it already matches the pin exactly;
#   3. a private versioned cache directory, populated by a checksum-bound
#      download (temp file -> SHA-256 verify -> extract -> atomic rename).
# Every path re-asserts the version banner; a mismatch is an error, never a
# silent fallback.

ANDROID_EMULATOR_PIN_VERSION="36.6.11.0"
ANDROID_EMULATOR_PIN_BUILD_ID="15507667"
ANDROID_EMULATOR_PIN_SHA256_LINUX="1eade4cf2df6ea8eeead4902c635897ba12aaa32aac4389eaae0fdb498a5b830"
ANDROID_EMULATOR_PIN_SHA256_WINDOWS="7dfb2c47291db7e0fd7ce45e541706d45827bc9875f5f736bf785585b549e0e4"
ANDROID_EMULATOR_PIN_BASE_URL="https://dl.google.com/android/repository"

android_emulator_pin_error() {
  printf 'android-emulator-pin: %s\n' "$1" >&2
  return 1
}

android_emulator_pin_note() {
  printf 'android-emulator-pin: %s\n' "$1" >&2
}

# Last -version probe result, recorded so mismatch paths can report what the
# candidate binary actually printed instead of failing blind.
ANDROID_EMULATOR_PIN_LAST_PROBE=""

# Matches when the -version output contains the pinned version and build id.
# Deliberately independent of the probe's exit status and line layout: the
# launcher delegates -version to the bundled qemu engine, whose exit code and
# log prefacing vary by host environment. Identity comes from the banner
# tokens alone; downloaded archives are additionally SHA-256 bound.
android_emulator_pin_matches() {
  local binary="$1"
  local output="" rc=0
  output="$("$binary" -version 2>&1)" || rc=$?
  ANDROID_EMULATOR_PIN_LAST_PROBE="exit=$rc first-lines: $(printf '%s' "$output" | head -n 3 | tr '\n' '|')"
  [[ "$output" == *"version $ANDROID_EMULATOR_PIN_VERSION"* ]] || return 1
  [[ "$output" == *"build_id $ANDROID_EMULATOR_PIN_BUILD_ID"* ]] || return 1
}

android_emulator_pin_archive_name() {
  case "$(uname -s)" in
    Linux) printf 'emulator-linux_x64-%s.zip\n' "$ANDROID_EMULATOR_PIN_BUILD_ID" ;;
    MINGW* | MSYS* | CYGWIN*)
      printf 'emulator-windows_x64-%s.zip\n' "$ANDROID_EMULATOR_PIN_BUILD_ID" ;;
    *) return 1 ;;
  esac
}

android_emulator_pin_archive_sha256() {
  case "$(uname -s)" in
    Linux) printf '%s\n' "$ANDROID_EMULATOR_PIN_SHA256_LINUX" ;;
    MINGW* | MSYS* | CYGWIN*) printf '%s\n' "$ANDROID_EMULATOR_PIN_SHA256_WINDOWS" ;;
    *) return 1 ;;
  esac
}

android_emulator_pin_cache_root() {
  printf '%s\n' \
    "${DEVE_MOBILE_ANDROID_EMULATOR_PIN_DIR:-$HOME/.cache/deve-android-emulator-pin}"
}

android_emulator_pin_binary_in() {
  local root="$1"
  local candidate
  for candidate in "$root/emulator/emulator" "$root/emulator/emulator.exe"; do
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

android_emulator_pin_download() {
  local cache_root archive_name archive_sha url staging archive extracted binary
  cache_root="$(android_emulator_pin_cache_root)"
  archive_name="$(android_emulator_pin_archive_name)" \
    || { android_emulator_pin_error "unsupported host OS for pinned download"; return 1; }
  archive_sha="$(android_emulator_pin_archive_sha256)" || return 1
  [[ "$archive_sha" =~ ^[0-9a-f]{64}$ ]] \
    || { android_emulator_pin_error "no SHA-256 pinned for this host OS"; return 1; }

  url="$ANDROID_EMULATOR_PIN_BASE_URL/$archive_name"
  staging="$cache_root/staging.$$"
  archive="$staging/$archive_name"
  extracted="$staging/extracted"
  mkdir -p "$extracted"

  curl -fsSL --retry 3 -o "$archive" "$url" \
    || { rm -rf -- "$staging"; android_emulator_pin_error "download failed: $url"; return 1; }
  printf '%s  %s\n' "$archive_sha" "$archive" | sha256sum -c --quiet - \
    || { rm -rf -- "$staging"; android_emulator_pin_error "SHA-256 mismatch for $archive_name"; return 1; }
  if command -v unzip >/dev/null 2>&1; then
    unzip -q "$archive" -d "$extracted" \
      || { rm -rf -- "$staging"; android_emulator_pin_error "extract failed: $archive_name"; return 1; }
  else
    python -m zipfile -e "$archive" "$extracted" \
      || { rm -rf -- "$staging"; android_emulator_pin_error "extract failed: $archive_name"; return 1; }
  fi
  android_emulator_pin_binary_in "$extracted" >/dev/null \
    || { rm -rf -- "$staging"; android_emulator_pin_error "archive lacks emulator/ payload"; return 1; }

  rm -rf -- "${cache_root:?}/$ANDROID_EMULATOR_PIN_BUILD_ID"
  mv -f -- "$extracted" "$cache_root/$ANDROID_EMULATOR_PIN_BUILD_ID" \
    || { rm -rf -- "$staging"; android_emulator_pin_error "atomic install failed"; return 1; }
  rm -rf -- "$staging"
  binary="$(android_emulator_pin_binary_in "$cache_root/$ANDROID_EMULATOR_PIN_BUILD_ID")" || return 1
  chmod +x "$binary" 2>/dev/null || true
  printf '%s\n' "$binary"
}

# Main entry: echoes the resolved pinned emulator binary, or fails closed.
android_resolve_pinned_emulator() {
  local binary cache_root

  if [[ -n "${DEVE_MOBILE_ANDROID_EMULATOR_BIN:-}" ]]; then
    binary="${DEVE_MOBILE_ANDROID_EMULATOR_BIN}"
    android_emulator_pin_matches "$binary" \
      || { android_emulator_pin_note "override probe: $ANDROID_EMULATOR_PIN_LAST_PROBE"; android_emulator_pin_error "override DEVE_MOBILE_ANDROID_EMULATOR_BIN does not match pin $ANDROID_EMULATOR_PIN_VERSION build $ANDROID_EMULATOR_PIN_BUILD_ID"; return 1; }
    printf '%s\n' "$binary"
    return 0
  fi

  if binary="$(android_tool_path emulator)"; then
    if android_emulator_pin_matches "$binary"; then
      printf '%s\n' "$binary"
      return 0
    fi
    android_emulator_pin_note "SDK emulator at $binary did not match pin ($ANDROID_EMULATOR_PIN_LAST_PROBE); falling back to pinned download"
  fi

  cache_root="$(android_emulator_pin_cache_root)"
  if binary="$(android_emulator_pin_binary_in "$cache_root/$ANDROID_EMULATOR_PIN_BUILD_ID")"; then
    if android_emulator_pin_matches "$binary"; then
      printf '%s\n' "$binary"
      return 0
    fi
    android_emulator_pin_note "cached emulator at $binary did not match pin ($ANDROID_EMULATOR_PIN_LAST_PROBE); re-downloading"
  fi

  binary="$(android_emulator_pin_download)" || return 1
  android_emulator_pin_matches "$binary" \
    || { android_emulator_pin_note "download probe: $ANDROID_EMULATOR_PIN_LAST_PROBE"; android_emulator_pin_error "downloaded emulator does not match pin $ANDROID_EMULATOR_PIN_VERSION build $ANDROID_EMULATOR_PIN_BUILD_ID"; return 1; }
  printf '%s\n' "$binary"
}
