#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'android-apk-signer-check: %s\n' "$*" >&2
  exit 1
}

mode="check"
if [[ $# -eq 2 && "$1" == "--print-fingerprint" ]]; then
  mode="print"
  apk="$2"
  expected=""
elif [[ $# -eq 2 ]]; then
  apk="$1"
  expected="${2#sha256:}"
  expected="${expected//:/}"
  expected="${expected,,}"
else
  fail "usage: $0 <signed-apk> <sha256:fingerprint> | --print-fingerprint <signed-apk>"
fi
[[ -f "$apk" ]] || fail "APK is missing: $apk"
if [[ "$mode" == "check" ]]; then
  [[ "$expected" =~ ^[0-9a-f]{64}$ ]] || fail "expected fingerprint must be SHA-256"
fi

apksigner="${DEVE_ANDROID_APKSIGNER:-}"
if [[ -z "$apksigner" && -n "${ANDROID_HOME:-}" && -d "$ANDROID_HOME/build-tools" ]]; then
  apksigner="$(find "$ANDROID_HOME/build-tools" -mindepth 1 -maxdepth 2 -type f -name apksigner -print | sort -V | tail -n1)"
fi
[[ -n "$apksigner" && -x "$apksigner" ]] || fail "apksigner is unavailable"

output="$("$apksigner" verify --verbose --print-certs "$apk")" || fail "apksigner rejected the APK"
mapfile -t declared_signer_counts < <(
  sed -nE 's/^Number of signers: ([0-9]+)$/\1/p' <<<"$output"
)
[[ ${#declared_signer_counts[@]} -le 1 ]] || fail "apksigner returned conflicting signer counts"
if [[ ${#declared_signer_counts[@]} -eq 1 ]]; then
  [[ "${declared_signer_counts[0]}" == "1" ]] ||
    fail "expected exactly one APK signer, observed ${declared_signer_counts[0]}"
fi

# Build-tools before and after the V3 signer-label change use different
# prefixes for the same certificate digest. Keep the accepted forms explicit;
# unrelated output must never become signer identity.
mapfile -t signer_digests < <(
  {
    sed -nE 's/^Signer #[0-9]+ certificate SHA-256 digest: ([0-9A-Fa-f:]+)$/\1/p' <<<"$output"
    sed -nE 's/^V[0-9]+(\.[0-9]+)* Signer: certificate SHA-256 digest: ([0-9A-Fa-f:]+)$/\2/p' <<<"$output"
  } | tr '[:upper:]' '[:lower:]'
)
[[ ${#signer_digests[@]} -gt 0 ]] || fail "apksigner returned no signer certificate SHA-256 digest"

normalized_signers=()
for signer_digest in "${signer_digests[@]}"; do
  normalized="${signer_digest//:/}"
  [[ "$normalized" =~ ^[0-9a-f]{64}$ ]] || fail "apksigner returned an invalid SHA-256 digest"
  normalized_signers+=("$normalized")
done

if [[ ${#declared_signer_counts[@]} -eq 0 ]]; then
  [[ ${#normalized_signers[@]} -eq 1 ]] ||
    fail "expected exactly one APK signer, observed ${#normalized_signers[@]} signer digest lines without a signer count"
  observed="${normalized_signers[0]}"
else
  mapfile -t unique_signers < <(printf '%s\n' "${normalized_signers[@]}" | sort -u)
  [[ ${#unique_signers[@]} -eq 1 ]] ||
    fail "declared one APK signer but observed ${#unique_signers[@]} distinct certificate digests"
  observed="${unique_signers[0]}"
fi

if [[ "$mode" == "print" ]]; then
  printf 'sha256:%s\n' "$observed"
  exit 0
fi

[[ "$observed" == "$expected" ]] || fail "sealed APK signer certificate does not match the candidate identity"

printf 'android-apk-signer-check: ok\n'
