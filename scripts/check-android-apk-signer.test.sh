#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
temp="$(mktemp -d)"
trap 'rm -rf -- "$temp"' EXIT

apk="$temp/candidate.apk"
mock="$temp/apksigner"
printf 'apk' >"$apk"
cat >"$mock" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'Signer #1 certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
MOCK
chmod +x "$mock"

fingerprint="$(printf 'ab%.0s' {1..32})"
DEVE_ANDROID_APKSIGNER="$mock" MOCK_SIGNER="$fingerprint" \
  bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" "$apk" "sha256:$fingerprint"

if DEVE_ANDROID_APKSIGNER="$mock" MOCK_SIGNER="$(printf 'cd%.0s' {1..32})" \
  bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" "$apk" "sha256:$fingerprint" >/dev/null 2>&1; then
  echo 'android-apk-signer-test: mismatched signer unexpectedly passed' >&2
  exit 1
fi

cat >"$mock" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'Signer #1 certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
printf 'Signer #2 certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
MOCK
if DEVE_ANDROID_APKSIGNER="$mock" MOCK_SIGNER="$fingerprint" \
  bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" "$apk" "sha256:$fingerprint" >/dev/null 2>&1; then
  echo 'android-apk-signer-test: multiple signers unexpectedly passed' >&2
  exit 1
fi

cat >"$mock" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'Verified using v2 scheme (APK Signature Scheme v2): true\n'
printf 'Verified using v3 scheme (APK Signature Scheme v3): true\n'
printf 'Number of signers: 1\n'
printf 'V2.0 Signer: certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
printf 'V3.0 Signer: certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
MOCK
observed="$(DEVE_ANDROID_APKSIGNER="$mock" MOCK_SIGNER="${fingerprint^^}" \
  bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" --print-fingerprint "$apk")"
[[ "$observed" == "sha256:$fingerprint" ]] || {
  printf 'android-apk-signer-test: current V3 output returned %s\n' "$observed" >&2
  exit 1
}

cat >"$mock" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'Number of signers: 2\n'
printf 'V3.0 Signer: certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
MOCK
if DEVE_ANDROID_APKSIGNER="$mock" MOCK_SIGNER="$fingerprint" \
  bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" --print-fingerprint "$apk" >/dev/null 2>&1; then
  echo 'android-apk-signer-test: declared multiple signers unexpectedly passed' >&2
  exit 1
fi

cat >"$mock" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'Number of signers: 1\n'
printf 'V2.0 Signer: certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
printf 'V3.0 Signer: certificate SHA-256 digest: %s\n' "${MOCK_OTHER_SIGNER:?}"
MOCK
if DEVE_ANDROID_APKSIGNER="$mock" MOCK_SIGNER="$fingerprint" \
  MOCK_OTHER_SIGNER="$(printf 'cd%.0s' {1..32})" \
  bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" --print-fingerprint "$apk" >/dev/null 2>&1; then
  echo 'android-apk-signer-test: conflicting signer digests unexpectedly passed' >&2
  exit 1
fi

cat >"$mock" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'Number of signers: 1\n'
printf 'Number of signers: 1\n'
printf 'V3.0 Signer: certificate SHA-256 digest: %s\n' "${MOCK_SIGNER:?}"
MOCK
if DEVE_ANDROID_APKSIGNER="$mock" MOCK_SIGNER="$fingerprint" \
  bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" --print-fingerprint "$apk" >/dev/null 2>&1; then
  echo 'android-apk-signer-test: conflicting signer count lines unexpectedly passed' >&2
  exit 1
fi

echo 'android-apk-signer-test: ok'
