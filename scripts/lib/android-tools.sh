#!/usr/bin/env bash

android_as_posix_path() {
  local path="$1"

  if command -v cygpath >/dev/null 2>&1; then
    cygpath -u "$path" 2>/dev/null && return 0
  fi
  printf '%s\n' "$path"
}

android_sdk_root() {
  local value
  local path

  for value in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}"; do
    [[ -n "$value" ]] || continue
    path="$(android_as_posix_path "$value")"
    [[ -d "$path" ]] || continue
    printf '%s\n' "$path"
    return 0
  done
  return 1
}

android_tool_path() {
  local tool="$1"
  local sdk
  local candidate
  local candidates=()

  if command -v "$tool" >/dev/null 2>&1; then
    command -v "$tool"
    return 0
  fi

  sdk="$(android_sdk_root)" || return 1
  candidates=(
    "$sdk/platform-tools/$tool"
    "$sdk/platform-tools/$tool.exe"
    "$sdk/emulator/$tool"
    "$sdk/emulator/$tool.exe"
    "$sdk/cmdline-tools/latest/bin/$tool"
    "$sdk/cmdline-tools/latest/bin/$tool.bat"
    "$sdk/cmdline-tools/latest/bin/$tool.exe"
  )
  for candidate in "${candidates[@]}"; do
    [[ -f "$candidate" ]] || continue
    printf '%s\n' "$candidate"
    return 0
  done
  return 1
}

android_java_major() {
  local java_bin="$1"
  local version
  local major

  version="$("$java_bin" -version 2>&1 | sed -n 's/.*version "\([^"]*\)".*/\1/p' | head -n 1)"
  [[ -n "$version" ]] || return 1
  case "$version" in
    1.*)
      major="${version#1.}"
      major="${major%%.*}"
      ;;
    *)
      major="${version%%.*}"
      ;;
  esac
  [[ "$major" =~ ^[0-9]+$ ]] || return 1
  printf '%s\n' "$major"
}

android_java_is_modern() {
  local java_bin="$1"
  local major

  major="$(android_java_major "$java_bin")" || return 1
  (( major >= 17 ))
}

android_current_java_is_modern() {
  local java_bin

  java_bin="$(command -v java 2>/dev/null || true)"
  [[ -n "$java_bin" ]] || return 1
  android_java_is_modern "$java_bin"
}

android_studio_jbr_candidates() {
  local path
  local windows_user="${USER:-}"

  if [[ -n "${ANDROID_STUDIO_JBR:-}" ]]; then
    android_as_posix_path "$ANDROID_STUDIO_JBR"
  fi
  if [[ -n "${ANDROID_STUDIO_HOME:-}" ]]; then
    path="$(android_as_posix_path "$ANDROID_STUDIO_HOME")"
    printf '%s\n' "$path/jbr"
  fi
  if [[ -n "${USERPROFILE:-}" ]]; then
    path="$(android_as_posix_path "$USERPROFILE")"
    printf '%s\n' "$path/scoop/apps/android-studio/current/jbr"
    windows_user="${path##*/}"
  fi
  if [[ -n "$windows_user" ]]; then
    printf '%s\n' "/c/Users/$windows_user/scoop/apps/android-studio/current/jbr"
  fi
  printf '%s\n' "/c/Program Files/Android/Android Studio/jbr"
}

android_prepare_java_home() {
  local jbr
  local java_bin

  if android_current_java_is_modern; then
    return 0
  fi

  while IFS= read -r jbr; do
    [[ -n "$jbr" && -d "$jbr" ]] || continue
    java_bin="$jbr/bin/java"
    [[ -f "$java_bin.exe" ]] && java_bin="$java_bin.exe"
    [[ -f "$java_bin" ]] || continue
    android_java_is_modern "$java_bin" || continue
    export JAVA_HOME="$jbr"
    export PATH="$JAVA_HOME/bin:$PATH"
    echo "android-tools: using Android Studio JBR: $JAVA_HOME"
    return 0
  done < <(android_studio_jbr_candidates)

  return 1
}

android_run_tool() {
  local tool="$1"
  local tool_path
  shift

  tool_path="$(android_tool_path "$tool")" || return 1
  "$tool_path" "$@"
}
