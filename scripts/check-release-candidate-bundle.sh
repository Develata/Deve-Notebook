#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

fail() {
  printf 'release-candidate-bundle: %s\n' "$*" >&2
  exit 1
}

required=(
  DEVE_RELEASE_CANDIDATE_DIR
  DEVE_RELEASE_CANDIDATE_HEAD
  DEVE_RELEASE_CANDIDATE_VERSION
  DEVE_RELEASE_CANDIDATE_WORKFLOW_PATH
  DEVE_RELEASE_CANDIDATE_RUN_ID
  DEVE_RELEASE_CANDIDATE_RUN_ATTEMPT
  DEVE_RELEASE_CANDIDATE_IMAGE_ID
  DEVE_RELEASE_ANDROID_SIGNER_SHA256
  DEVE_RELEASE_WINDOWS_MSI
  DEVE_RELEASE_WINDOWS_NSIS
  DEVE_RELEASE_MACOS_DMG
  DEVE_RELEASE_ANDROID_APK
  DEVE_RELEASE_DOCKER_ARCHIVE
  DEVE_RELEASE_SOURCE_SBOM
  DEVE_RELEASE_IMAGE_SBOM
  DEVE_RELEASE_PROVENANCE_BUNDLE
  DEVE_RELEASE_DOCKER_SBOM_BUNDLE
)
for name in "${required[@]}"; do
  [[ -n "${!name:-}" ]] || fail "missing required environment $name"
done

baseline="${DEVE_BASELINE_BIN:-}"
if [[ -z "$baseline" ]]; then
  baseline="$ROOT_DIR/target/debug/deve_baseline"
  [[ "${OS:-}" != "Windows_NT" ]] || baseline="$baseline.exe"
fi
[[ -x "$baseline" || -f "$baseline" ]] || fail "baseline executable is unavailable: $baseline"

args=(
  release-candidate verify
  --candidate-dir "$DEVE_RELEASE_CANDIDATE_DIR"
  --output release-candidate.json
  --head "$DEVE_RELEASE_CANDIDATE_HEAD"
  --version "$DEVE_RELEASE_CANDIDATE_VERSION"
  --workflow-path "$DEVE_RELEASE_CANDIDATE_WORKFLOW_PATH"
  --run-id "$DEVE_RELEASE_CANDIDATE_RUN_ID"
  --run-attempt "$DEVE_RELEASE_CANDIDATE_RUN_ATTEMPT"
  --docker-image-id "$DEVE_RELEASE_CANDIDATE_IMAGE_ID"
  --android-signer-sha256 "$DEVE_RELEASE_ANDROID_SIGNER_SHA256"
  --windows-msi "$DEVE_RELEASE_WINDOWS_MSI"
  --windows-nsis "$DEVE_RELEASE_WINDOWS_NSIS"
  --macos-dmg "$DEVE_RELEASE_MACOS_DMG"
  --android-apk "$DEVE_RELEASE_ANDROID_APK"
  --docker-archive "$DEVE_RELEASE_DOCKER_ARCHIVE"
  --source-sbom "$DEVE_RELEASE_SOURCE_SBOM"
  --image-sbom "$DEVE_RELEASE_IMAGE_SBOM"
  --provenance-bundle "$DEVE_RELEASE_PROVENANCE_BUNDLE"
  --docker-sbom-bundle "$DEVE_RELEASE_DOCKER_SBOM_BUNDLE"
)
"$baseline" "${args[@]}"

bash "$ROOT_DIR/scripts/check-android-apk-signer.sh" \
  "$DEVE_RELEASE_CANDIDATE_DIR/$DEVE_RELEASE_ANDROID_APK" \
  "$DEVE_RELEASE_ANDROID_SIGNER_SHA256"

if [[ "${DEVE_RELEASE_ATTESTATION_VERIFY_REQUIRED:-0}" == "1" ]]; then
  command -v gh >/dev/null 2>&1 || fail "gh is required for attestation verification"
  [[ -n "${GITHUB_REPOSITORY:-}" ]] || fail "GITHUB_REPOSITORY is required for attestation verification"
  signer_workflow="$GITHUB_REPOSITORY/.github/workflows/release-candidate.yml"
  provenance="$DEVE_RELEASE_CANDIDATE_DIR/$DEVE_RELEASE_PROVENANCE_BUNDLE"
  docker_sbom="$DEVE_RELEASE_CANDIDATE_DIR/$DEVE_RELEASE_DOCKER_SBOM_BUNDLE"
  for relative in \
    "$DEVE_RELEASE_WINDOWS_MSI" \
    "$DEVE_RELEASE_WINDOWS_NSIS" \
    "$DEVE_RELEASE_MACOS_DMG" \
    "$DEVE_RELEASE_ANDROID_APK" \
    "$DEVE_RELEASE_DOCKER_ARCHIVE" \
    "$DEVE_RELEASE_SOURCE_SBOM" \
    "$DEVE_RELEASE_IMAGE_SBOM"; do
    gh attestation verify "$DEVE_RELEASE_CANDIDATE_DIR/$relative" \
      --repo "$GITHUB_REPOSITORY" \
      --bundle "$provenance" \
      --signer-workflow "$signer_workflow" \
      --source-digest "$DEVE_RELEASE_CANDIDATE_HEAD" >/dev/null
  done
  gh attestation verify "$DEVE_RELEASE_CANDIDATE_DIR/$DEVE_RELEASE_DOCKER_ARCHIVE" \
    --repo "$GITHUB_REPOSITORY" \
    --bundle "$docker_sbom" \
    --predicate-type https://spdx.dev/Document/v2.3 \
    --signer-workflow "$signer_workflow" \
    --source-digest "$DEVE_RELEASE_CANDIDATE_HEAD" >/dev/null
fi

printf 'release-candidate-bundle: ok\n'
