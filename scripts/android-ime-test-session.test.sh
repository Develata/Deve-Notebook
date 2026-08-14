#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-ime-test-session.sh"

TEST_ROOT="$(mktemp -d)"
trap 'rm -rf -- "$TEST_ROOT"' EXIT
STATE_FILE="$TEST_ROOT/default-ime"
ORIGINAL_IME="com.example.original/.OriginalIme"
TEST_IME="com.example.test/.TestIme"

fail_test() {
  echo "android-ime-test-session.test: $*" >&2
  exit 1
}

reset_fixture() {
  printf '%s\n' "$ORIGINAL_IME" >"$STATE_FILE"
  FAKE_DROP_TEST_SET_RESPONSE=0
  FAKE_BLOCK_ORIGINAL_RESTORE=0
  ANDROID_IME_TEST_ORIGINAL_SERVICE=""
  ANDROID_IME_TEST_SELECTED_SERVICE=""
  ANDROID_IME_TEST_RESTORE_REQUIRED=0
}

fake_adb() {
  case "$*" in
    "shell settings get secure default_input_method")
      cat "$STATE_FILE"
      ;;
    "shell ime list -s")
      printf '%s\n%s\n' "$ORIGINAL_IME" "$TEST_IME"
      ;;
    "shell ime set $TEST_IME")
      printf '%s\n' "$TEST_IME" >"$STATE_FILE"
      [[ "$FAKE_DROP_TEST_SET_RESPONSE" == "0" ]]
      ;;
    "shell ime set $ORIGINAL_IME")
      if [[ "$FAKE_BLOCK_ORIGINAL_RESTORE" == "1" ]]; then
        return 1
      fi
      printf '%s\n' "$ORIGINAL_IME" >"$STATE_FILE"
      ;;
    *)
      fail_test "unexpected fake adb command: $*"
      ;;
  esac
}

reset_fixture
FAKE_DROP_TEST_SET_RESPONSE=1
if android_ime_test_begin fake_adb "$TEST_IME"; then
  fail_test "lost set response must fail the begin operation"
fi
[[ "$(cat "$STATE_FILE")" == "$TEST_IME" ]] || fail_test "test IME mutation was not committed"
[[ "$ANDROID_IME_TEST_RESTORE_REQUIRED" == "1" ]] || fail_test "ambiguous mutation did not require restore"
FAKE_DROP_TEST_SET_RESPONSE=0
android_ime_test_restore fake_adb
[[ "$(cat "$STATE_FILE")" == "$ORIGINAL_IME" ]] || fail_test "ambiguous mutation was not restored"

reset_fixture
android_ime_test_begin fake_adb "$TEST_IME"
FAKE_BLOCK_ORIGINAL_RESTORE=1
if android_ime_test_restore fake_adb; then
  fail_test "restore command failure must remain blocking"
fi
[[ "$ANDROID_IME_TEST_RESTORE_REQUIRED" == "1" ]] || fail_test "failed restore retired recovery ownership"

FAKE_BLOCK_ORIGINAL_RESTORE=0
android_ime_test_restore fake_adb
[[ "$ANDROID_IME_TEST_RESTORE_REQUIRED" == "0" ]] || fail_test "verified restore did not retire recovery ownership"

echo "android-ime-test-session.test: ok"
