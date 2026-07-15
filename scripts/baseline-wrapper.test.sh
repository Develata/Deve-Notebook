#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

fail() {
  echo "baseline-wrapper.test: $*" >&2
  exit 1
}

[[ "$(baseline_windows_drive_path 'C:\\tools\\node' && echo yes)" == "yes" ]] \
  || fail "Windows backslash drive paths must be recognized"
[[ "$(baseline_windows_drive_path 'D:/tools/node' && echo yes)" == "yes" ]] \
  || fail "Windows slash drive paths must be recognized"
if baseline_windows_drive_path "/usr/local"; then
  fail "Unix paths must not be recognized as Windows drive paths"
fi

original_repo_probe="$(declare -f baseline_repo_on_wsl_windows_mount)"
baseline_repo_on_wsl_windows_mount() {
  return 0
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

windows_npm="$tmp_dir/npm-windows"
unix_npm="$tmp_dir/npm-unix"
printf '%s\n' '#!/usr/bin/env bash' '[[ "$*" == "prefix -g" ]] || exit 2' "printf '%s\\n' 'C:\\\\tools\\\\node'" >"$windows_npm"
printf '%s\n' '#!/usr/bin/env bash' '[[ "$*" == "prefix -g" ]] || exit 2' "printf '%s\\n' '/usr/local'" >"$unix_npm"
chmod +x "$windows_npm" "$unix_npm"

baseline_npm_runtime_uses_windows_paths "/mnt/e/repo" "$windows_npm" \
  || fail "Windows-backed npm runtime must be detected from its runtime prefix"
if baseline_npm_runtime_uses_windows_paths "/mnt/e/repo" "$unix_npm"; then
  fail "native npm runtime must keep Unix paths"
fi

wslpath() {
  [[ "$1" == "-w" ]] || return 2
  printf 'E:\\repo\\apps\\web\n'
}

converted="$(baseline_npm_prefix_path "/mnt/e/repo" "$windows_npm" "/mnt/e/repo/apps/web")"
[[ "$converted" == 'E:\repo\apps\web' ]] \
  || fail "Windows-backed npm prefix was not converted: $converted"

unchanged="$(baseline_npm_prefix_path "/mnt/e/repo" "$unix_npm" "/mnt/e/repo/apps/web")"
[[ "$unchanged" == "/mnt/e/repo/apps/web" ]] \
  || fail "native npm prefix was unexpectedly converted: $unchanged"

baseline_invocation="$tmp_dir/baseline-invocation"
baseline_bin="$tmp_dir/deve-baseline"
printf '%s\n' '#!/usr/bin/env bash' 'printf "%s\\n" "$*" >"$BASELINE_TEST_INVOCATION"' >"$baseline_bin"
chmod +x "$baseline_bin"
BASELINE_TEST_INVOCATION="$baseline_invocation" \
  DEVE_BASELINE_BIN="$baseline_bin" \
  CARGO_BIN="$tmp_dir/missing-cargo" \
  run_deve_baseline "$ROOT_DIR" "release" "baseline-wrapper-test" "tag-ready"
[[ "$(<"$baseline_invocation")" == "release tag-ready" ]] \
  || fail "runner-owned baseline executable was not reused"

eval "$original_repo_probe"
echo "baseline-wrapper.test: ok"
