#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-emulator-pin.sh"

temporary="$(mktemp -d)"
fake="$temporary/emulator"
cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT

fail_test() {
  printf 'android-emulator-pin.test: %s\n' "$*" >&2
  exit 1
}

cat >"$fake" <<'SCRIPT'
#!/usr/bin/env bash
case "${FAKE_MODE:?}" in
  canonical-stdout)
    printf 'Android emulator version 36.6.11.0 (build_id 15507667) (CL:N/A)\n'
    ;;
  canonical-stderr)
    printf 'Android emulator version 36.6.11.0 (build_id 15507667)\n' >&2
    ;;
  prefaced)
    printf 'INFO | delegated qemu probe\n'
    printf 'Android emulator version 36.6.11.0 (build_id 15507667)\n'
    ;;
  nonzero-canonical)
    printf 'Android emulator version 36.6.11.0 (build_id 15507667)\n'
    exit 97
    ;;
  loose-tokens)
    printf 'not-an-emulator expected version 36.6.11.0 after build_id 15507667\n'
    exit 97
    ;;
  wrong-version)
    printf 'Android emulator version 36.6.10.0 (build_id 15507667)\n'
    ;;
  empty)
    ;;
  timeout)
    sleep 3
    ;;
  oversized)
    printf 'Android emulator version 36.6.11.0 (build_id 15507667)\n'
    dd if=/dev/zero bs=2048 count=1 2>/dev/null | tr '\0' x
    ;;
  *)
    exit 98
    ;;
esac
SCRIPT
chmod +x "$fake"

expect_match() {
  local mode="$1"
  FAKE_MODE="$mode" android_emulator_pin_matches "$fake" \
    || fail_test "$mode should match: $ANDROID_EMULATOR_PIN_LAST_PROBE"
}

expect_reject() {
  local mode="$1"
  if FAKE_MODE="$mode" android_emulator_pin_matches "$fake"; then
    fail_test "$mode should fail closed"
  fi
}

expect_match canonical-stdout
expect_match canonical-stderr
expect_match prefaced
expect_match nonzero-canonical
expect_reject loose-tokens
expect_reject wrong-version
expect_reject empty

ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS=1
expect_reject timeout
[[ "$ANDROID_EMULATOR_PIN_LAST_PROBE" == exit=124* ]] \
  || fail_test "timeout result was not recorded: $ANDROID_EMULATOR_PIN_LAST_PROBE"

ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS=15
ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES=1024
expect_reject oversized
[[ "$ANDROID_EMULATOR_PIN_LAST_PROBE" == *"bytes=1025"* ]] \
  || fail_test "oversized result was not bounded: $ANDROID_EMULATOR_PIN_LAST_PROBE"

if android_emulator_pin_matches "$temporary/missing"; then
  fail_test "missing binary should fail closed"
fi

ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS=0
expect_reject canonical-stdout
[[ "$ANDROID_EMULATOR_PIN_LAST_PROBE" == "invalid timeout bound" ]] \
  || fail_test "invalid timeout bound was not reported"

ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS=15
ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES=999999
expect_reject canonical-stdout
[[ "$ANDROID_EMULATOR_PIN_LAST_PROBE" == "invalid output bound" ]] \
  || fail_test "invalid output bound was not reported"

ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES=65536
concurrent_cache="$temporary/concurrent-cache"
mkdir -p "$concurrent_cache"
publisher_pids=()
for publisher in 1 2 3 4; do
  (
    staging="$concurrent_cache/staging.$publisher"
    extracted="$staging/extracted"
    mkdir -p "$extracted/emulator"
    cat >"$extracted/emulator/emulator" <<'SCRIPT'
#!/usr/bin/env bash
printf 'Android emulator version 36.6.11.0 (build_id 15507667)\n'
SCRIPT
    chmod +x "$extracted/emulator/emulator"
    printf '%s\n' "$publisher" >"$extracted/publisher"
    android_emulator_pin_publish_extracted "$concurrent_cache" "$staging" "$extracted" \
      >"$concurrent_cache/result.$publisher"
  ) &
  publisher_pids+=("$!")
done
for publisher_pid in "${publisher_pids[@]}"; do
  wait "$publisher_pid" || fail_test "concurrent publication process failed"
done
published_binary="$concurrent_cache/$ANDROID_EMULATOR_PIN_BUILD_ID/emulator/emulator"
[[ -x "$published_binary" ]] || fail_test "concurrent publication lost the pinned binary"
android_emulator_pin_matches "$published_binary" \
  || fail_test "concurrent publication produced an invalid binary"
for publisher in 1 2 3 4; do
  [[ "$(cat "$concurrent_cache/result.$publisher")" == "$published_binary" ]] \
    || fail_test "publisher $publisher returned a noncanonical cache path"
done
[[ ! -e "$concurrent_cache/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.lock" ]] \
  || fail_test "publication lock leaked after concurrent resolution"

invalid_cache="$temporary/invalid-cache"
invalid_staging="$invalid_cache/staging"
mkdir -p "$invalid_cache/$ANDROID_EMULATOR_PIN_BUILD_ID" \
  "$invalid_staging/extracted/emulator"
printf 'preserve\n' >"$invalid_cache/$ANDROID_EMULATOR_PIN_BUILD_ID/owner-marker"
cp "$published_binary" "$invalid_staging/extracted/emulator/emulator"
if android_emulator_pin_publish_extracted \
    "$invalid_cache" "$invalid_staging" "$invalid_staging/extracted" >/dev/null; then
  fail_test "publisher replaced an existing invalid cache entry"
fi
[[ -f "$invalid_cache/$ANDROID_EMULATOR_PIN_BUILD_ID/owner-marker" ]] \
  || fail_test "publisher deleted an existing cache entry without safe authority"
[[ ! -e "$invalid_cache/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.lock" ]] \
  || fail_test "failed publication leaked the build lock"

interrupt_cache="$temporary/interrupt-cache"
interrupt_staging="$interrupt_cache/staging"
mkdir -p "$interrupt_cache/$ANDROID_EMULATOR_PIN_BUILD_ID/emulator" \
  "$interrupt_staging/extracted/emulator"
cat >"$interrupt_cache/$ANDROID_EMULATOR_PIN_BUILD_ID/emulator/emulator" <<'SCRIPT'
#!/usr/bin/env bash
sleep 10
printf 'Android emulator version 36.6.11.0 (build_id 15507667)\n'
SCRIPT
cat >"$interrupt_staging/extracted/emulator/emulator" <<'SCRIPT'
#!/usr/bin/env bash
printf 'Android emulator version 36.6.11.0 (build_id 15507667)\n'
SCRIPT
chmod +x "$interrupt_cache/$ANDROID_EMULATOR_PIN_BUILD_ID/emulator/emulator" \
  "$interrupt_staging/extracted/emulator/emulator"
ANDROID_EMULATOR_PIN_PROBE_TIMEOUT_SECS=5 \
  android_emulator_pin_publish_extracted \
    "$interrupt_cache" "$interrupt_staging" "$interrupt_staging/extracted" >/dev/null &
interrupt_pid=$!
for _ in {1..100}; do
  [[ -f "$interrupt_cache/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.lock" ]] && break
  sleep 0.05
done
[[ -f "$interrupt_cache/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.lock" ]] \
  || fail_test "interrupt test did not observe the publication lock"
kill -TERM "$interrupt_pid"
if wait "$interrupt_pid"; then
  fail_test "interrupted publication unexpectedly succeeded"
fi
[[ ! -e "$interrupt_cache/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.lock" ]] \
  || fail_test "interrupted publication leaked the build lock"
[[ ! -e "$interrupt_staging" ]] \
  || fail_test "interrupted publication leaked its staging directory"

window_cache="$temporary/acquisition-window-cache"
window_staging="$window_cache/staging"
mkdir -p "$window_staging/extracted/emulator"
cp "$published_binary" "$window_staging/extracted/emulator/emulator"
(
  ln() {
    command ln "$@"
    kill -TERM "$BASHPID"
  }
  android_emulator_pin_publish_extracted \
    "$window_cache" "$window_staging" "$window_staging/extracted" >/dev/null
) &
window_pid=$!
if wait "$window_pid"; then
  fail_test "acquisition-window signal unexpectedly succeeded"
fi
[[ ! -e "$window_cache/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.lock" ]] \
  || fail_test "acquisition-window signal leaked the build lock"
if compgen -G "$window_cache/.publish-$ANDROID_EMULATOR_PIN_BUILD_ID.owner.*" >/dev/null; then
  fail_test "acquisition-window signal leaked the owner file"
fi
[[ ! -e "$window_staging" ]] \
  || fail_test "acquisition-window signal leaked its staging directory"

echo "android-emulator-pin.test: ok"
