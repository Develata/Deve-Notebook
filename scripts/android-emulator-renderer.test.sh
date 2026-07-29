#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-emulator-renderer.sh"

temporary="$(mktemp -d)"
log="$temporary/emulator.log"
cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT

fail_test() {
  printf 'android-emulator-renderer.test: %s\n' "$*" >&2
  exit 1
}

expect_match() {
  android_emulator_renderer_verify "$log" \
    || fail_test "expected match: $ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE"
}

expect_reject() {
  if android_emulator_renderer_verify "$log"; then
    fail_test "expected fail-closed renderer result"
  fi
}

printf 'INFO | emuglConfig_init: vulkan_mode_selected:swiftshader gles_mode_selected:swangle\n' >"$log"
expect_match
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == "vulkan/gles=swiftshader swangle" ]] \
  || fail_test "canonical swangle renderer evidence was not recorded"

printf 'INFO | emuglConfig_init: vulkan_mode_selected:swangle gles_mode_selected:swiftshader\n' >"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == unapproved* ]] \
  || fail_test "reversed renderer pair was not rejected"

for unapproved_pair in \
  "swiftshader swiftshader" \
  "swangle swangle" \
  "software software"; do
  read -r vulkan_mode gles_mode <<<"$unapproved_pair"
  printf 'INFO | emuglConfig_init: vulkan_mode_selected:%s gles_mode_selected:%s\n' \
    "$vulkan_mode" "$gles_mode" >"$log"
  expect_reject
  [[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == unapproved* ]] \
    || fail_test "former fallback renderer pair was not rejected: $unapproved_pair"
done

printf 'INFO | emulator started without a renderer selection\n' >"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == renderer\ selection\ is\ missing* ]] \
  || fail_test "missing renderer evidence was not reported"

printf '%s\n' \
  'INFO | emuglConfig_init: vulkan_mode_selected:swiftshader gles_mode_selected:swiftshader' \
  'INFO | emuglConfig_init: vulkan_mode_selected:host gles_mode_selected:host' >"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == conflicting* ]] \
  || fail_test "conflicting renderer evidence was not reported"

printf '%s\n' \
  'INFO | emuglConfig_init: vulkan_mode_selected:host gles_mode_selected:host vulkan_mode_selected:swiftshader gles_mode_selected:swangle' >"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == conflicting* ]] \
  || fail_test "same-line conflicting renderer evidence was not reported"

for _ in {1..16}; do
  printf 'INFO | emuglConfig_init: vulkan_mode_selected:swiftshader gles_mode_selected:swiftshader\n'
done >"$log"
printf 'INFO | emuglConfig_init: vulkan_mode_selected:host gles_mode_selected:host\n' >>"$log"
expect_reject

printf 'INFO | emuglConfig_init: vulkan_mode_selected:host gles_mode_selected:host\n' >"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == unapproved* ]] \
  || fail_test "unapproved renderer evidence was not reported"

printf '%s\n' \
  'INFO | -gpu swiftshader_indirect' \
  'INFO | emuglConfig_init: vulkan_mode_selected:swiftshader gles_mode_selected:swiftshader' >"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == legacy* ]] \
  || fail_test "legacy renderer evidence was not reported"

printf 'INFO | -gpu swiftshader_indirect\n' >"$log"
dd if=/dev/zero bs=1024 count=256 2>/dev/null | tr '\0' x >>"$log"
printf '\nINFO | emuglConfig_init: vulkan_mode_selected:swiftshader gles_mode_selected:swiftshader\n' >>"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == legacy* ]] \
  || fail_test "legacy renderer evidence in a large log was hidden by pipe status"

ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES=1024
dd if=/dev/zero bs=1024 count=1 2>/dev/null | tr '\0' x >"$log"
printf '\nINFO | emuglConfig_init: vulkan_mode_selected:swiftshader gles_mode_selected:swiftshader\n' >>"$log"
expect_reject
[[ "$ANDROID_EMULATOR_RENDERER_LAST_EVIDENCE" == renderer\ selection\ is\ missing* ]] \
  || fail_test "selection beyond bounded log prefix was accepted"

if android_emulator_renderer_verify "$temporary/missing.log"; then
  fail_test "missing renderer log should fail closed"
fi

echo "android-emulator-renderer.test: ok"
