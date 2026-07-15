#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'android-apk-signer-check: %s\n' "$*" >&2
  exit 1
}

[[ $# -eq 2 ]] || fail "usage: $0 <signed-apk> <sha256:fingerprint>"
apk="$1"
expected="${2#sha256:}"
expected="${expected//:/}"
expected="${expected,,}"
[[ -f "$apk" ]] || fail "APK is missing: $apk"
[[ "$expected" =~ ^[0-9a-f]{64}$ ]] || fail "expected fingerprint must be SHA-256"

apksigner="${DEVE_ANDROID_APKSIGNER:-}"
if [[ -z "$apksigner" && -n "${ANDROID_HOME:-}" && -d "$ANDROID_HOME/build-tools" ]]; then
  apksigner="$(find "$ANDROID_HOME/build-tools" -mindepth 1 -maxdepth 2 -type f -name apksigner -print | sort -V | tail -n1)"
fi
[[ -n "$apksigner" && -x "$apksigner" ]] || fail "apksigner is unavailable"

output="$("$apksigner" verify --verbose --print-certs "$apk")" || fail "apksigner rejected the APK"
mapfile -t signers < <(
  sed -n 's/^Signer #[0-9][0-9]* certificate SHA-256 digest: //p' <<<"$output" |
    tr '[:upper:]' '[:lower:]'
)
[[ ${#signers[@]} -eq 1 ]] || fail "expected exactly one APK signer, observed ${#signers[@]}"
observed="${signers[0]//:/}"
[[ "$observed" =~ ^[0-9a-f]{64}$ ]] || fail "apksigner returned an invalid SHA-256 digest"
[[ "$observed" == "$expected" ]] || fail "sealed APK signer certificate does not match the candidate identity"

printf 'android-apk-signer-check: ok\n'
