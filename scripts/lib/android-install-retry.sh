#!/usr/bin/env bash
# Bounded Android APK install recovery shared by every gate that installs the
# debug WebView shell over adb. Right after boot (and intermittently for the
# first minutes of an emulator session) the guest package service can drop the
# binder connection mid-install; this module retries only the exact admitted
# package-service failure signatures and stays fail-closed for everything else.
#
# Contract:
# - The including script MUST define `adb_with_timeout <timeout-secs>
#   <adb-args...>` that runs one adb command under its bounded timeout
#   machinery and returns that command's status.
# - The including script MUST define `fail <msg>` and set `APP_ID`,
#   `APK_PATH`, and `ADB_TIMEOUT_SECS` before calling `install_apk`.
# - `ANDROID_INSTALL_RETRY_LOG_PREFIX` labels retry progress messages; hosts
#   set it to their own log prefix so remote logs attribute the recovery.
# - `install_apk` performs at most 3 install attempts under one absolute
#   deadline, admits only the exact package-service bootstrap and
#   package-services-ready failure lines, waits for the package service to
#   return between attempts, and requires the launcher activity to resolve
#   before reporting success. Timeouts and mixed failures never retry.

if [[ -n "${ANDROID_INSTALL_RETRY_LOADED:-}" ]]; then
  return 0
fi
ANDROID_INSTALL_RETRY_LOADED=1

readonly INSTALL_RETRY_DEADLINE_SECS=180
readonly PACKAGE_SERVICE_READY_ATTEMPTS=10
readonly PACKAGE_SERVICE_READY_INTERVAL_SECS=2
readonly LAUNCHER_READY_ATTEMPTS=10
readonly LAUNCHER_READY_INTERVAL_SECS=2

android_install_retry_log() {
  echo "${ANDROID_INSTALL_RETRY_LOG_PREFIX:-android-install-retry}: $*" >&2
}

install_retry_now() {
  printf '%s\n' "$SECONDS"
}

adb_retry_timed() {
  local deadline="$1"
  local now remaining operation_timeout
  shift

  now="$(install_retry_now)"
  remaining=$((deadline - now))
  (( remaining > 0 )) || return 124
  operation_timeout="$ADB_TIMEOUT_SECS"
  (( operation_timeout <= remaining )) || operation_timeout="$remaining"
  adb_with_timeout "$operation_timeout" "$@"
}

retryable_android_package_install_failure() {
  local status="$1"
  local output="$2"

  (( status == 1 )) || return 1
  printf '%s\n' "$output" | tr -d '\r' | awk '
    /^[[:space:]]*$/ { next }
    $0 == "Performing Streamed Install" {
      streamed_install += 1
      next
    }
    /^adb: failed to install .+: cmd: Failure calling service package: Broken pipe \(32\)$/ {
      broken_pipe += 1
      next
    }
    /^adb: failed to install .+: cmd: Can'\''t find service: package$/ {
      package_service_missing += 1
      next
    }
    { unexpected = 1 }
    END {
      exit !(broken_pipe + package_service_missing == 1 &&
        streamed_install <= 1 &&
        unexpected == 0)
    }
  '
}

retryable_android_package_services_ready_install_failure() {
  local status="$1"
  local output="$2"

  (( status == 1 )) || return 1
  printf '%s\n' "$output" | tr -d '\r' | awk '
    /^[[:space:]]*$/ { next }
    $0 == "Performing Streamed Install" {
      streamed_install += 1
      next
    }
    /^adb: failed to install .+:[[:space:]]*$/ {
      install_header += 1
      next
    }
    $0 == "Exception occurred while executing '\''install'\'':" {
      exception_header += 1
      next
    }
    $0 == "java.lang.NullPointerException: Attempt to invoke virtual method '\''void android.content.pm.PackageManagerInternal.freeStorage(java.lang.String, long, int)'\'' on a null object reference" {
      package_internal_missing += 1
      next
    }
    /^[[:space:]]+at / {
      stack_frames += 1
      if ($0 ~ /^[[:space:]]+at com\.android\.server\.StorageManagerService\.allocateBytes\(/) {
        storage_allocate += 1
      }
      if ($0 ~ /^[[:space:]]+at com\.android\.server\.pm\.PackageInstallerSession\.doWriteInternal\(/) {
        package_write += 1
      }
      next
    }
    { unexpected = 1 }
    END {
      exit !(install_header == 1 &&
        exception_header == 1 &&
        package_internal_missing == 1 &&
        storage_allocate == 1 &&
        package_write == 1 &&
        stack_frames >= 2 &&
        streamed_install <= 1 &&
        unexpected == 0)
    }
  '
}

retryable_android_package_readiness_failure() {
  local status="$1"
  local output="$2"

  (( status == 20 )) || return 1
  printf '%s\n' "$output" | tr -d '\r' | awk '
    /^[[:space:]]*$/ { next }
    $0 == "cmd: Can'\''t find service: package" {
      unavailable += 1
      next
    }
    { unexpected = 1 }
    END { exit !(unavailable == 1 && unexpected == 0) }
  '
}

wait_for_android_package_service() {
  local deadline="$1"
  local attempt now output status

  for ((attempt = 1; attempt <= PACKAGE_SERVICE_READY_ATTEMPTS; attempt += 1)); do
    if output="$(adb_retry_timed "$deadline" shell cmd package list packages 2>&1)"; then
      return 0
    else
      status=$?
    fi
    printf '%s\n' "$output" >&2
    if ! retryable_android_package_readiness_failure "$status" "$output"; then
      return "$status"
    fi
    if (( attempt == PACKAGE_SERVICE_READY_ATTEMPTS )); then
      return "$status"
    fi
    now="$(install_retry_now)"
    (( deadline - now > PACKAGE_SERVICE_READY_INTERVAL_SECS )) || return 124
    android_install_retry_log "waiting for package service (attempt $attempt/$PACKAGE_SERVICE_READY_ATTEMPTS)"
    sleep "$PACKAGE_SERVICE_READY_INTERVAL_SECS" || return $?
  done
}

wait_for_android_launcher_activity() {
  local deadline="$1"
  local attempt now output status
  local expected_component="$APP_ID/.MainActivity"

  for ((attempt = 1; attempt <= LAUNCHER_READY_ATTEMPTS; attempt += 1)); do
    if output="$(adb_retry_timed "$deadline" shell cmd package resolve-activity \
        --components \
        -a android.intent.action.MAIN \
        -c android.intent.category.LAUNCHER \
        -p "$APP_ID" 2>&1)"; then
      status=0
    else
      status=$?
    fi
    output="${output//$'\r'/}"
    if (( status == 0 )) && [[ "$output" == "$expected_component" ]]; then
      return 0
    fi
    printf '%s\n' "$output" >&2
    if (( status != 0 )); then
      return "$status"
    fi
    [[ "$output" == "No activity found" ]] || return 1
    if (( attempt == LAUNCHER_READY_ATTEMPTS )); then
      return 1
    fi
    now="$(install_retry_now)"
    (( deadline - now > LAUNCHER_READY_INTERVAL_SECS )) || return 124
    android_install_retry_log "waiting for launcher activity (attempt $attempt/$LAUNCHER_READY_ATTEMPTS)"
    sleep "$LAUNCHER_READY_INTERVAL_SECS" || return $?
  done
}

verify_install_retry_contract() {
  local package_services_ready_failure readiness_status

  package_services_ready_failure=$'Performing Streamed Install\nadb: failed to install candidate.apk:\nException occurred while executing \'install\':\njava.lang.NullPointerException: Attempt to invoke virtual method \'void android.content.pm.PackageManagerInternal.freeStorage(java.lang.String, long, int)\' on a null object reference\n\tat com.android.server.StorageManagerService.allocateBytes(StorageManagerService.java:4266)\n\tat com.android.server.pm.PackageInstallerSession.doWriteInternal(PackageInstallerSession.java:2314)'

  retryable_android_package_install_failure 1 \
    "adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)" \
    || fail "the exact package-service Broken pipe must remain retryable"
  retryable_android_package_install_failure 1 \
    $'Performing Streamed Install\nadb: failed to install candidate.apk: cmd: Can'\''t find service: package' \
    || fail "the exact missing package service must remain retryable"
  if retryable_android_package_install_failure 1 \
    "adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipeline (32)"; then
    fail "Broken pipeline must not be classified as Broken pipe"
  fi
  if retryable_android_package_install_failure 124 \
    "adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)"; then
    fail "timed-out Android installs must not be retried"
  fi
  if retryable_android_package_install_failure 1 \
    $'adb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)\nFailure [INSTALL_FAILED_INVALID_APK]'; then
    fail "mixed Android install failures must not be retried"
  fi
  if retryable_android_package_install_failure 1 \
    $'Performing Streamed Install\nPerforming Streamed Install\nadb: failed to install candidate.apk: cmd: Failure calling service package: Broken pipe (32)'; then
    fail "duplicate streamed-install Broken pipe prefixes must remain fail-closed"
  fi
  if retryable_android_package_install_failure 1 \
    $'Performing Streamed Install\nPerforming Streamed Install\nadb: failed to install candidate.apk: cmd: Can'\''t find service: package'; then
    fail "duplicate streamed-install missing-service prefixes must remain fail-closed"
  fi
  if retryable_android_package_install_failure 1 \
    $'adb: failed to install candidate.apk: cmd: Can'\''t find service: package\nFailure [INSTALL_FAILED_INVALID_APK]'; then
    fail "mixed missing-service install failures must remain fail-closed"
  fi
  if retryable_android_package_install_failure 1 \
    "adb: failed to install candidate.apk: cmd: Can't find service: activity"; then
    fail "a missing non-package service must remain fail-closed"
  fi
  if retryable_android_package_install_failure 1 \
    "adb: failed to install candidate.apk: Failure [INSTALL_FAILED_INVALID_APK]"; then
    fail "non-transport Android install failures must remain fail-closed"
  fi
  retryable_android_package_services_ready_install_failure 1 \
    "$package_services_ready_failure" \
    || fail "the exact package-services-ready race must remain retryable only after an admitted package-service bootstrap failure"
  if retryable_android_package_services_ready_install_failure 124 \
    "$package_services_ready_failure"; then
    fail "timed-out package-services-ready failures must not be retried"
  fi
  if retryable_android_package_services_ready_install_failure 1 \
    "$package_services_ready_failure"$'\nFailure [INSTALL_FAILED_INVALID_APK]'; then
    fail "mixed package-services-ready failures must remain fail-closed"
  fi
  if retryable_android_package_services_ready_install_failure 1 \
    $'Performing Streamed Install\n'"$package_services_ready_failure"; then
    fail "duplicate streamed-install prefixes must remain fail-closed"
  fi
  retryable_android_package_readiness_failure 20 \
    "cmd: Can't find service: package" \
    || fail "package-service restart must remain the only retryable readiness failure"
  for readiness_status in 124 130 143; do
    if retryable_android_package_readiness_failure "$readiness_status" \
      "cmd: Can't find service: package"; then
      fail "timeout and interruption statuses must not be retried: $readiness_status"
    fi
  done
  if retryable_android_package_readiness_failure 20 \
    $'cmd: Can'\''t find service: package\nerror: device offline'; then
    fail "mixed Android readiness failures must not be retried"
  fi
}

install_apk() {
  local attempt now output retry_reason status
  local deadline
  local recovering_package_services=0
  now="$(install_retry_now)"
  deadline=$((now + INSTALL_RETRY_DEADLINE_SECS))

  for attempt in 1 2 3; do
    echo "+ adb_timed install -r $APK_PATH (attempt $attempt/3)"
    if output="$(adb_retry_timed "$deadline" install -r "$APK_PATH" 2>&1)"; then
      printf '%s\n' "$output"
      wait_for_android_launcher_activity "$deadline" || return $?
      return 0
    else
      status=$?
    fi
    printf '%s\n' "$output" >&2
    if (( attempt == 3 )); then
      return "$status"
    fi
    if retryable_android_package_install_failure "$status" "$output"; then
      recovering_package_services=1
      retry_reason="package-service bootstrap"
    elif (( recovering_package_services == 1 )) \
        && retryable_android_package_services_ready_install_failure "$status" "$output"; then
      retry_reason="package-services-ready race"
    else
      return "$status"
    fi
    android_install_retry_log "retrying after $retry_reason"
    now="$(install_retry_now)"
    (( deadline - now > 2 )) || return 124
    sleep 2 || return $?
    adb_retry_timed "$deadline" wait-for-device >/dev/null || return $?
    wait_for_android_package_service "$deadline" || return $?
  done
}
