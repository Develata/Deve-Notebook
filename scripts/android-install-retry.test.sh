#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/check-mobile-android-install-startup-smoke.sh
source "$ROOT_DIR/scripts/check-mobile-android-install-startup-smoke.sh"
readonly PRODUCTION_ADB_RETRY_TIMED="$(declare -f adb_retry_timed)"
readonly PRODUCTION_INSTALL_STARTUP_CLEANUP="$(declare -f cleanup)"

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
  local count now
  [[ -n "$active_timing" ]] || return 0
  printf 'sleep %s\n' "$1" >>"$active_timing"
  count="$(awk '$1 == "sleep" { count += 1 } END { print count + 0 }' "$active_timing")"
  if [[ "$active_mode" == "readiness-sleep-fail" && "$count" == "2" ]]; then
    return 41
  fi
  if [[ "$active_mode" == "launcher-sleep-fail" && "$count" == "1" ]]; then
    return 42
  fi
  now="$(cat "$clock")"
  printf '%s\n' "$((now + $1))" >"$clock"
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

print_settings_provider_ready_failure() {
  printf '%s\n' \
    "Performing Streamed Install" \
    "adb: failed to install $APK_PATH:" \
    "Exception occurred while executing 'install':" \
    "java.lang.IllegalStateException: Cannot access system provider: 'settings' before system providers are installed!" \
    $'\tat com.android.server.am.ContentProviderHelper.getContentProviderImpl(ContentProviderHelper.java:423)' \
    $'\tat com.android.server.am.ContentProviderHelper.getContentProvider(ContentProviderHelper.java:151)' \
    $'\tat com.android.server.am.ActivityManagerService.getContentProvider(ActivityManagerService.java:7846)' \
    $'\tat android.app.ActivityThread.acquireProvider(ActivityThread.java:8702)' \
    $'\tat android.app.ContextImpl$ApplicationContentResolver.acquireProvider(ContextImpl.java:4057)' \
    $'\tat android.content.ContentResolver.acquireProvider(ContentResolver.java:2610)' \
    $'\tat android.provider.Settings$ContentProviderHolder.getProvider(Settings.java:3666)' \
    $'\tat android.provider.Settings$NameValueCache.getStringForUser(Settings.java:3930)' \
    $'\tat android.provider.Settings$Global.getStringForUser(Settings.java:19681)' \
    $'\tat android.provider.Settings$Global.getString(Settings.java:19664)' \
    $'\tat android.provider.Settings$Global.getInt(Settings.java:19886)' \
    $'\tat com.android.internal.content.InstallLocationUtils$1.getForceAllowOnExternalSetting(InstallLocationUtils.java:118)' \
    $'\tat com.android.internal.content.InstallLocationUtils.resolveInstallVolume(InstallLocationUtils.java:197)' \
    $'\tat com.android.internal.content.InstallLocationUtils.resolveInstallVolume(InstallLocationUtils.java:157)' \
    $'\tat com.android.internal.content.InstallLocationUtils.resolveInstallVolume(InstallLocationUtils.java:172)' \
    $'\tat com.android.server.pm.PackageInstallerService.createSessionInternal(PackageInstallerService.java:1064)' \
    $'\tat com.android.server.pm.PackageInstallerService.createSession(PackageInstallerService.java:758)' \
    $'\tat com.android.server.pm.PackageManagerShellCommand.doCreateSession(PackageManagerShellCommand.java:4119)' \
    $'\tat com.android.server.pm.PackageManagerShellCommand.doRunInstall(PackageManagerShellCommand.java:1641)' \
    $'\tat com.android.server.pm.PackageManagerShellCommand.runInstall(PackageManagerShellCommand.java:1577)' \
    $'\tat com.android.server.pm.PackageManagerShellCommand.onCommand(PackageManagerShellCommand.java:249)' \
    $'\tat com.android.modules.utils.BasicShellCommandHandler.exec(BasicShellCommandHandler.java:97)' \
    $'\tat android.os.ShellCommand.exec(ShellCommand.java:39)' \
    $'\tat com.android.server.pm.PackageManagerService$IPackageManagerImpl.onShellCommand(PackageManagerService.java:7069)' \
    $'\tat android.os.Binder.shellCommand(Binder.java:1088)' \
    $'\tat android.os.Binder.onTransact(Binder.java:946)' \
    $'\tat android.content.pm.IPackageManager$Stub.onTransact(IPackageManager.java:4786)' \
    $'\tat com.android.server.pm.PackageManagerService$IPackageManagerImpl.onTransact(PackageManagerService.java:7053)' \
    $'\tat android.os.Binder.execTransactInternal(Binder.java:1369)' \
    $'\tat android.os.Binder.execTransact(Binder.java:1323)'
}

print_settings_provider_probe_failure() {
  printf '%s\n' \
    "java.lang.IllegalStateException: Cannot access system provider: 'settings' before system providers are installed!" \
    $'\tat com.android.server.am.ContentProviderHelper.getContentProviderImpl(ContentProviderHelper.java:423)' \
    $'\tat android.provider.Settings$NameValueCache.getStringForUser(Settings.java:3930)'
}

run_case() {
  local mode="$1"
  local expected_status="$2"
  local expected_install_count="$3"
  local operations="$temporary/$mode.operations"
  local timing="$temporary/$mode.timing"
  local status install_count operation_count deadline_count deadline_unique
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
          package-missing-recover)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "Performing Streamed Install" \
                "adb: failed to install $APK_PATH: cmd: Can't find service: package"
              return 1
            fi
            printf '%s\n' "Success"
            ;;
          package-missing-mixed)
            printf '%s\n' \
              "Performing Streamed Install" \
              "adb: failed to install $APK_PATH: cmd: Can't find service: package" \
              "Failure [INSTALL_FAILED_INVALID_APK]"
            return 1
            ;;
          package-missing-other-service)
            printf '%s\n' \
              "adb: failed to install $APK_PATH: cmd: Can't find service: activity"
            return 1
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
          settings-provider-after-bootstrap)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            if (( install_count == 2 )); then
              print_settings_provider_ready_failure
              return 1
            fi
            printf '%s\n' "Success"
            ;;
          settings-provider-first)
            print_settings_provider_ready_failure
            return 1
            ;;
          settings-provider-mixed)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            print_settings_provider_ready_failure
            printf '%s\n' "Failure [INSTALL_FAILED_INVALID_APK]"
            return 1
            ;;
          settings-readiness-recover|settings-readiness-mixed|settings-readiness-timeout)
            if (( install_count == 1 )); then
              printf '%s\n' \
                "adb: failed to install $APK_PATH: cmd: Failure calling service package: Broken pipe (32)"
              return 1
            fi
            printf '%s\n' "Success"
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
          launcher-delayed|launcher-mixed|launcher-other|launcher-timeout|launcher-unavailable|launcher-deadline|launcher-sleep-fail)
            printf '%s\n' "Success"
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
        if [[ "$*" == "shell cmd package list packages" ]]; then
          case "$mode" in
            package-fail)
              printf '%s\n' "error: device offline"
              return 18
              ;;
            package-recover)
              package_count="$(grep -c '^shell cmd package list packages$' "$operations")"
              if (( package_count == 1 )); then
                printf '%s\n' "cmd: Failure calling service package: Broken pipe (32)"
                return 224
              fi
              if (( package_count == 2 )); then
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
          printf '%s\n' "package:android"
        elif [[ "$*" == "shell settings get global device_provisioned" ]]; then
          case "$mode" in
            settings-readiness-recover)
              settings_count="$(grep -c '^shell settings get global device_provisioned$' "$operations")"
              if (( settings_count == 1 )); then
                print_settings_provider_probe_failure
                return 1
              fi
              ;;
            settings-readiness-mixed)
              printf '%s\n' "null" "unexpected settings output"
              return 0
              ;;
            settings-readiness-timeout)
              return 124
              ;;
          esac
          printf '%s\n' "1"
        elif [[ "$*" == "shell cmd package resolve-activity --components -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p $APP_ID" ]]; then
          case "$mode" in
            launcher-delayed)
              launcher_count="$(grep -c '^shell cmd package resolve-activity ' "$operations")"
              if (( launcher_count < 3 )); then
                printf '%s\n' "No activity found"
                return 0
              fi
              ;;
            launcher-mixed)
              printf '%s\n' "No activity found" "unexpected launcher output"
              return 0
              ;;
            launcher-other)
              printf '%s\n' "com.example/.OtherActivity"
              return 0
              ;;
            launcher-timeout)
              return 124
              ;;
            launcher-unavailable|launcher-sleep-fail)
              printf '%s\n' "No activity found"
              return 0
              ;;
            launcher-deadline)
              printf '%s\n' "No activity found"
              printf '%s\n' "$INSTALL_RETRY_DEADLINE_SECS" >"$clock"
              return 0
              ;;
          esac
          printf '%s\n' "$APP_ID/.MainActivity"
        else
          return 98
        fi
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
      sed -n '1,80p' "$temporary/$mode.stderr" >&2
      return 1
    }
  install_count="$(grep -c '^install ' "$operations")"
  operation_count="$(wc -l <"$operations" | tr -d ' ')"
  deadline_count="$(awk '$1 == "deadline" { count += 1 } END { print count + 0 }' "$timing")"
  deadline_unique="$(awk '$1 == "deadline" { seen[$2] = 1 } END { for (value in seen) count += 1; print count + 0 }' "$timing")"
  [[ "$install_count" == "$expected_install_count" ]] \
    || {
      printf 'android-install-retry.test: %s install count %s, expected %s\n' \
        "$mode" "$install_count" "$expected_install_count" >&2
      return 1
    }
  [[ "$deadline_count" == "$operation_count" && "$deadline_unique" == "1" ]] \
    || {
      printf 'android-install-retry.test: %s deadline count/unique %s/%s, expected %s/1\n' \
        "$mode" "$deadline_count" "$deadline_unique" "$operation_count" >&2
      return 1
    }
  if (( expected_install_count > 1 )); then
    grep -Fx 'shell cmd package list packages' "$operations" >/dev/null
    grep -Fx 'shell settings get global device_provisioned' "$operations" >/dev/null
  fi
  if [[ "$mode" == "package-timeout-status" ]]; then
    grep -Fx \
      "mobile-android-install-startup-smoke-check: guest-service admission failed: package-manager=unavailable status=124" \
      "$temporary/$mode.stderr" >/dev/null
  fi
  active_mode=""
  active_timing=""
}

verify_install_retry_contract
run_case success-after-retry 0 2
run_case package-missing-recover 0 2
run_case package-missing-mixed 1 1
run_case package-missing-other-service 1 1
run_case always-broken 1 3
run_case invalid 9 1
run_case timeout 124 1
run_case pipeline 1 1
run_case wait-fail 17 1
run_case package-fail 18 1
run_case package-recover 0 2
run_case package-internal-recover 0 3
run_case package-internal-first 1 1
run_case package-internal-mixed 1 2
run_case settings-provider-after-bootstrap 0 3
run_case settings-provider-first 1 1
run_case settings-provider-mixed 1 2
run_case settings-readiness-recover 0 2
run_case settings-readiness-mixed 1 1
run_case settings-readiness-timeout 124 1
run_case package-unavailable 124 1
run_case package-timeout-status 124 1
run_case package-deadline 124 1
run_case readiness-sleep-fail 41 1
run_case deadline 124 1
run_case third-non-one 23 3
run_case launcher-delayed 0 1
run_case launcher-mixed 1 1
run_case launcher-other 1 1
run_case launcher-timeout 124 1
run_case launcher-unavailable 1 1
run_case launcher-deadline 124 1
run_case launcher-sleep-fail 42 1

verify_admission_log_failure_preserves_status() {
  local expected status
  android_guest_services_wait_stable() {
    ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="forced-status=$expected"
    return "$expected"
  }
  android_install_retry_log() {
    return 42
  }
  for expected in 124 224; do
    set +e
    wait_for_android_guest_services_stable 180
    status=$?
    set -e
    [[ "$status" == "$expected" ]]
  done
}

verify_admission_log_failure_preserves_status

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
  adb_retry_timed 20 install -r "$APK_PATH"
  status=$?
  set -e
  [[ "$status" == "29" ]]
  [[ "$(cat "$captured_timeout")" == "5" ]]

  : >"$captured_timeout"
  set +e
  adb_retry_timed 15 install -r "$APK_PATH"
  status=$?
  set -e
  [[ "$status" == "124" ]]
  [[ ! -s "$captured_timeout" ]]
}

verify_operation_timeout_cap

verify_install_startup_cleanup_case() (
  local mode="$1"
  local expected="$2"
  local status
  UNINSTALL_AFTER=1
  eval "$PRODUCTION_INSTALL_STARTUP_CLEANUP"

  adb_timed() {
    case "$*" in
      "uninstall $APP_ID")
        [[ "$mode" != "uninstall-fail" ]]
        ;;
      "shell pm list packages $APP_ID")
        [[ "$mode" != "package-probe-fail" ]] || return 19
        [[ "$mode" != "package-remains" ]] || printf 'package:%s\n' "$APP_ID"
        return 0
        ;;
      "shell cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER $APP_ID")
        if [[ "$mode" == "launcher-remains" ]]; then
          printf '%s/.MainActivity\n' "$APP_ID"
        else
          printf 'No activity found\n'
        fi
        return 0
        ;;
      "shell ps -A")
        printf 'USER PID PPID VSZ RSS WCHAN ADDR S NAME\n'
        [[ "$mode" != "process-remains" ]] \
          || printf 'u0_a1 123 1 0 0 0 0 S %s\n' "$APP_ID"
        return 0
        ;;
      *)
        return 97
        ;;
    esac
  }

  set +e
  cleanup >/dev/null 2>&1
  status=$?
  set -e
  [[ "$status" == "$expected" ]] \
    || { printf 'android-install-retry.test: cleanup %s status %s, expected %s\n' \
      "$mode" "$status" "$expected" >&2; return 1; }
)

verify_install_startup_cleanup_case retired 0
verify_install_startup_cleanup_case uninstall-fail 1
verify_install_startup_cleanup_case package-probe-fail 1
verify_install_startup_cleanup_case package-remains 1
verify_install_startup_cleanup_case launcher-remains 1
verify_install_startup_cleanup_case process-remains 1

verify_cleanup_status_precedence() {
  local status
  set +e
  (
    cleanup() { return 42; }
    trap cleanup_on_exit EXIT
    exit 17
  )
  status=$?
  set -e
  [[ "$status" == "17" ]] \
    || { printf 'android-install-retry.test: primary cleanup status %s, expected 17\n' "$status" >&2; return 1; }

  set +e
  (
    cleanup() { return 42; }
    trap cleanup_on_exit EXIT
    exit 0
  )
  status=$?
  set -e
  [[ "$status" == "42" ]] \
    || { printf 'android-install-retry.test: success cleanup status %s, expected 42\n' "$status" >&2; return 1; }
}

verify_cleanup_status_precedence

echo "android-install-retry.test: ok"
