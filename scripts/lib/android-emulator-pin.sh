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
#      download (temp file -> SHA-256 verify -> extract -> build-locked rename).
# Every path re-asserts the version banner; a mismatch is an error, never a
# silent fallback.

ANDROID_EMULATOR_PIN_VERSION="36.6.11.0"
ANDROID_EMULATOR_PIN_BUILD_ID="15507667"
ANDROID_EMULATOR_PIN_SHA256_LINUX="1eade4cf2df6ea8eeead4902c635897ba12aaa32aac4389eaae0fdb498a5b830"
ANDROID_EMULATOR_PIN_SHA256_WINDOWS="7dfb2c47291db7e0fd7ce45e541706d45827bc9875f5f736bf785585b549e0e4"
ANDROID_EMULATOR_PIN_BASE_URL="https://dl.google.com/android/repository"
ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS="${ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS:-15}"
ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES="${ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES:-65536}"
ANDROID_EMULATOR_PIN_PUBLISH_LOCK_TIMEOUT_SECS="${ANDROID_EMULATOR_PIN_PUBLISH_LOCK_TIMEOUT_SECS:-120}"

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

# Matches only a bounded canonical `Android emulator version ... (build_id
# ...)` banner. The launcher may delegate -version to a bundled qemu engine
# whose exit status and preceding log lines vary by host, so a non-zero exit is
# accepted only when the complete canonical banner identifies the pin.
android_emulator_pin_matches() {
  local binary="$1"
  local probe_dir output_file output="" banner="" rc=0 bytes=0
  local -a probe_status=()

  if ! [[ "$ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS" =~ ^[1-9][0-9]?$ ]] \
      || (( ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS > 30 )); then
    ANDROID_EMULATOR_PIN_LAST_PROBE="invalid timeout bound"
    return 1
  fi
  if ! [[ "$ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES" =~ ^[1-9][0-9]*$ ]] \
      || (( ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES < 1024 || ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES > 65536 )); then
    ANDROID_EMULATOR_PIN_LAST_PROBE="invalid output bound"
    return 1
  fi
  [[ -f "$binary" && -x "$binary" ]] \
    || { ANDROID_EMULATOR_PIN_LAST_PROBE="binary missing or not executable"; return 1; }
  command -v timeout >/dev/null 2>&1 \
    || { ANDROID_EMULATOR_PIN_LAST_PROBE="timeout command unavailable"; return 1; }

  probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/deve-android-emulator-pin.XXXXXX")" \
    || { ANDROID_EMULATOR_PIN_LAST_PROBE="probe temp directory unavailable"; return 1; }
  output_file="$probe_dir/version.log"
  {
    timeout --signal=TERM --kill-after=5s \
      "${ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS}s" "$binary" -version 2>&1 \
      | head -c "$((ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES + 1))" >"$output_file"
    probe_status=("${PIPESTATUS[@]}")
  } || :
  rc="${probe_status[0]:-1}"
  bytes="$(wc -c <"$output_file" | tr -d '[:space:]')"
  output="$(tr -d '\r' <"$output_file")"
  rm -rf -- "$probe_dir"

  ANDROID_EMULATOR_PIN_LAST_PROBE="exit=$rc bytes=$bytes first-lines: $(printf '%s' "$output" | head -n 3 | cut -c 1-256 | tr '\n' '|')"
  (( bytes <= ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES )) || return 1
  [[ "$rc" != "124" && "$rc" != "137" ]] || return 1
  banner="$(printf '%s\n' "$output" \
    | grep -aE -m 1 '^[[:space:]]*Android emulator version [0-9]+([.][0-9]+){3} [(]build_id [0-9]+[)]([[:space:]]|$)' \
    || true)"
  [[ "$banner" =~ ^[[:space:]]*Android[[:space:]]emulator[[:space:]]version[[:space:]]([0-9]+([.][0-9]+){3})[[:space:]]\(build_id[[:space:]]([0-9]+)\) ]] \
    || return 1
  [[ "${BASH_REMATCH[1]}" == "$ANDROID_EMULATOR_PIN_VERSION" ]] || return 1
  [[ "${BASH_REMATCH[3]}" == "$ANDROID_EMULATOR_PIN_BUILD_ID" ]] || return 1
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

android_emulator_pin_acquire_publish_lock() {
  local lock_file="$1" owner_file="$2" owner_token="$3" deadline
  if ! [[ "$ANDROID_EMULATOR_PIN_PUBLISH_LOCK_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] \
      || (( ANDROID_EMULATOR_PIN_PUBLISH_LOCK_TIMEOUT_SECS > 600 )); then
    android_emulator_pin_error "invalid emulator publish lock timeout"
    return 1
  fi
  if ! (umask 077; set -o noclobber; printf '%s\n' "$owner_token" >"$owner_file"); then
    android_emulator_pin_error "failed to create unique emulator publication owner"
    return 1
  fi
  deadline=$((SECONDS + ANDROID_EMULATOR_PIN_PUBLISH_LOCK_TIMEOUT_SECS))
  while ! ln -- "$owner_file" "$lock_file" 2>/dev/null; do
    if (( SECONDS >= deadline )); then
      android_emulator_pin_error "timed out waiting for emulator build publication lock"
      return 1
    fi
    sleep 1
  done
}

android_emulator_pin_owns_publish_lock() {
  local lock_file="$1" owner_token="$2"
  [[ -f "$lock_file" && ! -L "$lock_file" ]] || return 1
  [[ "$(cat -- "$lock_file")" == "$owner_token" ]]
}

android_emulator_pin_cleanup_publication() {
  local staging="$1" lock_file="$2" owner_file="$3" owner_token="$4" cleanup_failed=0
  rm -rf -- "$staging" || cleanup_failed=1
  if android_emulator_pin_owns_publish_lock "$lock_file" "$owner_token"; then
    rm -f -- "$lock_file" || cleanup_failed=1
  fi
  rm -f -- "$owner_file" || cleanup_failed=1
  (( cleanup_failed == 0 ))
}

android_emulator_pin_publish_extracted() {
  local cache_root="$1" staging="$2" extracted="$3"
  local destination lock_file owner_file owner_token binary publish_error=""
  local previous_hup previous_int previous_term cleanup_failed=0
  destination="$cache_root/$ANDROID_EMULATOR_PIN_BUILD_ID"
  lock_file="$cache_root/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.lock"
  owner_token="${BASHPID:-$$}.$RANDOM.$RANDOM"
  owner_file="$cache_root/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.owner.$owner_token"
  previous_hup="$(trap -p HUP)"
  previous_int="$(trap -p INT)"
  previous_term="$(trap -p TERM)"
  trap 'android_emulator_pin_cleanup_publication "$staging" "$lock_file" "$owner_file" "$owner_token" || true; exit 130' HUP INT TERM

  if ! android_emulator_pin_acquire_publish_lock "$lock_file" "$owner_file" "$owner_token"; then
    android_emulator_pin_cleanup_publication \
      "$staging" "$lock_file" "$owner_file" "$owner_token" || true
    trap - HUP INT TERM
    [[ -z "$previous_hup" ]] || eval "$previous_hup"
    [[ -z "$previous_int" ]] || eval "$previous_int"
    [[ -z "$previous_term" ]] || eval "$previous_term"
    return 1
  fi
  if ! android_emulator_pin_owns_publish_lock "$lock_file" "$owner_token"; then
    publish_error="emulator publication lock ownership was lost"
  elif binary="$(android_emulator_pin_binary_in "$destination")" \
      && android_emulator_pin_matches "$binary"; then
    :
  elif [[ -e "$destination" ]]; then
    publish_error="existing emulator cache entry is invalid; refusing automatic replacement while it may be in use"
  else
    if ! mv -- "$extracted" "$destination"; then
      publish_error="atomic emulator cache publication failed"
    else
      binary="$(android_emulator_pin_binary_in "$destination")" \
        || publish_error="published emulator cache lacks binary"
    fi
  fi
  android_emulator_pin_cleanup_publication "$staging" "$lock_file" "$owner_file" "$owner_token" \
    || cleanup_failed=1
  trap - HUP INT TERM
  [[ -z "$previous_hup" ]] || eval "$previous_hup"
  [[ -z "$previous_int" ]] || eval "$previous_int"
  [[ -z "$previous_term" ]] || eval "$previous_term"
  if (( cleanup_failed )); then
    android_emulator_pin_error "failed to clean emulator publication staging or lock"
    return 1
  fi
  if [[ -n "$publish_error" ]]; then
    android_emulator_pin_error "$publish_error"
    return 1
  fi
  chmod +x "$binary" 2>/dev/null || true
  printf '%s\n' "$binary"
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
  staging="$cache_root/staging.${BASHPID:-$$}.$RANDOM"
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
  binary="$(android_emulator_pin_binary_in "$extracted")" \
    || { rm -rf -- "$staging"; android_emulator_pin_error "archive lacks emulator/ payload"; return 1; }
  chmod +x "$binary" 2>/dev/null || true
  android_emulator_pin_matches "$binary" \
    || { rm -rf -- "$staging"; android_emulator_pin_note "download probe: $ANDROID_EMULATOR_PIN_LAST_PROBE"; android_emulator_pin_error "downloaded emulator does not match pin $ANDROID_EMULATOR_PIN_VERSION build $ANDROID_EMULATOR_PIN_BUILD_ID"; return 1; }
  android_emulator_pin_publish_extracted "$cache_root" "$staging" "$extracted"
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
    android_emulator_pin_note "cached emulator at $binary did not match pin ($ANDROID_EMULATOR_PIN_LAST_PROBE)"
    android_emulator_pin_error "refusing to replace an invalid emulator cache entry that may still be in use"
    return 1
  elif [[ -e "$cache_root/$ANDROID_EMULATOR_PIN_BUILD_ID" ]]; then
    android_emulator_pin_error "emulator cache entry exists without a canonical binary; refusing automatic replacement"
    return 1
  fi

  binary="$(android_emulator_pin_download)" || return 1
  android_emulator_pin_matches "$binary" \
    || { android_emulator_pin_note "download probe: $ANDROID_EMULATOR_PIN_LAST_PROBE"; android_emulator_pin_error "downloaded emulator does not match pin $ANDROID_EMULATOR_PIN_VERSION build $ANDROID_EMULATOR_PIN_BUILD_ID"; return 1; }
  printf '%s\n' "$binary"
}
