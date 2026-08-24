#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-prebuilt-apk.sh"

fail() {
  echo "android-prebuilt-apk-contract-test: $*" >&2
  exit 1
}

fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
release_apk="apps/mobile/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk"
debug_apk="apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
manifest="$fixture/android-apk-input.sha256"
mkdir -p "$(dirname "$fixture/$release_apk")" "$(dirname "$fixture/$debug_apk")"
printf 'release-apk\n' >"$fixture/$release_apk"
printf 'debug-apk\n' >"$fixture/$debug_apk"
(cd "$fixture" && sha256sum "$release_apk" "$debug_apk" >"$manifest")

android_prebuilt_apk_manifest_verify "$fixture" "$manifest" "$release_apk" "$debug_apk" \
  || fail "valid exact two-APK manifest was rejected"

# GNU sha256sum uses a binary marker on Windows and two spaces on Unix. Both
# encodings bind the same digest and path and must remain portable.
sed 's/  / */' "$manifest" >"$fixture/binary-mode.sha256"
android_prebuilt_apk_manifest_verify \
  "$fixture" "$fixture/binary-mode.sha256" "$release_apk" "$debug_apk" \
  || fail "valid binary-mode two-APK manifest was rejected"

printf 'tampered\n' >>"$fixture/$debug_apk"
if android_prebuilt_apk_manifest_verify "$fixture" "$manifest" "$release_apk" "$debug_apk"; then
  fail "tampered APK was accepted"
fi
printf 'debug-apk\n' >"$fixture/$debug_apk"

printf '%s\n' "$(head -n 1 "$manifest")" >"$fixture/missing.sha256"
if android_prebuilt_apk_manifest_verify "$fixture" "$fixture/missing.sha256" "$release_apk" "$debug_apk"; then
  fail "incomplete manifest was accepted"
fi

cp "$manifest" "$fixture/duplicate.sha256"
sed -i "2c$(head -n 1 "$manifest")" "$fixture/duplicate.sha256"
if android_prebuilt_apk_manifest_verify "$fixture" "$fixture/duplicate.sha256" "$release_apk" "$debug_apk"; then
  fail "duplicate manifest path was accepted"
fi

echo "android-prebuilt-apk-contract-test: ok"
