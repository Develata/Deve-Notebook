#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/lib/docker-msys.sh
source "$ROOT_DIR/scripts/lib/docker-msys.sh"

CAPTURE_FILE="$(mktemp)"
trap 'rm -f "$CAPTURE_FILE"' EXIT

record_docker_argv() {
  {
    printf 'msys=%s\n' "${MSYS2_ARG_CONV_EXCL:-<unset>}"
    printf 'arg=%s\n' "$@"
  } >"$CAPTURE_FILE"
}

export MSYS2_ARG_CONV_EXCL='/preserve-caller-value'
docker_run_without_msys_arg_conversion \
  record_docker_argv run -d -e DEVE_LEDGER_DIR=/data/ledger deve-notebook:test

grep -Fxq 'msys=*' "$CAPTURE_FILE"
grep -Fxq 'arg=DEVE_LEDGER_DIR=/data/ledger' "$CAPTURE_FILE"
grep -Fxq 'arg=deve-notebook:test' "$CAPTURE_FILE"
[[ "$MSYS2_ARG_CONV_EXCL" == '/preserve-caller-value' ]]

echo "docker-msys-argv-test: ok"
