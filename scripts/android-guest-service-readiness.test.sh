#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-guest-service-readiness.sh"

temporary="$(mktemp -d)"
cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT

mode=""
clock=0
operations="$temporary/operations"
sleeps="$temporary/sleeps"
logs="$temporary/logs"
package_counter="$temporary/package-counter"
settings_counter="$temporary/settings-counter"
guard_state="$temporary/guard-state"
last_wait_status=0

fake_now() {
  printf '%s\n' "$clock"
}

sleep() {
  printf '%s\n' "$1" >>"$sleeps"
  if [[ "$mode" == "sleep-fail" ]]; then
    return 41
  fi
  clock=$((clock + $1))
}

fake_log() {
  printf '%s\n' "$*" >>"$logs"
  case "$mode" in
    package-log-fail | settings-log-fail | stabilizing-log-fail)
      return 42
      ;;
  esac
}

fake_guard() {
  [[ "$mode" != "guard-fail" && "$(cat "$guard_state")" == "alive" ]] \
    || return 23
}

settings_provider_uninstalled() {
  printf '%s\n' \
    "java.lang.IllegalStateException: Cannot access system provider: 'settings' before system providers are installed!" \
    $'\tat com.android.server.am.ContentProviderHelper.getContentProviderImpl(ContentProviderHelper.java:423)' \
    $'\tat android.provider.Settings$NameValueCache.getStringForUser(Settings.java:3930)'
}

next_count() {
  local counter="$1"
  local count
  count="$(cat "$counter")"
  count=$((count + 1))
  printf '%s\n' "$count" >"$counter"
  printf '%s\n' "$count"
}

fake_probe() {
  local deadline="$1"
  local count
  shift
  printf '%s\t%s\n' "$deadline" "$*" >>"$operations"

  case "$*" in
    "shell cmd package list packages")
      count="$(next_count "$package_counter")"
      case "$mode" in
        package-reset)
          if (( count == 2 )); then
            printf '%s\n' "cmd: Can't find service: package"
            return 20
          fi
          ;;
        package-broken-pipe-reset)
          if (( count == 2 )); then
            printf '%s\n' "cmd: Failure calling service package: Broken pipe (32)"
            return 1
          fi
          ;;
        package-persistent)
          printf '%s\n' "cmd: Can't find service: package"
          return 20
          ;;
        package-timeout)
          printf '%s\n' "cmd: Can't find service: package"
          return 124
          ;;
        package-ready-then-timeout)
          printf '%s\n' "package:android"
          return 124
          ;;
        package-empty)
          return 0
          ;;
        package-garbage)
          printf '%s\n' "unexpected package output"
          return 0
          ;;
        package-success-mixed)
          printf '%s\n' "package:android" "error: device offline"
          return 0
          ;;
        package-log-fail)
          printf '%s\n' "cmd: Can't find service: package"
          return 20
          ;;
        package-mixed)
          printf '%s\n' "cmd: Can't find service: package" "error: device offline"
          return 20
          ;;
      esac
      printf '%s\n' "package:android"
      ;;
    "shell settings get global device_provisioned")
      count="$(next_count "$settings_counter")"
      case "$mode" in
        settings-reset)
          if (( count == 2 )); then
            settings_provider_uninstalled
            return 1
          fi
          ;;
        settings-broken-pipe-reset)
          if (( count == 2 )); then
            printf '%s\n' "cmd: Failure calling service settings: Broken pipe (32)"
            return 20
          fi
          ;;
        settings-null)
          printf '%s\n' "null"
          return 0
          ;;
        settings-log-fail)
          printf '%s\n' "null"
          return 0
          ;;
        settings-mixed)
          printf '%s\n' "1" "unexpected"
          return 0
          ;;
        settings-ready-then-timeout)
          printf '%s\n' "1"
          return 124
          ;;
        stable-zero)
          printf '%s\r\n' "0"
          return 0
          ;;
      esac
      printf '%s\n' "1"
      if [[ "$mode" == "guard-final-fail" && "$clock" == "10" ]]; then
        printf '%s\n' dead >"$guard_state"
      fi
      ;;
    *)
      return 99
      ;;
  esac
}

reset_case() {
  mode="$1"
  clock=0
  printf '0\n' >"$package_counter"
  printf '0\n' >"$settings_counter"
  printf '%s\n' alive >"$guard_state"
  : >"$operations"
  : >"$sleeps"
  : >"$logs"
}

wait_status() {
  local deadline="$1"
  local status=0
  android_guest_services_wait_stable \
    "$deadline" fake_probe fake_now fake_log fake_guard || status=$?
  last_wait_status="$status"
}

expect_stable() {
  local case_mode="$1"
  local expected_clock="$2"
  local expected_packages="$3"
  local expected_settings="$4"
  local expected_value="$5"
  reset_case "$case_mode"
  wait_status 30
  [[ "$last_wait_status" == "0" ]]
  [[ "$clock" == "$expected_clock" ]]
  [[ "$(cat "$package_counter")" == "$expected_packages" ]]
  [[ "$(cat "$settings_counter")" == "$expected_settings" ]]
  [[ "$ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE" \
    == "package-manager=stable settings-provider=stable stable_seconds=10 device_provisioned=$expected_value" ]]
  awk -F '\t' '$1 != 30 { exit 1 }' "$operations"
}

expect_failure() {
  local case_mode="$1"
  local deadline="$2"
  local expected_status="$3"
  local expected_evidence="$4"
  reset_case "$case_mode"
  wait_status "$deadline"
  [[ "$last_wait_status" == "$expected_status" ]]
  [[ "$ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE" == "$expected_evidence" ]]
}

expect_stable stable 10 6 6 1
expect_stable stable-zero 10 6 6 0
expect_stable package-reset 14 8 7 1
expect_stable package-broken-pipe-reset 14 8 7 1
expect_stable settings-reset 14 8 8 1
expect_stable settings-broken-pipe-reset 14 8 8 1

expect_failure \
  package-persistent 6 124 \
  "poll-sleep=failed status=124 after=package-manager-transient"
expect_failure \
  package-timeout 30 124 \
  "package-manager=unavailable status=124"
expect_failure \
  package-ready-then-timeout 30 124 \
  "package-manager=unavailable status=124"
expect_failure \
  package-empty 30 1 \
  "package-manager=invalid response=noncanonical"
expect_failure \
  package-garbage 30 1 \
  "package-manager=invalid response=noncanonical"
expect_failure \
  package-success-mixed 30 1 \
  "package-manager=invalid response=noncanonical"
expect_failure \
  package-mixed 30 20 \
  "package-manager=unavailable status=20"
expect_failure \
  settings-null 30 124 \
  "poll-sleep=failed status=124 after=settings-provider-transient"
expect_failure \
  settings-mixed 30 1 \
  "settings-provider=invalid device_provisioned=noncanonical"
expect_failure \
  settings-ready-then-timeout 30 124 \
  "settings-provider=unavailable status=124"
expect_failure \
  stable 10 124 \
  "poll-sleep=failed status=124 after=stabilizing"
expect_failure \
  guard-fail 30 23 \
  "process-guard=failed status=23"
expect_failure \
  guard-final-fail 30 23 \
  "process-guard=failed status=23 phase=admission"
expect_failure \
  package-log-fail 30 42 \
  "logger=failed status=42 after=package-manager-transient"
expect_failure \
  settings-log-fail 30 42 \
  "logger=failed status=42 after=settings-provider-transient"
expect_failure \
  stabilizing-log-fail 30 42 \
  "logger=failed status=42 after=stabilizing"
expect_failure \
  sleep-fail 30 41 \
  "poll-sleep=failed status=41 after=stabilizing"

echo "android-guest-service-readiness.test: ok"
