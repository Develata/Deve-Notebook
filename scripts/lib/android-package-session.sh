#!/usr/bin/env bash

# Owns the package-retention boundary for local physical Android diagnostics.
# Formal target-host runs use mode 0 and keep the existing uninstall cleanup.
# Mode 1 requires an already installed exact package, so the next `adb install
# -r` is an overlay update, and clears test data on every admitted exit without
# turning this convenience path into formal receipt evidence.

android_package_session_validate_mode() {
  local mode="$1"
  [[ "$mode" == "0" || "$mode" == "1" ]] || {
    echo "android-package-session: preserve mode must be 0 or 1" >&2
    return 1
  }
}

android_package_session_validate_app_id() {
  local app_id="$1"
  [[ "$app_id" =~ ^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$ ]] || {
    echo "android-package-session: invalid application id" >&2
    return 1
  }
}

android_package_session_validate_receipt_boundary() {
  local mode="$1"
  local evidence_path="$2"
  android_package_session_validate_mode "$mode" || return 1
  if [[ "$mode" == "1" && -n "$evidence_path" ]]; then
    echo "android-package-session: preserve mode cannot write formal evidence" >&2
    return 1
  fi
}

android_package_session_prepare() {
  local mode="$1"
  local adb_function="$2"
  local app_id="$3"
  local output="" status=0

  android_package_session_validate_mode "$mode" || return 1
  android_package_session_validate_app_id "$app_id" || return 1
  [[ "$mode" == "1" ]] || return 0

  if output="$("$adb_function" shell pm list packages --user 0 "$app_id" 2>/dev/null)"; then
    status=0
  else
    status=$?
  fi
  output="${output//$'\r'/}"
  if (( status != 0 )) || [[ "$output" != "package:$app_id" ]]; then
    echo "android-package-session: preserve mode requires the exact package to be installed" >&2
    return 1
  fi
}

android_package_session_cleanup() {
  local mode="$1"
  local adb_function="$2"
  local app_id="$3"
  local output="" status=0

  android_package_session_validate_mode "$mode" || return 1
  android_package_session_validate_app_id "$app_id" || return 1
  if [[ "$mode" == "0" ]]; then
    if output="$("$adb_function" uninstall "$app_id" 2>&1)"; then
      status=0
    else
      status=$?
    fi
    output="${output//$'\r'/}"
    if (( status != 0 )) || [[ "$output" != "Success" ]]; then
      echo "android-package-session: formal package uninstall failed" >&2
      return 1
    fi
    echo "android-package-session: formal_package_uninstalled=true"
    return 0
  fi

  if output="$("$adb_function" shell pm clear "$app_id" 2>&1)"; then
    status=0
  else
    status=$?
  fi
  output="${output//$'\r'/}"
  if (( status != 0 )) || [[ "$output" != "Success" ]]; then
    echo "android-package-session: preserved package data cleanup failed" >&2
    return 1
  fi
  echo "android-package-session: preserved_package_cleared=true"
}

android_package_session_final_status() {
  local primary_status="$1"
  local cleanup_status="$2"
  if (( primary_status != 0 )); then
    return "$primary_status"
  fi
  return "$cleanup_status"
}
