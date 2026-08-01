#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-emulator-feature-policy.sh"

fail() {
  echo "android-emulator-feature-policy-test: $*" >&2
  exit 1
}

fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
log_file="$fixture/emulator.log"

write_log() {
  local direct="$1"
  local shared="$2"
  cat >"$log_file" <<EOF
DEBUG | gfxstreamFeature:HasSharedSlotsHostMemoryAllocator = $shared
DEBUG | gfxstreamFeature:GlDirectMem = $direct
EOF
}

always_alive() { return 0; }
already_dead() { return 1; }

android_emulator_feature_policy_configure default
[[ "${#ANDROID_EMULATOR_FEATURE_ARGS[@]}" == 0 \
  && "$ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR" == "0/0" ]] \
  || fail "default policy drifted"

android_emulator_feature_policy_configure direct-memory
[[ "${ANDROID_EMULATOR_FEATURE_ARGS[*]}" == "-feature GLDirectMem" \
  && "$ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR" == "1/0" ]] \
  || fail "direct-memory policy drifted"

android_emulator_feature_policy_configure direct-memory-shared-slots
[[ "${ANDROID_EMULATOR_FEATURE_ARGS[*]}" \
    == "-feature GLDirectMem -feature HasSharedSlotsHostMemoryAllocator" \
  && "$ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR" == "1/1" ]] \
  || fail "DMA feature conjunction drifted"

for case in "default 0 0" "direct-memory 1 0" "direct-memory-shared-slots 1 1"; do
  read -r policy direct shared <<<"$case"
  write_log "$direct" "$shared"
  android_emulator_feature_policy_observe "$log_file" "$policy" \
    || fail "$policy observation failed: $ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE"
  [[ "$ANDROID_EMULATOR_FEATURE_POLICY_LAST_PAIR" == "$direct/$shared" ]] \
    || fail "$policy observation pair drifted"
done

write_log 0 0
android_emulator_feature_policy_observe "$log_file" direct-memory \
  && fail "policy mismatch was accepted"
printf '%s\n' 'DEBUG | gfxstreamFeature:GlDirectMem = 1' >>"$log_file"
android_emulator_feature_policy_wait "$log_file" default 3 always_alive \
  && fail "conflicting feature state was accepted"
printf '%s\n' 'unrelated log' >"$log_file"
android_emulator_feature_policy_observe "$log_file" default \
  && fail "missing feature state was accepted"
android_emulator_feature_policy_wait "$log_file" default 3 already_dead \
  && fail "feature wait ignored an exited emulator process"
[[ "$ANDROID_EMULATOR_FEATURE_POLICY_LAST_EVIDENCE" \
    == "emulator process exited before feature observation" ]] \
  || fail "feature wait lost its process-guard evidence"
: >"$log_file"
(sleep 1; write_log 1 1) &
writer_pid="$!"
android_emulator_feature_policy_wait \
  "$log_file" direct-memory-shared-slots 3 always_alive \
  || fail "bounded feature wait missed progressive log output"
wait "$writer_pid"
android_emulator_feature_policy_configure unknown \
  && fail "unknown feature policy was accepted"

echo "android-emulator-feature-policy-test: ok"
