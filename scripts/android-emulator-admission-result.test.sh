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
RESULT_PATH="$RESULT_DIR/pinned-api37-direct-memory-shared-slots.json"
CYCLE_RESULT_DIR="$RESULT_DIR/cycle-results"
EXPECTED_HEAD="0123456789abcdef0123456789abcdef01234567"
VARIANT_ID="pinned-api37-direct-memory-shared-slots"
EMULATOR_SOURCE="pinned"
GPU_MODE="swangle"
FEATURE_POLICY="direct-memory-shared-slots"
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
      rendererPair: "swiftshader swangle",
      featurePair: "1/1"
    }' >"$CYCLE_RESULT_DIR/cycle-$cycle.json"
done

android_admission_write_summary_result true true ""
jq -e \
  '.schemaVersion == 2
    and .kind == "android-emulator-admission-diagnostic"
    and .complete == true
    and .stable == true
    and .harnessError == null
    and .variantId == "pinned-api37-direct-memory-shared-slots"
    and .gpuMode == "swangle"
    and .featurePolicy == "direct-memory-shared-slots"
    and ([.cycles[].rendererPair] | unique) == ["swiftshader swangle"]
    and ([.cycles[].featurePair] | unique) == ["1/1"]
    and (.cycles | length == 3)' \
  "$RESULT_PATH" >/dev/null \
  || fail "atomic summary result did not preserve the expected schema"
compgen -G "$RESULT_PATH.tmp.*" >/dev/null \
  && fail "temporary result file survived atomic publication"

rm -f -- "$CYCLE_RESULT_DIR/cycle-1.json"
cp -- "$CYCLE_RESULT_DIR/cycle-2.json" "$CYCLE_RESULT_DIR/cycle-duplicate.json"
android_admission_write_summary_result true false ""
jq -e \
  '.complete == false
    and .stable == false
    and .harnessError == "cycle result set is incomplete: expected exact cycles 1..3"
    and ([.cycles[].cycle] == [2, 2, 3])' \
  "$RESULT_PATH" >/dev/null \
  || fail "incomplete cycle set was not published as a fail-closed harness error"
compgen -G "$RESULT_PATH.tmp.*" >/dev/null \
  && fail "incomplete result left a temporary file behind"

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
[[ "$(android_admission_classify_cycle_failure "$failure_log" feature-admission)" \
    == "feature_identity" ]] \
  || fail "feature-identity classification drifted"

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
run_cycle_source="$(sed -n '/^run_cycle() (/,/^main() {/p' "$worker")"
run_cycle_context="$(sed -n '/^run_cycle() (/,/^  mkdir -p /p' "$worker")"
grep -Fq 'run_cycle() (' "$worker" \
  || fail "worker cycle is no longer isolated in a subshell"
grep -Fq '  cycle="$1"' <<<"$run_cycle_context" \
  || fail "worker finalizer context is not retained at subshell scope"
grep -Eq '^[[:space:]]*(local|declare|typeset)([[:space:]]|$)' <<<"$run_cycle_source" \
  && fail "function-local context disappears before an implicit-errexit EXIT trap"
(( $(grep -Fc '  trap cycle_finish EXIT' <<<"$run_cycle_source") == 1 )) \
  || fail "worker must register exactly one cycle EXIT finalizer"
grep -Fq "  trap 'exit 130' INT" <<<"$run_cycle_source" \
  || fail "worker no longer maps cycle interruption to exit 130"
grep -Fq "  trap 'exit 143' TERM" <<<"$run_cycle_source" \
  || fail "worker no longer maps cycle termination to exit 143"
grep -Fq '    trap - EXIT INT TERM' <<<"$run_cycle_source" \
  || fail "worker finalizer no longer prevents signal-trap reentry"
grep -Fq 'run_cycle "$cycle" >"$RESULT_DIR/cycle-$cycle/cycle.log" 2>&1' "$worker" \
  || fail "worker no longer invokes a direct cycle command"
grep -Fq 'cycle_status=$?' "$worker" \
  || fail "worker no longer captures the direct cycle status"
grep -Fq 'if run_cycle ' "$worker" \
  && fail "conditional function invocation would suppress cycle errexit"
grep -Fq -- '-gpu "$GPU_MODE"' "$worker" \
  || fail "worker no longer passes the admitted renderer mode to the emulator"
grep -Fq '"${ANDROID_EMULATOR_FEATURE_ARGS[@]}"' "$worker" \
  || fail "worker no longer passes the admitted gfxstream feature policy"
grep -Fq 'android_emulator_renderer_wait \' "$worker" \
  || fail "worker no longer proves the actual renderer before boot admission"
grep -Fq 'android_emulator_feature_policy_observe' "$worker" \
  || fail "worker no longer proves the actual gfxstream feature state"
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

finalizer_contract_probe() (
  set -euo pipefail
  probe_mode="$1"
  probe_result="$2"
  probe_owner="$3"
  probe_child_file="$4"
  probe_context="retained-after-errexit"
  probe_child_pid=""
  probe_cleanup_status=0
  probe_temporary="$probe_result.tmp.$$"
  probe_finish() {
    probe_status=$?
    trap - EXIT INT TERM
    android_admission_cleanup_emulator \
      "$probe_owner" "$(dirname "$probe_result")" "$probe_child_pid" \
      || probe_cleanup_status=$?
    jq -n \
      --argjson exitStatus "$probe_status" \
      --argjson cleanupStatus "$probe_cleanup_status" \
      --arg context "$probe_context" \
      '{exitStatus: $exitStatus, cleanupStatus: $cleanupStatus, context: $context}' \
      >"$probe_temporary"
    mv -f -- "$probe_temporary" "$probe_result"
    exit "$probe_status"
  }
  trap probe_finish EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  android_admission_write_emulator_owner "$probe_owner"
  sleep 300 &
  probe_child_pid="$!"
  printf '%s\n' "$probe_child_pid" >"$probe_child_file"
  android_admission_write_emulator_owner "$probe_owner" "$probe_child_pid"
  case "$probe_mode" in
    failure) bash -c 'exit 42' ;;
    int) kill -INT "$BASHPID" ;;
    term) kill -TERM "$BASHPID" ;;
    *) exit 99 ;;
  esac
)

run_finalizer_contract_probe() {
  local mode="$1"
  local expected_status="$2"
  local result="$fixture/finalizer-$mode.json"
  local owner="$fixture/finalizer-$mode.owner"
  local child_file="$fixture/finalizer-$mode.child"
  local child_pid status
  set +e
  finalizer_contract_probe "$mode" "$result" "$owner" "$child_file"
  status=$?
  set -e
  (( status == expected_status )) \
    || fail "$mode finalizer replaced status $expected_status with $status"
  jq -e \
    --argjson expected "$expected_status" \
    '.exitStatus == $expected and .context == "retained-after-errexit"' \
    "$result" >/dev/null \
    || fail "$mode finalizer did not atomically publish its exact status and context"
  compgen -G "$result.tmp.*" >/dev/null \
    && fail "$mode finalizer left a temporary result behind"
  child_pid="$(cat "$child_file")"
  ! kill -0 "$child_pid" >/dev/null 2>&1 \
    || fail "$mode finalizer left its direct child running"
}

run_finalizer_contract_probe failure 42
run_finalizer_contract_probe int 130
run_finalizer_contract_probe term 143

echo "android-emulator-admission-result-test: ok"
