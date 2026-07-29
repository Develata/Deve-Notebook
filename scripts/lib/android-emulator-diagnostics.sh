#!/usr/bin/env bash
# Failure-only diagnostics for the owned Android emulator orchestration gate.

print_emulator_diagnostics() {
  DIAGNOSTICS_PRINTED=1
  if command -v adb >/dev/null 2>&1; then
    echo "mobile-android-emulator-install-startup-smoke-check: adb devices:"
    adb devices 2>&1 || true
  elif android_tool_path adb >/dev/null 2>&1; then
    echo "mobile-android-emulator-install-startup-smoke-check: adb devices:"
    android_run_tool adb devices 2>&1 || true
  fi
  if command -v adb >/dev/null 2>&1 || android_tool_path adb >/dev/null 2>&1; then
    echo "mobile-android-emulator-install-startup-smoke-check: emulator /data capacity:"
    adb_cmd -s "$EMULATOR_SERIAL" shell "df -k /data" 2>&1 || true
  fi
  if command -v emulator >/dev/null 2>&1 || android_tool_path emulator >/dev/null 2>&1; then
    echo "mobile-android-emulator-install-startup-smoke-check: emulator AVD list:"
    android_run_tool emulator -list-avds 2>&1 || true
  fi
  if [[ -f "$LOG_DIR/avdmanager.log" ]]; then
    echo "mobile-android-emulator-install-startup-smoke-check: avdmanager.log tail:"
    tail -n 120 "$LOG_DIR/avdmanager.log" || true
  fi
  if [[ -f "$LOG_DIR/emulator.log" ]]; then
    echo "mobile-android-emulator-install-startup-smoke-check: emulator.log tail:"
    tail -n 120 "$LOG_DIR/emulator.log" || true
  fi
}
