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

echo 'android-apk-signer-test: ok'
