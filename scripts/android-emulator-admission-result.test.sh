#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_ROOT="$ROOT_DIR"
source "$ROOT_DIR/scripts/lib/android-admission-diagnostic-result.sh"
source "$ROOT_DIR/scripts/lib/android-admission-emulator-lifecycle.sh"

fail() {
  echo "android-emulator-admission-result-test: $*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is required"
fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT

RESULT_DIR="$fixture/result"
RESULT_PATH="$RESULT_DIR/pinned-api37-software.json"
CYCLE_RESULT_DIR="$RESULT_DIR/cycle-results"
EXPECTED_HEAD="0123456789abcdef0123456789abcdef01234567"
VARIANT_ID="pinned-api37-software"
EMULATOR_SOURCE="pinned"
GPU_MODE="software"
EMULATOR_VERSION="36.7.0.0"
EMULATOR_BUILD_ID="15600000"
EMULATOR_PROBE_STATUS="0"
SDK_EMULATOR_REVISION="36.7.0"
API_LEVEL="37.0"
SYSTEM_TARGET="google_apis"
SYSTEM_IMAGE_REVISION="1"
ARCHITECTURE="x86_64"
APK_SHA256="$(printf apk | sha256sum | awk '{print $1}')"
REQUESTED_CYCLES=3
mkdir -p "$CYCLE_RESULT_DIR"

for cycle in 1 2 3; do
  jq -n \
    --argjson cycle "$cycle" \
    '{
      cycle: $cycle,
      outcome: "passed",
      phase: "complete",
      exitStatus: 0,
      cleanupStatus: 0,
      failureClass: null,
      systemServerPidBefore: "123",
      systemServerPidAfter: "123",
      rendererPair: "swiftshader swiftshader"
    }' >"$CYCLE_RESULT_DIR/cycle-$cycle.json"
done

android_admission_write_summary_result true true ""
jq -e \
  '.schemaVersion == 1
    and .kind == "android-emulator-admission-diagnostic"
    and .complete == true
    and .stable == true
    and .harnessError == null
    and .variantId == "pinned-api37-software"
    and .gpuMode == "software"
    and ([.cycles[].rendererPair] | unique) == ["swiftshader swiftshader"]
    and (.cycles | length == 3)' \
  "$RESULT_PATH" >/dev/null \
  || fail "atomic summary result did not preserve the expected schema"
compgen -G "$RESULT_PATH.tmp.*" >/dev/null \
  && fail "temporary result file survived atomic publication"

failure_log="$fixture/failure.log"
printf '%s\n' \
  'adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)' \
  >"$failure_log"
[[ "$(android_admission_classify_cycle_failure "$failure_log" install)" == "binder_epipe" ]] \
  || fail "Binder EPIPE classification drifted"
printf '%s\n' "cmd: Can't find service: settings" >"$failure_log"
[[ "$(android_admission_classify_cycle_failure "$failure_log" boot-admission)" \
    == "settings_service_unavailable" ]] \
  || fail "settings-service classification drifted"
printf '%s\n' "unrecognized failure" >"$failure_log"
[[ "$(android_admission_classify_cycle_failure "$failure_log" post-install-admission)" \
    == "post_install_instability" ]] \
  || fail "post-install classification drifted"
[[ "$(android_admission_classify_cycle_failure "$failure_log" renderer-admission)" \
    == "renderer_identity" ]] \
  || fail "renderer-identity classification drifted"

oversized_log="$fixture/oversized.log"
head -c $((ANDROID_ADMISSION_LOG_FILE_BUDGET_BYTES + 4096)) /dev/zero \
  | tr '\0' x >"$oversized_log"
android_admission_bound_log_file "$oversized_log"
(( $(wc -c <"$oversized_log") <= ANDROID_ADMISSION_LOG_FILE_BUDGET_BYTES )) \
  || fail "bounded log exceeded its fixed file budget"
grep -Fq -- 'deve diagnostic log truncated' "$oversized_log" \
  || fail "bounded log did not record truncation"
oversized_variant="$fixture/oversized-variant"
mkdir -p "$oversized_variant"
head -c $((ANDROID_ADMISSION_VARIANT_LOG_BUDGET_BYTES + 1)) /dev/zero \
  >"$oversized_variant/combined.log"
android_admission_verify_variant_log_budget "$oversized_variant" \
  && fail "variant output above the fixed aggregate budget was accepted"

ROOT_DIR="$fixture"
EMULATOR_SERIAL="emulator-5584"
AVD_NAME="deve-admission-test"
owner_file="$fixture/owner"
cleanup_script="$fixture/noop-cleanup.sh"
printf '#!/usr/bin/env bash\nexit 0\n' >"$cleanup_script"
android_admission_write_emulator_owner "$owner_file"
sleep 300 &
emulator_pid="$!"
ANDROID_ADMISSION_CLEANUP_SCRIPT="$cleanup_script"
if android_admission_cleanup_emulator "$owner_file" "$fixture" "$emulator_pid"; then
  fail "reserved-owner cleanup unexpectedly hid the required direct-child fallback"
fi
android_admission_direct_child_alive "$emulator_pid" \
  && fail "direct-child fallback left the emulator process running"

worker="$WORKSPACE_ROOT/scripts/diagnose-android-emulator-admission.sh"
grep -Fq 'run_cycle "$cycle" >"$RESULT_DIR/cycle-$cycle/cycle.log" 2>&1' "$worker" \
  || fail "worker no longer invokes a direct cycle command"
grep -Fq 'cycle_status=$?' "$worker" \
  || fail "worker no longer captures the direct cycle status"
grep -Fq 'if run_cycle ' "$worker" \
  && fail "conditional function invocation would suppress cycle errexit"
grep -Fq -- '-gpu "$GPU_MODE"' "$worker" \
  || fail "worker no longer passes the admitted renderer mode to the emulator"
grep -Fq 'android_emulator_renderer_wait \' "$worker" \
  || fail "worker no longer proves the actual renderer before boot admission"
(( $(grep -Fc 'android_emulator_renderer_observe "$cycle_dir/emulator.log"' "$worker") == 1 )) \
  || fail "worker no longer revalidates the complete renderer log during finalization"

errexit_marker="$fixture/errexit-was-suppressed"
failing_cycle_probe() (
  set -euo pipefail
  false
  printf 'unreachable\n' >"$errexit_marker"
)
set +e
failing_cycle_probe
probe_status=$?
set -e
(( probe_status != 0 )) || fail "direct cycle invocation hid a failing command"
[[ ! -e "$errexit_marker" ]] || fail "direct cycle invocation suppressed subshell errexit"

echo "android-emulator-admission-result-test: ok"
