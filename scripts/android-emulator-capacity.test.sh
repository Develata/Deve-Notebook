#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-emulator-capacity.sh"

assert_capacity() {
  local expected="$1"
  local fixture="$2"
  local actual=""

  actual="$(printf '%s\n' "$fixture" | parse_android_emulator_data_capacity)"
  [[ "$actual" == "$expected" ]] \
    || {
      echo "android-emulator-capacity-test: expected '$expected', got '$actual'" >&2
      exit 1
    }
}

assert_rejected() {
  local fixture="$1"

  if printf '%s\n' "$fixture" | parse_android_emulator_data_capacity >/dev/null; then
    echo "android-emulator-capacity-test: invalid fixture was accepted" >&2
    exit 1
  fi
}

HEADER="Filesystem 1K-blocks Used Available Use% Mounted on"

assert_capacity \
  "4054752 3210804" \
  "$HEADER
/dev/block/dm-33 4054752 743680 3210804 19% /data"
assert_capacity \
  "4054752 3211608" \
  "$HEADER
/dev/block/dm-33 4054752 742876 3211608 19% /data/user/0"
assert_capacity \
  "4054752 3211608" \
  $'Filesystem 1K-blocks Used Available Use% Mounted on\r\n/dev/block/dm-33 4054752 742876 3211608 19% /data\r'

assert_rejected \
  "$HEADER
/dev/block/dm-33 invalid 743680 3210804 19% /data/user/0"
assert_rejected \
  "$HEADER
/dev/block/dm-33 4054752 743680 3210804 19% /system"
assert_rejected \
  "$HEADER
/dev/block/dm-33 3145728 0 4194304 0% /data"
assert_rejected \
  "$HEADER
/dev/block/dm-33 4054752 743680 3210804 19% /data
/dev/block/dm-33 4054752 743680 3210804 19% /data/user/0"

echo "android-emulator-capacity-test: ok"
