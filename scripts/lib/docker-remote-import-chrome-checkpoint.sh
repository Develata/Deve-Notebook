#!/usr/bin/env bash
# shellcheck shell=bash

# Optional local-only checkpoint for Chrome MCP inspection after both automated
# journeys pass. The acceptance producer never enables it by default.

remote_import_chrome_checkpoint_cleanup() {
  local state_root="$1"
  rm -f -- \
    "$state_root/chrome-checkpoint.json" \
    "$state_root/chrome-checkpoint.release" \
    "$state_root/.auth-password" \
    "$state_root/.auth-pass" \
    "$state_root"/chrome-checkpoint.json.tmp-*
}

remote_import_chrome_checkpoint_write() {
  local state_root="$1"
  local checkpoint="$state_root/chrome-checkpoint.json"
  CHROME_CHECKPOINT_FILE="$checkpoint" \
  CHROME_WEBDAV_BASE_URL="$DEVE_REMOTE_IMPORT_WEBDAV_BASE_URL" \
  CHROME_S3_BASE_URL="$DEVE_REMOTE_IMPORT_S3_BASE_URL" \
  CHROME_AUTH_USER="$DEVE_REMOTE_IMPORT_AUTH_USER" \
  CHROME_AUTH_PASSWORD="$DEVE_REMOTE_IMPORT_AUTH_PASSWORD" \
    node -e '
const fs = require("node:fs");
const path = require("node:path");
const target = process.env.CHROME_CHECKPOINT_FILE;
const temporary = `${target}.tmp-${process.pid}-${Date.now()}`;
const payload = {
  webdav_base_url: process.env.CHROME_WEBDAV_BASE_URL,
  s3_base_url: process.env.CHROME_S3_BASE_URL,
  auth_user: process.env.CHROME_AUTH_USER,
  auth_password: process.env.CHROME_AUTH_PASSWORD,
};
let descriptor;
try {
  descriptor = fs.openSync(temporary, "wx", 0o600);
  fs.writeFileSync(descriptor, `${JSON.stringify(payload)}\n`);
  fs.fsyncSync(descriptor);
  fs.closeSync(descriptor);
  descriptor = undefined;
  fs.renameSync(temporary, target);
  fs.chmodSync(target, 0o600);
  if (process.platform !== "win32") {
    const directory = fs.openSync(path.dirname(target), "r");
    try {
      fs.fsyncSync(directory);
    } finally {
      fs.closeSync(directory);
    }
  }
} finally {
  if (descriptor !== undefined) fs.closeSync(descriptor);
  try {
    fs.unlinkSync(temporary);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}
'
}

remote_import_chrome_checkpoint_wait() {
  local state_root="$1"
  [[ "${DEVE_REMOTE_IMPORT_CHROME_CHECKPOINT:-0}" == "1" ]] || return 0
  local timeout_seconds="${DEVE_REMOTE_IMPORT_CHROME_WAIT_SECONDS:-600}"
  [[ "$timeout_seconds" =~ ^[1-9][0-9]*$ ]] \
    && ((timeout_seconds <= 900)) \
    || remote_import_fixture_fail \
      "DEVE_REMOTE_IMPORT_CHROME_WAIT_SECONDS must be between 1 and 900"

  local checkpoint="$state_root/chrome-checkpoint.json"
  local release="$state_root/chrome-checkpoint.release"
  remote_import_chrome_checkpoint_cleanup "$state_root"
  remote_import_chrome_checkpoint_write "$state_root"
  printf 'docker-remote-import: Chrome checkpoint ready at %s\n' "$checkpoint"

  local elapsed=0
  while [[ ! -f "$release" ]] && ((elapsed < timeout_seconds)); do
    sleep 1
    ((elapsed += 1))
  done
  if [[ ! -f "$release" ]]; then
    remote_import_chrome_checkpoint_cleanup "$state_root"
    remote_import_fixture_fail "Chrome checkpoint timed out"
    return 1
  fi
  remote_import_chrome_checkpoint_cleanup "$state_root"
}
