#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- native-target-host-evidence [reports...]
REPORT_ARGS=("$@")

if baseline_repo_on_wsl_windows_mount "$ROOT_DIR"; then
  cargo_bin="${CARGO_BIN:-${CARGO:-}}"
  if [[ -z "$cargo_bin" ]]; then
    cargo_bin="$(resolve_baseline_cargo "$ROOT_DIR" || true)"
  fi

  if [[ -n "$cargo_bin" ]] && baseline_windows_exe "$cargo_bin" && command -v wslpath >/dev/null 2>&1; then
    for index in "${!REPORT_ARGS[@]}"; do
      case "${REPORT_ARGS[$index]}" in
        /*) REPORT_ARGS[$index]="$(wslpath -w "${REPORT_ARGS[$index]}")" ;;
      esac
    done
  fi
fi

run_deve_baseline "$ROOT_DIR" "native-target-host-evidence" "native-target-host-evidence-check" "${REPORT_ARGS[@]}"
