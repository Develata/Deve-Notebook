#!/usr/bin/env bash
# Shared Android package/settings stable-admission boundary.
#
# The caller supplies:
# - an absolute deadline;
# - a bounded probe callback with signature
#   `<probe> <deadline> [adb-prefix...] <adb-args...>`;
# - a monotonic clock callback returning integer seconds;
# - a progress logger callback;
# - a process guard callback (`:` when no extra guard is required).
#
# A single successful package/settings sample is not admission. Both services
# must remain ready for one continuous window. Exact bootstrap transients reset
# that window; unknown, mixed, timed-out, interrupted, and guard failures
# propagate fail-closed.

if [[ -n "${ANDROID_GUEST_SERVICE_READINESS_LOADED:-}" ]]; then
  return 0
fi
ANDROID_GUEST_SERVICE_READINESS_LOADED=1

readonly ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS=10
readonly ANDROID_GUEST_SERVICE_POLL_INTERVAL_SECS=2
readonly ANDROID_GUEST_SERVICE_DIAGNOSTIC_PREVIEW_BYTES=160
ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="not-probed"

android_guest_service_rejected_response_diagnostic() {
  local LC_ALL=C
  local prefix="${1:0:$((ANDROID_GUEST_SERVICE_DIAGNOSTIC_PREVIEW_BYTES + 1))}"
  local truncated="false"
  local shape="" previous_class="" mapped class char
  local prefix_bytes line_count index

  if (( ${#prefix} > ANDROID_GUEST_SERVICE_DIAGNOSTIC_PREVIEW_BYTES )); then
    truncated="true"
    prefix="${prefix:0:ANDROID_GUEST_SERVICE_DIAGNOSTIC_PREVIEW_BYTES}"
  fi
  prefix="${prefix//$'\r'/}"
  prefix_bytes="${#prefix}"
  if (( prefix_bytes == 0 )); then
    line_count=0
  else
    line_count=1
  fi

  for (( index = 0; index < prefix_bytes; index += 1 )); do
    char="${prefix:index:1}"
    case "$char" in
      $'\n')
        line_count=$((line_count + 1))
        mapped=" "
        class="space"
        ;;
      $'\t' | " ")
        mapped=" "
        class="space"
        ;;
      [A-Za-z])
        mapped="a"
        class="alpha"
        ;;
      [0-9])
        mapped="0"
        class="digit"
        ;;
      "." | "," | ":" | ";" | "_" | "(" | ")" | "/" | "=" | "-")
        mapped="$char"
        class=""
        ;;
      *)
        mapped="?"
        class="other"
        ;;
    esac
    if [[ -n "$class" && "$class" == "$previous_class" ]]; then
      continue
    fi
    shape+="$mapped"
    previous_class="$class"
  done

  [[ -n "$shape" ]] || shape="empty"
  printf 'response_prefix_bytes=%s response_sample_lines=%s response_truncated=%s response_shape=[%s]\n' \
    "$prefix_bytes" "$line_count" "$truncated" "$shape"
}

# AOSP cmd returns a negative Binder status directly. EPIPE (-32) is observed
# by a POSIX host shell as 224, so that status is admitted only when canonical
# output independently proves the matching Broken pipe condition.
android_guest_service_retryable_package_failure() {
  local status="$1"
  local output="$2"

  (( status == 1 || status == 20 || status == 224 )) || return 1
  printf '%s\n' "$output" | tr -d '\r' | awk -v observed_status="$status" '
    /^[[:space:]]*$/ { next }
    $0 == "cmd: Can'\''t find service: package" {
      missing += 1
      next
    }
    $0 == "cmd: Failure calling service package: Broken pipe (32)" {
      broken_pipe += 1
      next
    }
    { unexpected = 1 }
    END {
      ordinary_status = observed_status == 1 || observed_status == 20
      binder_epipe_status = observed_status == 224
      exit !(unexpected == 0 &&
        ((ordinary_status && missing + broken_pipe == 1) ||
         (binder_epipe_status && broken_pipe == 1 && missing == 0)))
    }
  '
}

android_guest_service_package_ready() {
  local output="$1"

  printf '%s\n' "$output" | tr -d '\r' | awk '
    /^[[:space:]]*$/ { next }
    $0 == "package:android" {
      platform_package += 1
      packages += 1
      next
    }
    /^package:[A-Za-z0-9_]+([.][A-Za-z0-9_]+)*$/ {
      packages += 1
      next
    }
    { unexpected = 1 }
    END {
      exit !(platform_package == 1 && packages >= 1 && unexpected == 0)
    }
  '
}

android_guest_service_retryable_settings_failure() {
  local status="$1"
  local output="${2//$'\r'/}"

  if (( status == 0 )); then
    [[ -z "$output" || "$output" == "null" ]]
    return
  fi
  if (( status == 224 )); then
    [[ "$output" == "cmd: Failure calling service settings: Broken pipe (32)" ]]
    return
  fi
  if (( status == 1 || status == 20 )); then
    case "$output" in
      "cmd: Can't find service: settings" \
        | "cmd: Failure calling service settings: Broken pipe (32)")
        return 0
        ;;
    esac
  fi
  (( status == 1 )) || return 1
  printf '%s\n' "$output" | awk '
    /^[[:space:]]*$/ { next }
    $0 == "java.lang.IllegalStateException: Cannot access system provider: '\''settings'\'' before system providers are installed!" {
      provider_uninstalled += 1
      next
    }
    /^[[:space:]]+at / {
      stack_frames += 1
      if ($0 ~ /^[[:space:]]+at com\.android\.server\.am\.ContentProviderHelper\.getContentProviderImpl\(/) {
        provider_lookup += 1
      }
      if ($0 ~ /^[[:space:]]+at android\.provider\.Settings\$NameValueCache\.getStringForUser\(/) {
        settings_lookup += 1
      }
      next
    }
    { unexpected = 1 }
    END {
      exit !(provider_uninstalled == 1 &&
        provider_lookup == 1 &&
        settings_lookup == 1 &&
        stack_frames >= 2 &&
        unexpected == 0)
    }
  '
}

android_guest_service_log_or_fail() {
  local log_fn="$1"
  local phase="$2"
  local status
  shift 2

  if "$log_fn" "$@"; then
    return 0
  else
    status=$?
  fi
  ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="logger=failed status=$status after=$phase"
  return "$status"
}

android_guest_service_read_clock() {
  local now_fn="$1"
  local now

  now="$("$now_fn")" || return $?
  [[ "$now" =~ ^[0-9]+$ ]] || return 1
  printf '%s\n' "$now"
}

android_guest_service_poll_sleep() {
  local deadline="$1"
  local now_fn="$2"
  local now remaining status

  now="$(android_guest_service_read_clock "$now_fn")" || return $?
  remaining=$((deadline - now))
  (( remaining > ANDROID_GUEST_SERVICE_POLL_INTERVAL_SECS )) || return 124
  if sleep "$ANDROID_GUEST_SERVICE_POLL_INTERVAL_SECS"; then
    return 0
  else
    status=$?
  fi
  return "$status"
}

android_guest_services_wait_stable() {
  local deadline="$1"
  local probe_fn="$2"
  local now_fn="$3"
  local log_fn="$4"
  local process_guard="${5:-:}"
  shift 5
  local probe_prefix=("$@")
  local stable_since=""
  local elapsed=0
  local now output package_status settings_status guard_status sleep_status
  local response_diagnostic
  local provisioned=""

  while :; do
    now="$(android_guest_service_read_clock "$now_fn")" || {
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="clock=invalid"
      return 1
    }
    if (( now >= deadline )); then
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="deadline=expired stable_seconds=$elapsed/$ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS"
      return 124
    fi

    if "$process_guard"; then
      :
    else
      guard_status=$?
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="process-guard=failed status=$guard_status"
      return "$guard_status"
    fi

    if output="$("$probe_fn" "$deadline" "${probe_prefix[@]}" \
        shell cmd package list packages 2>&1)"; then
      package_status=0
    else
      package_status=$?
    fi
    if (( package_status == 0 )); then
      if ! android_guest_service_package_ready "$output"; then
        ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="package-manager=invalid response=noncanonical"
        return 1
      fi
    else
      if ! android_guest_service_retryable_package_failure \
          "$package_status" "$output"; then
        if (( package_status == 224 )) \
            && response_diagnostic="$(
              android_guest_service_rejected_response_diagnostic "$output"
            )"; then
          ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="package-manager=unavailable status=$package_status $response_diagnostic"
        else
          ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="package-manager=unavailable status=$package_status"
        fi
        return "$package_status"
      fi
      stable_since=""
      elapsed=0
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="package-manager=transient status=$package_status stable-window=reset"
      android_guest_service_log_or_fail \
        "$log_fn" package-manager-transient \
        "waiting for stable package service" || return $?
      if android_guest_service_poll_sleep "$deadline" "$now_fn"; then
        continue
      else
        sleep_status=$?
      fi
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="poll-sleep=failed status=$sleep_status after=package-manager-transient"
      return "$sleep_status"
    fi

    if output="$("$probe_fn" "$deadline" "${probe_prefix[@]}" \
        shell settings get global device_provisioned 2>&1)"; then
      settings_status=0
    else
      settings_status=$?
    fi
    output="${output//$'\r'/}"
    if (( settings_status == 0 )) && [[ "$output" == "0" || "$output" == "1" ]]; then
      provisioned="$output"
    elif android_guest_service_retryable_settings_failure \
        "$settings_status" "$output"; then
      stable_since=""
      elapsed=0
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="settings-provider=transient status=$settings_status stable-window=reset"
      android_guest_service_log_or_fail \
        "$log_fn" settings-provider-transient \
        "waiting for stable settings provider" || return $?
      if android_guest_service_poll_sleep "$deadline" "$now_fn"; then
        continue
      else
        sleep_status=$?
      fi
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="poll-sleep=failed status=$sleep_status after=settings-provider-transient"
      return "$sleep_status"
    else
      if (( settings_status == 0 )); then
        ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="settings-provider=invalid device_provisioned=noncanonical"
        return 1
      fi
      if (( settings_status == 224 )) \
          && response_diagnostic="$(
            android_guest_service_rejected_response_diagnostic "$output"
          )"; then
        ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="settings-provider=unavailable status=$settings_status $response_diagnostic"
      else
        ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="settings-provider=unavailable status=$settings_status"
      fi
      return "$settings_status"
    fi

    now="$(android_guest_service_read_clock "$now_fn")" || {
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="clock=invalid"
      return 1
    }
    if (( now >= deadline )); then
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="deadline=expired stable_seconds=$elapsed/$ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS"
      return 124
    fi
    if [[ -z "$stable_since" ]]; then
      stable_since="$now"
    fi
    elapsed=$((now - stable_since))
    if (( elapsed >= ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS )); then
      if "$process_guard"; then
        :
      else
        guard_status=$?
        ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="process-guard=failed status=$guard_status phase=admission"
        return "$guard_status"
      fi
      now="$(android_guest_service_read_clock "$now_fn")" || {
        ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="clock=invalid"
        return 1
      }
      if (( now >= deadline )); then
        ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="deadline=expired stable_seconds=$elapsed/$ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS"
        return 124
      fi
      ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="package-manager=stable settings-provider=stable stable_seconds=$elapsed device_provisioned=$provisioned"
      return 0
    fi

    ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="package-manager=stabilizing settings-provider=stabilizing stable_seconds=$elapsed/$ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS device_provisioned=$provisioned"
    android_guest_service_log_or_fail \
      "$log_fn" stabilizing \
      "stabilizing Android guest services ($elapsed/$ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS seconds)" \
      || return $?
    if android_guest_service_poll_sleep "$deadline" "$now_fn"; then
      continue
    else
      sleep_status=$?
    fi
    ANDROID_GUEST_SERVICE_READINESS_LAST_EVIDENCE="poll-sleep=failed status=$sleep_status after=stabilizing"
    return "$sleep_status"
  done
}
