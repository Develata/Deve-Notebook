#!/usr/bin/env bash
# shellcheck shell=bash

# Fixed schema serializers for the RemoteBrowser fixture. Lifecycle and
# authority decisions stay in the owning wrapper; this module only projects
# already-validated state into private JSON files.

remote_fixture_write_state() {
  local state_file="$1"
  STATE_FILE="$state_file" node <<'NODE'
const fs = require("fs");
const env = process.env;
const nullableNumber = (value) => value ? Number(value) : null;
const state = {
  schema: 1,
  fixture_id: env.FIXTURE_ID,
  expected_head: env.EXPECTED_HEAD,
  source_kind: env.SOURCE_KIND,
  https_origin: env.HTTPS_ORIGIN,
  credentials_file: env.CREDENTIALS_FILE,
  environment_file: env.ENVIRONMENT_FILE,
  backend_pid: nullableNumber(env.BACKEND_PID),
  backend_process_token: env.BACKEND_TOKEN || null,
  tunnel_pid: nullableNumber(env.TUNNEL_PID),
  tunnel_process_token: env.TUNNEL_TOKEN || null,
  container_name: env.CONTAINER_NAME || null,
  created_at: new Date().toISOString(),
};
fs.writeFileSync(env.STATE_FILE, `${JSON.stringify(state, null, 2)}\n`, { mode: 0o600 });
NODE
  chmod 0600 "$state_file"
}

remote_fixture_write_environment() {
  local destination="$1"
  local origin="$2"
  local credentials_file="$3"
  local state_file="$4"
  ENV_DESTINATION="$destination" HTTPS_ORIGIN="$origin" CREDENTIALS_FILE="$credentials_file" STATE_FILE="$state_file" node <<'NODE'
const fs = require("fs");
const env = process.env;
fs.writeFileSync(env.ENV_DESTINATION, `${JSON.stringify({
  https_origin: env.HTTPS_ORIGIN,
  credentials_file: env.CREDENTIALS_FILE,
  state_file: env.STATE_FILE,
}, null, 2)}\n`, { mode: 0o600 });
NODE
  chmod 0600 "$destination"
}
