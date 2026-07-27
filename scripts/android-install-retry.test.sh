#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/check-mobile-android-install-startup-smoke.sh
source "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
readonly PRODUCTION_ADB_RETRY_TIMED="$(declare -f adb_retry_timed)"

temporary="$(mktemp -d)"
cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT

APK_PATH="$temporary/candidate.apk"
ADB_TIMEOUT_SECS=60
clock="$temporary/clock"
touch "$APK_PATH"

install_retry_now() {
  cat "$clock"
}

sleep() {
  :
}

run_case() {
  local mode="$1"
  local expected_status="$2"
  local expected_install_count="$3"
  local expected_operation_count="$4"
  local operations="$temporary/$mode.operations"
  local status install_count operation_count
  : >"$operations"
  printf '0\n' >"$clock"

  adb_retry_timed() {
    local _deadline="$1"
    shift
    printf '%s\n' "$*" >>"$operations"
    case "$1" in
      install)
        install_count="$(grep -c '^install ' "$operations")"
        case "$mode" in
          success-after-retry)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            printf '%s\n' "Success"
            ;;
          always-broken)
            printf '%s\n' \
              "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
            return 1
            ;;
          invalid)
            printf '%s\n' "Failure [INSTALL_FAILED_INVALID_APK]"
            return 9
            ;;
          timeout)
            printf '%s\n' \
              "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
            return 124
            ;;
          pipeline)
            printf '%s\n' \
              "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipeline (32)"
            return 1
            ;;
          wait-fail)
            printf '%s\n' \
              "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
            return 1
            ;;
          package-fail)
            printf '%s\n' \
              "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
            return 1
            ;;
          deadline)
            printf '%s\n' \
              "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
            printf '%s\n' "$INSTALL_RETRY_DEADLINE_SECS" >"$clock"
            return 1
            ;;
          third-non-one)
            if (( install_count < 3 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            printf '%s\n' "Failure [INSTALL_FAILED_INTERNAL_ERROR]"
            return 23
            ;;
          *)
            return 99
            ;;
        esac
        ;;
      wait-for-device)
        [[ "$mode" != "wait-fail" ]] || return 17
        ;;
      shell)
        [[ "$*" == "shell cmd package list packages" ]] || return 98
        [[ "$mode" != "package-fail" ]] || return 18
        ;;
      *)
        return 97
        ;;
    esac
  }

  set +e
  install_apk >"$temporary/$mode.stdout" 2>"$temporary/$mode.stderr"
  status=$?
  set -e
  [[ "$status" == "$expected_status" ]] \
    || {
      printf 'android-install-retry.test: %s status %s, expected %s\n' \
        "$mode" "$status" "$expected_status" >&2
      return 1
    }
  install_count="$(grep -c '^install ' "$operations")"
  operation_count="$(wc -l <"$operations" | tr -d ' ')"
  [[ "$install_count" == "$expected_install_count" ]] \
    || {
      printf 'android-install-retry.test: %s install count %s, expected %s\n' \
        "$mode" "$install_count" "$expected_install_count" >&2
      return 1
    }
  [[ "$operation_count" == "$expected_operation_count" ]] \
    || {
      printf 'android-install-retry.test: %s operation count %s, expected %s\n' \
        "$mode" "$operation_count" "$expected_operation_count" >&2
      return 1
    }
}

verify_install_retry_contract
run_case success-after-retry 0 2 4
run_case always-broken 1 3 7
run_case invalid 9 1 1
run_case timeout 124 1 1
run_case pipeline 1 1 1
run_case wait-fail 17 1 2
run_case package-fail 18 1 3
run_case deadline 124 1 1
run_case third-non-one 23 3 7

verify_operation_timeout_cap() {
  local captured_timeout="$temporary/operation-timeout"
  local status

  eval "$PRODUCTION_ADB_RETRY_TIMED"
  declare -f adb_with_timeout \
    | grep -F 'timeout --kill-after="${ADB_KILL_AFTER_SECS}s" "${timeout_secs}s"' >/dev/null
  install_retry_now() {
    printf '10\n'
  }
  adb_with_timeout() {
    printf '%s\n' "$1" >"$captured_timeout"
    return 29
  }

  set +e
  adb_retry_timed 12 install -r "$APK_PATH"
  status=$?
  set -e
  [[ "$status" == "29" ]]
  [[ "$(cat "$captured_timeout")" == "2" ]]

  : >"$captured_timeout"
  set +e
  adb_retry_timed 10 install -r "$APK_PATH"
  status=$?
  set -e
  [[ "$status" == "124" ]]
  [[ ! -s "$captured_timeout" ]]
}

verify_operation_timeout_cap

echo "android-install-retry.test: ok"
