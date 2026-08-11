#!/usr/bin/env bash
# Bounded, app-specific Android startup exit diagnostics.
#
# Contract:
# - The including script MUST define `android_startup_diag_adb <timeout-secs>
#   <adb-args...>` that runs one adb command under its bounded timeout
#   machinery and returns that command's status.
# - `android_startup_diagnostics_prepare <app_id>` runs immediately before the
#   app launch. It is best-effort: it records a validated device-time marker
#   (non-destructive default) or falls back to clearing the relevant logcat
#   buffers, reports setup failure, and always returns 0 so it can never
#   replace the primary startup result.
# - `android_startup_diagnostics_collect <app_id>` runs only after the caller
#   identifies a startup-process failure (missing/replaced identity or a failed
#   bounded readiness probe). Every command is time bounded, combined output
#   is capped by an explicit byte budget, and the function always returns 0 so
#   a diagnostic failure can never hide the primary process failure.
# - Diagnostics stay app-specific (exit-info, crash buffer, ActivityManager /
#   AndroidRuntime / DEBUG / libc / package lines, app process state). No
#   environment dumps, no credentials, no unbounded dumpsys.

ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES="${ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES:-131072}"
ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS="${ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS:-20}"
ANDROID_STARTUP_DIAG_LOGCAT_MARKER=""
_android_startup_diag_used=0

# Both knobs are clamped to explicit ranges (1 KiB..1 MiB, 1..300 s) so an
# oversized override can never defeat the bounds this module exists to enforce.
_android_startup_diag_validate_config() {
  if ! [[ "$ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES" =~ ^[1-9][0-9]{3,6}$ ]] \
      || (( ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES < 1024 || ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES > 1048576 )); then
    echo "android-startup-diagnostics: invalid output budget; using 131072 bytes" >&2
    ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES=131072
  fi
  if ! [[ "$ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS" =~ ^[1-9][0-9]{0,2}$ ]] \
      || (( ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS > 300 )); then
    echo "android-startup-diagnostics: invalid command timeout; using 20s" >&2
    ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS=20
  fi
}

_android_startup_diag_logcat() {
  local buffers="$1"
  if [[ -n "$ANDROID_STARTUP_DIAG_LOGCAT_MARKER" ]]; then
    android_startup_diag_adb "$ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS" \
      logcat -b "$buffers" -d -v threadtime -T "$ANDROID_STARTUP_DIAG_LOGCAT_MARKER"
    return
  fi
  android_startup_diag_adb "$ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS" \
    logcat -b "$buffers" -d -v threadtime
}

_android_startup_diag_runtime_logcat() {
  local app_id="$1"
  local escaped_app_id
  local native_handoff_category
  escaped_app_id="$(printf '%s' "$app_id" | sed 's/[][\.*^$]/\\&/g')"
  native_handoff_category='(android_initial_webview_admission_invalid|android_initial_webview_admission_timeout|android_initial_webview_admission_cancelled|android_native_cookie_callback_rejected|android_native_cookie_not_retained|android_native_cookie_verification_failed|android_native_cookie_callback_invalid|android_native_cookie_jni_setup_failed|android_native_cookie_callback_already_pending|android_native_cookie_request_id_exhausted|android_native_cookie_callback_channel_closed|android_native_cookie_callback_timeout|android_native_cookie_callback_registry_poisoned|android_native_cookie_webview_dispatch_failed|native_session_handoff_failed)'
  # `grep` exit 1 only means no matching lines; keep real grep errors visible.
  _android_startup_diag_logcat main,system \
    | { grep -E "ActivityManager|AndroidRuntime|DEBUG|libc|$escaped_app_id|deve_mobile initial native session handoff failed closed: $native_handoff_category$" || [[ $? -eq 1 ]]; }
}

# Runs one diagnostic command, truncates its combined output to the remaining
# byte budget, and reports its status. Never returns nonzero.
_android_startup_diag_section() {
  local title="$1"
  shift
  local remaining=$((ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES - _android_startup_diag_used))
  if (( remaining <= 0 )); then
    echo "android-startup-diagnostics: output budget exhausted before section: $title" >&2
    return 0
  fi
  local output="" status=0 captured_bytes
  output="$("$@" 2>&1 | head -c "$remaining")" || status=$?
  captured_bytes="$(printf '%s' "$output" | wc -c | tr -d '[:space:]')"
  _android_startup_diag_used=$((_android_startup_diag_used + captured_bytes))
  echo "android-startup-diagnostics: --- $title (status=$status bytes=$captured_bytes) ---" >&2
  [[ -z "$output" ]] || printf '%s\n' "$output" >&2
  if (( status == 141 )); then
    echo "android-startup-diagnostics: section output truncated at the byte budget (nonfatal): $title" >&2
  elif (( status != 0 )); then
    echo "android-startup-diagnostics: section command failed (nonfatal): $title" >&2
  fi
  return 0
}

android_startup_diagnostics_prepare() {
  local app_id="$1"
  local marker
  _android_startup_diag_validate_config
  ANDROID_STARTUP_DIAG_LOGCAT_MARKER=""
  _android_startup_diag_used=0
  # The strict marker shape guards the later `logcat -T` argument against
  # arbitrary device output. Marker-first keeps ambient device logs intact;
  # buffer clearing is only the fallback isolation mechanism.
  if marker="$(android_startup_diag_adb "$ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS" \
        shell date '+%m-%d %H:%M:%S.000' 2>/dev/null | tr -d '\r')" \
      && [[ "$marker" =~ ^[0-9]{2}-[0-9]{2}\ [0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3}$ ]]; then
    ANDROID_STARTUP_DIAG_LOGCAT_MARKER="$marker"
    return 0
  fi
  if android_startup_diag_adb "$ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS" \
      logcat -b main,system,crash -c >/dev/null 2>&1; then
    echo "android-startup-diagnostics: device-time marker unavailable for $app_id; cleared logcat buffers instead" >&2
    return 0
  fi
  echo "android-startup-diagnostics: diagnostic setup failed (nonfatal); startup exit logs for $app_id may include pre-launch noise" >&2
  return 0
}

android_startup_diagnostics_collect() {
  local app_id="$1"
  _android_startup_diag_validate_config
  _android_startup_diag_used=0
  echo "android-startup-diagnostics: collecting bounded startup process evidence for $app_id" >&2
  _android_startup_diag_section "activity exit-info" \
    android_startup_diag_adb "$ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS" \
    shell dumpsys activity exit-info "$app_id"
  _android_startup_diag_section "crash buffer logcat" \
    _android_startup_diag_logcat crash
  _android_startup_diag_section "recent runtime logcat" \
    _android_startup_diag_runtime_logcat "$app_id"
  _android_startup_diag_section "app process state" \
    android_startup_diag_adb "$ANDROID_STARTUP_DIAG_CMD_TIMEOUT_SECS" \
    shell dumpsys activity processes "$app_id"
  echo "android-startup-diagnostics: done (bytes=$_android_startup_diag_used budget=$ANDROID_STARTUP_DIAG_TOTAL_BUDGET_BYTES)" >&2
  return 0
}
