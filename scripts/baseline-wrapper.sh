#!/usr/bin/env bash
set -euo pipefail

resolve_baseline_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    command -v cargo
    return 0
  fi

  if command -v cargo.exe >/dev/null 2>&1; then
    command -v cargo.exe
    return 0
  fi

  if command -v bash >/dev/null 2>&1; then
    bash -lc 'command -v cargo || command -v cargo.exe' 2>/dev/null | head -n 1
  fi
}

run_deve_baseline() {
  local root_dir="$1"
  local baseline="$2"
  local label="$3"
  local cargo_bin="${CARGO:-}"

  if [[ -z "$cargo_bin" ]]; then
    cargo_bin="$(resolve_baseline_cargo || true)"
  fi

  if [[ -z "$cargo_bin" ]]; then
    echo "$label: cargo is required to run cargo run -p deve_baseline -- $baseline" >&2
    return 1
  fi

  if ! (
    cd "$root_dir"
    "$cargo_bin" run -p deve_baseline -- "$baseline"
  ); then
    echo "$label: cargo run -p deve_baseline -- $baseline failed" >&2
    return 1
  fi
}
