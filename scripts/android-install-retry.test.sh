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
active_mode=""
active_timing=""
touch "$APK_PATH"

install_retry_now() {
  cat "$clock"
}

sleep() {
  local count
  [[ -n "$active_timing" ]] || return 0
  printf 'sleep %s\n' "$1" >>"$active_timing"
  count="$(awk '$1 == "sleep" { count += 1 } END { print count + 0 }' "$active_timing")"
  if [[ "$active_mode" == "readiness-sleep-fail" && "$count" == "2" ]]; then
    return 41
  fi
}

print_package_services_ready_failure() {
  printf '%s\n' \
    "Performing Streamed Install" \
    "adb: failed to install $APK_PATH:" \
    "Exception occurred while executing 'install':" \
    "java.lang.NullPointerException: Attempt to invoke virtual method 'void android.content.pm.PackageManagerInternal.freeStorage(java.lang.String, long, int)' on a null object reference" \
    $'\tat com.android.server.StorageManagerService.allocateBytes(StorageManagerService.java:4266)' \
    $'\tat com.android.server.pm.PackageInstallerSession.doWriteInternal(PackageInstallerSession.java:2314)'
}

run_case() {
  local mode="$1"
  local expected_status="$2"
  local expected_install_count="$3"
  local expected_operation_count="$4"
  local expected_sleep_count="$5"
  local operations="$temporary/$mode.operations"
  local timing="$temporary/$mode.timing"
  local status install_count operation_count sleep_count deadline_count deadline_unique
  : >"$operations"
  : >"$timing"
  printf '0\n' >"$clock"
  active_mode="$mode"
  active_timing="$timing"

  adb_retry_timed() {
    local _deadline="$1"
    shift
    printf 'deadline %s\n' "$_deadline" >>"$timing"
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
          package-recover)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            printf '%s\n' "Success"
            ;;
          package-internal-recover)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            if (( install_count == 2 )); then
              print_package_services_ready_failure
              return 1
            fi
            printf '%s\n' "Success"
            ;;
          package-internal-first)
            print_package_services_ready_failure
            return 1
            ;;
          package-internal-mixed)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            print_package_services_ready_failure
            printf '%s\n' "Failure [INSTALL_FAILED_INVALID_APK]"
            return 1
            ;;
          package-unavailable|package-timeout-status|package-deadline|readiness-sleep-fail)
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
        case "$mode" in
          package-fail)
            printf '%s\n' "error: device offline"
            return 18
            ;;
          package-recover)
            package_count="$(grep -c '^shell cmd package list packages$' "$operations")"
            if (( package_count < 3 )); then
              printf '%s\n' "cmd: Can't find service: package"
              return 20
            fi
            ;;
          package-unavailable)
            printf '%s\n' "cmd: Can't find service: package"
            return 20
            ;;
          package-timeout-status)
            printf '%s\n' "cmd: Can't find service: package"
            return 124
            ;;
          package-deadline)
            printf '%s\n' "cmd: Can't find service: package"
            printf '%s\n' "$INSTALL_RETRY_DEADLINE_SECS" >"$clock"
            return 20
            ;;
          readiness-sleep-fail)
            printf '%s\n' "cmd: Can't find service: package"
            return 20
            ;;
        esac
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
  sleep_count="$(awk '$1 == "sleep" && $2 == "2" { count += 1 } END { print count + 0 }' "$timing")"
  deadline_count="$(awk '$1 == "deadline" { count += 1 } END { print count + 0 }' "$timing")"
  deadline_unique="$(awk '$1 == "deadline" { seen[$2] = 1 } END { for (value in seen) count += 1; print count + 0 }' "$timing")"
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
  [[ "$sleep_count" == "$expected_sleep_count" ]] \
    || {
      printf 'android-install-retry.test: %s sleep count %s, expected %s\n' \
        "$mode" "$sleep_count" "$expected_sleep_count" >&2
      return 1
    }
  [[ "$deadline_count" == "$operation_count" && "$deadline_unique" == "1" ]] \
    || {
      printf 'android-install-retry.test: %s deadline count/unique %s/%s, expected %s/1\n' \
        "$mode" "$deadline_count" "$deadline_unique" "$operation_count" >&2
      return 1
    }
  active_mode=""
  active_timing=""
}

verify_install_retry_contract
run_case success-after-retry 0 2 4 1
run_case always-broken 1 3 7 2
run_case invalid 9 1 1 0
run_case timeout 124 1 1 0
run_case pipeline 1 1 1 0
run_case wait-fail 17 1 2 1
run_case package-fail 18 1 3 1
run_case package-recover 0 2 6 3
run_case package-internal-recover 0 3 7 2
run_case package-internal-first 1 1 1 0
run_case package-internal-mixed 1 2 4 1
run_case package-unavailable 20 1 12 10
run_case package-timeout-status 124 1 3 1
run_case package-deadline 124 1 3 1
run_case readiness-sleep-fail 41 1 3 2
run_case deadline 124 1 1 0
run_case third-non-one 23 3 7 2

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
