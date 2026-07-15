#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-emulator-owner.sh"

fail() {
  echo "android-emulator-cleanup-test: $*" >&2
  exit 1
}

write_owner() {
  local path="$1"
  local pid="$2"
  printf 'launch_state=launched\nemulator_pid=%s\nemulator_serial=emulator-9998\navd_name=deve-owner-fixture\n' \
    "$pid" >"$path"
}

verify_unproven_pid_is_not_signaled() {
  local fixture expected_owner fake_sdk
  fixture="$(mktemp -d)"
  expected_owner="$fixture/android-emulator-owner.txt"
  fake_sdk="$fixture/sdk"
  mkdir -p "$fake_sdk/platform-tools"
  printf '#!/usr/bin/env bash\nprintf "List of devices attached\\n"\n' \
    >"$fake_sdk/platform-tools/adb"
  chmod +x "$fake_sdk/platform-tools/adb"

  if DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$fixture" \
      DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE="$fixture/escape/owner.txt" \
      android_emulator_owner_file "$fixture" >/dev/null 2>&1; then
    rm -rf -- "$fixture"
    fail "runner-owned state accepted an escaping owner path"
  fi

  write_owner "$expected_owner" "$$"
  if DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$fixture" \
      DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE="$expected_owner" \
      DEVE_MOBILE_ANDROID_EMULATOR_CLEANUP_TIMEOUT_SECS=1 \
      ANDROID_HOME="$fake_sdk" ANDROID_SDK_ROOT="$fake_sdk" \
      bash "$ROOT_DIR/scripts/cleanup-mobile-android-emulator.sh" >/dev/null 2>&1; then
    rm -rf -- "$fixture"
    fail "cleanup accepted a live PID without a verified emulator serial"
  fi
  kill -0 "$$" >/dev/null 2>&1 \
    || fail "cleanup signaled a live PID without verified emulator ownership"
  [[ -e "$expected_owner" ]] || fail "failed cleanup discarded owner state"
  rm -rf -- "$fixture"
}

verify_adb_probe_failure_retains_owner() {
  local fixture expected_owner fake_sdk
  fixture="$(mktemp -d)"
  expected_owner="$fixture/android-emulator-owner.txt"
  fake_sdk="$fixture/sdk"
  mkdir -p "$fake_sdk/platform-tools"
  printf '#!/usr/bin/env bash\nexit 3\n' >"$fake_sdk/platform-tools/adb"
  chmod +x "$fake_sdk/platform-tools/adb"
  write_owner "$expected_owner" "999999"

  if DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$fixture" \
      DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE="$expected_owner" \
      DEVE_MOBILE_ANDROID_EMULATOR_CLEANUP_TIMEOUT_SECS=1 \
      ANDROID_HOME="$fake_sdk" ANDROID_SDK_ROOT="$fake_sdk" \
      bash "$ROOT_DIR/scripts/cleanup-mobile-android-emulator.sh" >/dev/null 2>&1; then
    rm -rf -- "$fixture"
    fail "failed ADB probe was treated as confirmed serial absence"
  fi
  [[ -e "$expected_owner" ]] || fail "ADB probe failure discarded owner state"
  rm -rf -- "$fixture"
}

verify_reserved_owner_cannot_terminate() {
  local fixture expected_owner fake_sdk kill_marker
  fixture="$(mktemp -d)"
  expected_owner="$fixture/android-emulator-owner.txt"
  fake_sdk="$fixture/sdk"
  kill_marker="$fixture/kill-requested"
  mkdir -p "$fake_sdk/platform-tools"
  cat >"$fake_sdk/platform-tools/adb" <<'FAKE_ADB'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  devices) printf 'List of devices attached\nemulator-9998\tdevice\n' ;;
  '-s emulator-9998 emu avd name') printf 'deve-owner-fixture\nOK\n' ;;
  '-s emulator-9998 emu kill') touch "$DEVE_FAKE_KILL_MARKER" ;;
  *) exit 2 ;;
esac
FAKE_ADB
  chmod +x "$fake_sdk/platform-tools/adb"
  printf 'launch_state=reserved\nemulator_pid=\nemulator_serial=emulator-9998\navd_name=deve-owner-fixture\n' \
    >"$expected_owner"

  if DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$fixture" \
      DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE="$expected_owner" \
      DEVE_MOBILE_ANDROID_EMULATOR_CLEANUP_TIMEOUT_SECS=1 \
      DEVE_FAKE_KILL_MARKER="$kill_marker" \
      ANDROID_HOME="$fake_sdk" ANDROID_SDK_ROOT="$fake_sdk" \
      bash "$ROOT_DIR/scripts/cleanup-mobile-android-emulator.sh" >/dev/null 2>&1; then
    rm -rf -- "$fixture"
    fail "reserved owner acquired termination authority"
  fi
  [[ ! -e "$kill_marker" ]] || fail "reserved owner requested emulator termination"
  [[ -e "$expected_owner" ]] || fail "reserved owner state was discarded"
  rm -rf -- "$fixture"
}

verify_shutdown_transition_is_idempotent() {
  local fixture expected_owner fake_sdk state_file child_pid
  fixture="$(mktemp -d)"
  expected_owner="$fixture/android-emulator-owner.txt"
  fake_sdk="$fixture/sdk"
  state_file="$fixture/state"
  mkdir -p "$fake_sdk/platform-tools"
  printf '0\n' >"$state_file"
  cat >"$fake_sdk/platform-tools/adb" <<'FAKE_ADB'
#!/usr/bin/env bash
set -euo pipefail
state="$(cat "$DEVE_FAKE_ADB_STATE")"
case "$*" in
  devices)
    printf 'List of devices attached\n'
    if [[ "$state" == "0" ]]; then
      printf 'emulator-9998\tdevice\n'
    elif [[ "$state" == "1" ]]; then
      printf 'emulator-9998\tdevice\n'
      printf '2\n' >"$DEVE_FAKE_ADB_STATE"
    fi
    ;;
  '-s emulator-9998 emu avd name')
    [[ "$state" == "0" ]] && printf 'deve-owner-fixture\nOK\n'
    ;;
  '-s emulator-9998 emu kill')
    printf '1\n' >"$DEVE_FAKE_ADB_STATE"
    kill "$DEVE_FAKE_EMULATOR_PID" >/dev/null 2>&1 || true
    ;;
  *) exit 2 ;;
esac
FAKE_ADB
  chmod +x "$fake_sdk/platform-tools/adb"

  sleep 30 &
  child_pid="$!"
  write_owner "$expected_owner" "$child_pid"
  if ! DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$fixture" \
      DEVE_MOBILE_ANDROID_EMULATOR_OWNER_FILE="$expected_owner" \
      DEVE_MOBILE_ANDROID_EMULATOR_CLEANUP_TIMEOUT_SECS=5 \
      DEVE_FAKE_ADB_STATE="$state_file" DEVE_FAKE_EMULATOR_PID="$child_pid" \
      ANDROID_HOME="$fake_sdk" ANDROID_SDK_ROOT="$fake_sdk" \
      bash "$ROOT_DIR/scripts/cleanup-mobile-android-emulator.sh" >/dev/null; then
    kill "$child_pid" >/dev/null 2>&1 || true
    wait "$child_pid" >/dev/null 2>&1 || true
    rm -rf -- "$fixture"
    fail "verified shutdown transition did not converge"
  fi
  wait "$child_pid" >/dev/null 2>&1 || true
  [[ ! -e "$expected_owner" ]] || fail "successful cleanup retained owner state"
  rm -rf -- "$fixture"
}

verify_unproven_pid_is_not_signaled
verify_adb_probe_failure_retains_owner
verify_reserved_owner_cannot_terminate
verify_shutdown_transition_is_idempotent
echo "android-emulator-cleanup-test: ok"
