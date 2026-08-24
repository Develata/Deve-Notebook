#!/usr/bin/env bash

# Verifies the immutable two-APK handoff used by parallel Android target-host
# consumers. Callers own artifact download and runner isolation.

android_prebuilt_apk_manifest_verify() {
  local root_dir="$1"
  local manifest_path="$2"
  local release_apk="$3"
  local debug_apk="$4"
  local expected line digest path
  local -a lines

  [[ -d "$root_dir" ]] || return 1
  [[ -f "$manifest_path" && ! -L "$manifest_path" ]] || return 1
  [[ "$release_apk" != /* && "$debug_apk" != /* ]] || return 1
  [[ "$release_apk" != *".."* && "$debug_apk" != *".."* ]] || return 1
  [[ -f "$root_dir/$release_apk" && ! -L "$root_dir/$release_apk" ]] || return 1
  [[ -f "$root_dir/$debug_apk" && ! -L "$root_dir/$debug_apk" ]] || return 1
  command -v sha256sum >/dev/null 2>&1 || return 1

  mapfile -t lines <"$manifest_path"
  (( ${#lines[@]} == 2 )) || return 1
  for expected in "$release_apk" "$debug_apk"; do
    local matches=0
    for line in "${lines[@]}"; do
      [[ "$line" =~ ^([0-9a-f]{64})[[:space:]][\ \*](.+)$ ]] || return 1
      digest="${BASH_REMATCH[1]}"
      path="${BASH_REMATCH[2]}"
      [[ "$path" == "$release_apk" || "$path" == "$debug_apk" ]] || return 1
      [[ "$path" == "$expected" ]] && matches=$((matches + 1))
      [[ -n "$digest" ]] || return 1
    done
    (( matches == 1 )) || return 1
  done

  (cd "$root_dir" && sha256sum --check --strict "$manifest_path" >/dev/null 2>&1)
}
