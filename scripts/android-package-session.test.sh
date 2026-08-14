#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-package-session.sh"

TEST_ROOT="$(mktemp -d)"
trap 'rm -rf -- "$TEST_ROOT"' EXIT
COMMAND_LOG="$TEST_ROOT/commands"
APP_ID="dev.deve.notebook.mobile"
FAKE_PACKAGE_INSTALLED=1
FAKE_PACKAGE_PROBE_STATUS=0
FAKE_CLEAR_STATUS=0
FAKE_CLEAR_OUTPUT="Success"
FAKE_UNINSTALL_STATUS=0
FAKE_UNINSTALL_OUTPUT="Success"

fail_test() {
  echo "android-package-session.test: $*" >&2
  exit 1
}

reset_fixture() {
  : >"$COMMAND_LOG"
  FAKE_PACKAGE_INSTALLED=1
  FAKE_PACKAGE_PROBE_STATUS=0
  FAKE_CLEAR_STATUS=0
  FAKE_CLEAR_OUTPUT="Success"
  FAKE_UNINSTALL_STATUS=0
  FAKE_UNINSTALL_OUTPUT="Success"
}

fake_adb() {
  printf '%s\n' "$*" >>"$COMMAND_LOG"
  case "$*" in
    "shell pm list packages --user 0 $APP_ID")
      (( FAKE_PACKAGE_PROBE_STATUS == 0 )) || return "$FAKE_PACKAGE_PROBE_STATUS"
      if [[ "$FAKE_PACKAGE_INSTALLED" == "1" ]]; then
        printf 'package:%s\r\n' "$APP_ID"
      fi
      ;;
    "shell pm clear $APP_ID")
      printf '%s\n' "$FAKE_CLEAR_OUTPUT"
      return "$FAKE_CLEAR_STATUS"
      ;;
    "uninstall $APP_ID")
      printf '%s\n' "$FAKE_UNINSTALL_OUTPUT"
      return "$FAKE_UNINSTALL_STATUS"
      ;;
    *)
      fail_test "unexpected fake adb command: $*"
      ;;
  esac
}

reset_fixture
android_package_session_prepare 0 fake_adb "$APP_ID"
[[ ! -s "$COMMAND_LOG" ]] || fail_test "formal mode unexpectedly probed an existing package"
android_package_session_cleanup 0 fake_adb "$APP_ID"
grep -Fx "uninstall $APP_ID" "$COMMAND_LOG" >/dev/null \
  || fail_test "formal mode did not preserve uninstall cleanup"

reset_fixture
FAKE_UNINSTALL_STATUS=7
if android_package_session_cleanup 0 fake_adb "$APP_ID" >/dev/null 2>"$TEST_ROOT/uninstall.err"; then
  fail_test "formal mode swallowed an uninstall failure"
fi
grep -F "formal package uninstall failed" "$TEST_ROOT/uninstall.err" >/dev/null \
  || fail_test "formal uninstall failure category drifted"

reset_fixture
FAKE_UNINSTALL_OUTPUT="secret-bearing-unexpected-response"
if android_package_session_cleanup 0 fake_adb "$APP_ID" >/dev/null 2>"$TEST_ROOT/uninstall-output.err"; then
  fail_test "formal mode admitted unexpected uninstall output"
fi
if grep -F "$FAKE_UNINSTALL_OUTPUT" "$TEST_ROOT/uninstall-output.err" >/dev/null; then
  fail_test "formal uninstall leaked raw command output"
fi

reset_fixture
android_package_session_prepare 1 fake_adb "$APP_ID"
grep -Fx "shell pm list packages --user 0 $APP_ID" "$COMMAND_LOG" >/dev/null \
  || fail_test "preserve mode did not prove the exact existing package"

reset_fixture
FAKE_PACKAGE_INSTALLED=0
if android_package_session_prepare 1 fake_adb "$APP_ID" 2>"$TEST_ROOT/missing.err"; then
  fail_test "preserve mode admitted a missing package"
fi
grep -F "requires the exact package" "$TEST_ROOT/missing.err" >/dev/null \
  || fail_test "missing-package failure category drifted"

reset_fixture
android_package_session_cleanup 1 fake_adb "$APP_ID" >"$TEST_ROOT/clear.out"
grep -Fx "android-package-session: preserved_package_cleared=true" "$TEST_ROOT/clear.out" >/dev/null \
  || fail_test "preserve cleanup did not emit its fixed success checkpoint"

reset_fixture
FAKE_CLEAR_OUTPUT="secret-bearing-unexpected-response"
if android_package_session_cleanup 1 fake_adb "$APP_ID" 2>"$TEST_ROOT/clear.err"; then
  fail_test "preserve cleanup admitted unexpected output"
fi
grep -F "preserved package data cleanup failed" "$TEST_ROOT/clear.err" >/dev/null \
  || fail_test "preserve cleanup failure category drifted"
if grep -F "$FAKE_CLEAR_OUTPUT" "$TEST_ROOT/clear.err" >/dev/null; then
  fail_test "preserve cleanup leaked raw command output"
fi

reset_fixture
FAKE_CLEAR_STATUS=7
if android_package_session_cleanup 1 fake_adb "$APP_ID" >/dev/null 2>&1; then
  fail_test "preserve cleanup swallowed an adb failure"
fi

if android_package_session_final_status 0 1; then
  fail_test "cleanup failure did not fail an otherwise successful journey"
else
  [[ "$?" == "1" ]] || fail_test "cleanup failure status was not propagated"
fi
if android_package_session_final_status 7 1; then
  fail_test "primary failure unexpectedly became success"
else
  [[ "$?" == "7" ]] || fail_test "cleanup failure overwrote the primary failure"
fi

if android_package_session_validate_mode unexpected >/dev/null 2>&1; then
  fail_test "invalid preserve mode was accepted"
fi
if android_package_session_validate_receipt_boundary 1 "/tmp/formal-claims.json" \
    >/dev/null 2>"$TEST_ROOT/receipt.err"; then
  fail_test "preserve mode entered the formal evidence channel"
fi
grep -F "preserve mode cannot write formal evidence" "$TEST_ROOT/receipt.err" >/dev/null \
  || fail_test "preserve/formal evidence failure category drifted"
android_package_session_validate_receipt_boundary 1 ""
android_package_session_validate_receipt_boundary 0 "/tmp/formal-claims.json"

echo "android-package-session.test: ok"
