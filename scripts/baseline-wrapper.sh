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

baseline_windows_exe() {
  local path="${1,,}"
  [[ "$path" == *.exe ]]
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
  local cargo_bin="${CARGO_BIN:-${CARGO:-}}"
  shift 3

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
