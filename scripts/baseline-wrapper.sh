#!/usr/bin/env bash
set -euo pipefail

baseline_repo_on_wsl_windows_mount() {
  local root_dir="$1"
  [[ "$root_dir" == /mnt/* ]] && grep -qi microsoft /proc/version 2>/dev/null
}

resolve_baseline_cargo() {
  local root_dir="${1:-}"

  if baseline_repo_on_wsl_windows_mount "$root_dir"; then
    if command -v cargo.exe >/dev/null 2>&1; then
      command -v cargo.exe
      return 0
    fi

    if command -v cargo >/dev/null 2>&1; then
      command -v cargo
      return 0
    fi

    if command -v bash >/dev/null 2>&1; then
      local login_cargo
      login_cargo="$(bash -lc 'command -v cargo' 2>/dev/null | head -n 1)"
      if [[ -n "$login_cargo" ]]; then
        printf '%s\n' "$login_cargo"
        return 0
      fi
    fi
  else
    if command -v cargo >/dev/null 2>&1; then
      command -v cargo
      return 0
    fi

    if command -v cargo.exe >/dev/null 2>&1; then
      command -v cargo.exe
      return 0
    fi
  fi

  if command -v bash >/dev/null 2>&1; then
    bash -lc 'command -v cargo || command -v cargo.exe' 2>/dev/null | head -n 1
  fi
}

run_baseline_cargo() {
  local root_dir="$1"
  local label="$2"
  local cargo_bin="${CARGO_BIN:-${CARGO:-}}"
  shift 2

  if [[ -z "$cargo_bin" ]]; then
    cargo_bin="$(resolve_baseline_cargo "$root_dir" || true)"
  fi

  if [[ -z "$cargo_bin" ]]; then
    echo "$label: cargo is required to run cargo $*" >&2
    return 1
  fi

  (
    cd "$root_dir"
    if baseline_repo_on_wsl_windows_mount "$root_dir" && baseline_windows_exe "$cargo_bin"; then
      WSLENV="$(baseline_deve_wslenv)" "$cargo_bin" "$@"
    else
      "$cargo_bin" "$@"
    fi
  )
}

baseline_windows_exe() {
  local path
  path="$(baseline_ascii_lower "$1")"
  [[ "$path" == *.exe ]]
}

baseline_ascii_lower() {
  LC_ALL=C tr '[:upper:]' '[:lower:]' <<<"$1"
}

baseline_windows_drive_path() {
  local path="$1"
  [[ "$path" =~ ^[A-Za-z]:[\\/] ]]
}

baseline_npm_runtime_uses_windows_paths() {
  local root_dir="$1"
  local npm_bin="$2"
  local runtime_prefix

  baseline_repo_on_wsl_windows_mount "$root_dir" || return 1
  runtime_prefix="$("$npm_bin" prefix -g 2>/dev/null | tr -d '\r' | head -n 1)" || return 1
  baseline_windows_drive_path "$runtime_prefix"
}

baseline_npm_prefix_path() {
  local root_dir="$1"
  local npm_bin="$2"
  local prefix_path="$3"

  if baseline_npm_runtime_uses_windows_paths "$root_dir" "$npm_bin"; then
    if ! command -v wslpath >/dev/null 2>&1; then
      echo "baseline npm path conversion: wslpath is required for Windows-backed npm" >&2
      return 1
    fi
    wslpath -w "$prefix_path"
  else
    printf '%s\n' "$prefix_path"
  fi
}

baseline_windows_path_to_unix() {
  local path="$1"
  local drive rest lower
  if [[ "$path" =~ ^([A-Za-z]):\\(.*)$ ]]; then
    drive="${BASH_REMATCH[1]}"
    rest="${BASH_REMATCH[2]//\\//}"
    lower="$(printf '%s' "$drive" | tr '[:upper:]' '[:lower:]')"
    printf '/mnt/%s/%s\n' "$lower" "$rest"
    printf '/%s/%s\n' "$lower" "$rest"
  fi
}

baseline_is_runnable_tool() {
  local path="$1"
  [[ -n "$path" ]] || return 1
  [[ -f "$path" || -x "$path" ]] || return 1
  "$path" --version >/dev/null 2>&1
}

baseline_resolve_tool() {
  local tool_name="$1"
  shift
  local candidate candidate_path converted

  for candidate in "$tool_name" "$@"; do
    if command -v "$candidate" >/dev/null 2>&1; then
      candidate_path="$(command -v "$candidate")"
      if baseline_is_runnable_tool "$candidate_path"; then
        printf '%s\n' "$candidate_path"
        return 0
      fi
    fi

    if command -v where.exe >/dev/null 2>&1; then
      candidate_path="$(where.exe "$candidate" 2>/dev/null | tr -d '\r' | head -n1 || true)"
      if baseline_is_runnable_tool "$candidate_path"; then
        printf '%s\n' "$candidate_path"
        return 0
      fi
      while IFS= read -r converted; do
        if baseline_is_runnable_tool "$converted"; then
          printf '%s\n' "$converted"
          return 0
        fi
      done < <(baseline_windows_path_to_unix "$candidate_path")
    fi
  done

  return 1
}

baseline_wslenv_has_name() {
  local wslenv="$1"
  local name="$2"
  local entry
  local -a entries

  IFS=':' read -ra entries <<<"$wslenv"
  for entry in "${entries[@]}"; do
    [[ "${entry%%/*}" == "$name" ]] && return 0
  done
  return 1
}

baseline_deve_wslenv() {
  local wslenv="${WSLENV:-}"
  local line
  local name

  while IFS= read -r line; do
    name="${line%%=*}"
    [[ "$name" == DEVE_* ]] || continue
    [[ "$name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
    if ! baseline_wslenv_has_name "$wslenv" "$name"; then
      wslenv="${wslenv:+$wslenv:}$name"
    fi
  done < <(env)

  printf '%s\n' "$wslenv"
}

run_deve_baseline() {
  local root_dir="$1"
  local baseline="$2"
  local label="$3"
  local baseline_bin="${DEVE_BASELINE_BIN:-}"
  local cargo_bin="${CARGO_BIN:-${CARGO:-}}"
  shift 3

  if [[ -n "$baseline_bin" ]]; then
    if ! (
      cd "$root_dir"
      "$baseline_bin" "$baseline" "$@"
    ); then
      echo "$label: $baseline_bin $baseline failed" >&2
      return 1
    fi
    return 0
  fi

  if [[ -z "$cargo_bin" ]]; then
    cargo_bin="$(resolve_baseline_cargo "$root_dir" || true)"
  fi

  if [[ -z "$cargo_bin" ]]; then
    echo "$label: cargo is required to run cargo run -p deve_baseline -- $baseline" >&2
    return 1
  fi

  if ! (
    cd "$root_dir"
    if baseline_repo_on_wsl_windows_mount "$root_dir" && baseline_windows_exe "$cargo_bin"; then
      WSLENV="$(baseline_deve_wslenv)" "$cargo_bin" run --quiet -p deve_baseline -- "$baseline" "$@"
    else
      "$cargo_bin" run --quiet -p deve_baseline -- "$baseline" "$@"
    fi
  ); then
    echo "$label: cargo run -p deve_baseline -- $baseline failed" >&2
    return 1
  fi
}
