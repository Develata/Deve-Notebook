#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/baseline-wrapper.sh
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/docker-remote-import-fixture.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-fixture.sh"
# shellcheck source=scripts/lib/docker-remote-import-edge.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-edge.sh"
# shellcheck source=scripts/lib/docker-remote-import-stable-edge.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-stable-edge.sh"
# shellcheck source=scripts/lib/docker-remote-import-chrome-checkpoint.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-chrome-checkpoint.sh"

COMPOSE_FILE="${DEVE_REMOTE_IMPORT_COMPOSE_FILE:-$ROOT_DIR/docker-compose.remote-import.yml}"
PLAYWRIGHT_PACKAGE="${DEVE_REMOTE_IMPORT_PLAYWRIGHT_PACKAGE:-playwright@1.55.0}"
STATE_FILE="$(remote_import_fixture_state_file)"
STATE_ROOT="$(dirname -- "$STATE_FILE")"
FAILED=0

diagnose() {
  ((FAILED == 1)) || return 0
  printf 'docker-remote-import: collecting diagnostics\n' >&2
  if [[ -n "${DEVE_REMOTE_IMPORT_COMPOSE_FILE:-}" \
    && -n "${DEVE_REMOTE_IMPORT_PROJECT:-}" ]]; then
    remote_import_fixture_compose ps >&2 || true
    remote_import_fixture_compose logs --no-color >&2 || true
  fi
  for log in "$STATE_ROOT"/*.log; do
    [[ -f "$log" ]] || continue
    printf '\n===== %s =====\n' "$(basename -- "$log")" >&2
    tail -n 160 "$log" >&2 || true
  done
}

cleanup_on_exit() {
  local status=$?
  trap - EXIT
  ((status == 0)) || FAILED=1
  remote_import_chrome_checkpoint_cleanup "$STATE_ROOT"
  diagnose
  if ! remote_import_fixture_cleanup "$STATE_FILE"; then
    status=1
  fi
  exit "$status"
}
trap cleanup_on_exit EXIT
trap 'exit 130' INT TERM

for command in docker curl node npm; do
  remote_fixture_require_command "$command"
done
[[ -f "$COMPOSE_FILE" ]] || remote_import_fixture_fail "compose file is missing"
[[ -n "${DEVE_RELEASE_CANDIDATE_IMAGE:-}" ]] \
  || remote_import_fixture_fail "DEVE_RELEASE_CANDIDATE_IMAGE is required"
[[ "${DEVE_RELEASE_CANDIDATE_IMAGE_ID:-}" =~ ^sha256:[0-9a-f]{64}$ ]] \
  || remote_import_fixture_fail "DEVE_RELEASE_CANDIDATE_IMAGE_ID must be an exact image ID"

node --test "$ROOT_DIR/scripts/smoke-docker-remote-import.test.mjs"
bash "$ROOT_DIR/scripts/smoke-docker-remote-import.test.sh"

if [[ -f "$STATE_FILE" ]]; then
  remote_import_fixture_cleanup "$STATE_FILE"
fi
mkdir -p -- "$STATE_ROOT"
chmod 0700 "$STATE_ROOT"

export DEVE_REMOTE_IMPORT_PROJECT="deve-remote-import-$(remote_fixture_random_hex 6)"
export DEVE_REMOTE_IMPORT_COMPOSE_FILE="$COMPOSE_FILE"
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_PORT
DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_PORT="$(remote_fixture_find_free_port)"
export DEVE_REMOTE_IMPORT_WEBDAV_PORT
DEVE_REMOTE_IMPORT_WEBDAV_PORT="$(remote_fixture_find_free_port)"
export DEVE_REMOTE_IMPORT_S3_PORT
DEVE_REMOTE_IMPORT_S3_PORT="$(remote_fixture_find_free_port)"
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_APP_PORT
DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_APP_PORT="$(remote_fixture_find_free_port)"
export DEVE_REMOTE_IMPORT_WEBDAV_APP_PORT
DEVE_REMOTE_IMPORT_WEBDAV_APP_PORT="$(remote_fixture_find_free_port)"
export DEVE_REMOTE_IMPORT_S3_APP_PORT
DEVE_REMOTE_IMPORT_S3_APP_PORT="$(remote_fixture_find_free_port)"
# Compose validates interpolation for every declared service even when only
# provider services are started. Keep candidate-only values non-routable until
# the public-CA tunnel origins are known; candidate services start only after
# these placeholders are replaced below.
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_LOCATOR="webdav+https://pending.invalid/failure/"
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_HOST="pending.invalid"
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_EDGE_IP="127.0.0.1"
export DEVE_REMOTE_IMPORT_WEBDAV_LOCATOR="webdav+https://pending.invalid/root/"
export DEVE_REMOTE_IMPORT_WEBDAV_HOST="pending.invalid"
export DEVE_REMOTE_IMPORT_WEBDAV_EDGE_IP="127.0.0.1"
export DEVE_REMOTE_IMPORT_S3_ORIGIN="https://pending.invalid"
export DEVE_REMOTE_IMPORT_S3_LOCATOR="s3+https://pending.invalid/bucket/prefix"
export DEVE_REMOTE_IMPORT_S3_HOST="pending.invalid"
export DEVE_REMOTE_IMPORT_S3_EDGE_IP="127.0.0.1"

WEBDAV_ROOT="$STATE_ROOT/webdav"
WEBDAV_PREFIX="imports/$(remote_fixture_random_hex 6)"
mkdir -p -- "$WEBDAV_ROOT/$WEBDAV_PREFIX/nested"
printf '%s\n' '# WebDAV provider receipt' 'sealed-before-refresh' \
  >"$WEBDAV_ROOT/$WEBDAV_PREFIX/webdav-sealed.md"
printf '%s\n' '# Nested provider entry' 'backend-owned-diff' \
  >"$WEBDAV_ROOT/$WEBDAV_PREFIX/nested/shared.md"
export DEVE_REMOTE_IMPORT_WEBDAV_FIXTURE
DEVE_REMOTE_IMPORT_WEBDAV_FIXTURE="$(remote_fixture_path_key "$WEBDAV_ROOT")"
export DEVE_REMOTE_IMPORT_WEBDAV_MUTATION_FILE
DEVE_REMOTE_IMPORT_WEBDAV_MUTATION_FILE="$WEBDAV_ROOT/$WEBDAV_PREFIX/webdav-sealed.md"

export DEVE_REMOTE_IMPORT_S3_BUCKET="b6-$(remote_fixture_random_hex 6)"
export DEVE_REMOTE_IMPORT_S3_PREFIX="notebooks/$(remote_fixture_random_hex 6)"
export DEVE_REMOTE_IMPORT_S3_ACCESS_KEY_ID="b6$(remote_fixture_random_hex 12)"
export DEVE_REMOTE_IMPORT_S3_SECRET_ACCESS_KEY
DEVE_REMOTE_IMPORT_S3_SECRET_ACCESS_KEY="$(remote_fixture_random_hex 32)"
export DEVE_REMOTE_IMPORT_AUTH_USER="deve-ci-$(remote_fixture_random_hex 6)"
export DEVE_REMOTE_IMPORT_AUTH_PASSWORD
DEVE_REMOTE_IMPORT_AUTH_PASSWORD="$(remote_fixture_random_hex 18)"
export DEVE_REMOTE_IMPORT_AUTH_SECRET
DEVE_REMOTE_IMPORT_AUTH_SECRET="$(remote_fixture_random_hex 48)"

PASSWORD_FILE="$STATE_ROOT/.auth-password"
AUTH_PASS_FILE="$STATE_ROOT/.auth-pass"
(
  umask 077
  printf '%s\n' "$DEVE_REMOTE_IMPORT_AUTH_PASSWORD" >"$PASSWORD_FILE"
  : >"$AUTH_PASS_FILE"
)
run_deve_baseline "$ROOT_DIR" remote-fixture-password-hash \
  docker-remote-import-password-hash --password-file "$PASSWORD_FILE" >"$AUTH_PASS_FILE"
rm -f -- "$PASSWORD_FILE"
grep -Eq '^\$argon2id\$[^[:space:]]+$' "$AUTH_PASS_FILE" \
  || remote_import_fixture_fail "password hasher did not emit Argon2id PHC"
export DEVE_REMOTE_IMPORT_AUTH_PASS
DEVE_REMOTE_IMPORT_AUTH_PASS="$(<"$AUTH_PASS_FILE")"
rm -f -- "$AUTH_PASS_FILE"

remote_import_fixture_write_state "$STATE_FILE"
remote_import_fixture_verify_candidate

remote_import_fixture_compose up -d webdav-failure webdav minio
remote_import_fixture_wait_url webdav-failure \
  "http://127.0.0.1:$DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_PORT/" 120
remote_import_fixture_wait_url webdav \
  "http://127.0.0.1:$DEVE_REMOTE_IMPORT_WEBDAV_PORT/" 120
remote_import_fixture_wait_url minio \
  "http://127.0.0.1:$DEVE_REMOTE_IMPORT_S3_PORT/minio/health/live" 120
remote_fixture_run_bounded "Remote Import MinIO seed" 180 4194304 \
  "$STATE_ROOT/minio-seed.stdout.log" "$STATE_ROOT/minio-seed.stderr.log" -- \
  "$DEVE_REMOTE_IMPORT_DOCKER_BIN" compose \
    -f "$DEVE_REMOTE_IMPORT_COMPOSE_FILE" -p "$DEVE_REMOTE_IMPORT_PROJECT" \
    run --rm minio-seed

PLAYWRIGHT_ROOT="$STATE_ROOT/playwright"
mkdir -p -- "$PLAYWRIGHT_ROOT"
if [[ ! -f "$PLAYWRIGHT_ROOT/package.json" ]]; then
  printf '{"private":true,"type":"module"}\n' >"$PLAYWRIGHT_ROOT/package.json"
fi
if ! DEVE_REMOTE_IMPORT_PLAYWRIGHT_REQUIRE_FROM="$PLAYWRIGHT_ROOT/package.json" \
  node --input-type=module -e '
    import { createRequire } from "node:module";
    const api = createRequire(process.env.DEVE_REMOTE_IMPORT_PLAYWRIGHT_REQUIRE_FROM)("playwright");
    if (typeof api.chromium?.launch !== "function") process.exit(1);
  ' >/dev/null 2>&1; then
  npm --prefix "$PLAYWRIGHT_ROOT" install --no-audit --no-fund "$PLAYWRIGHT_PACKAGE"
fi
npm --prefix "$PLAYWRIGHT_ROOT" exec -- playwright install chromium
export DEVE_REMOTE_IMPORT_PLAYWRIGHT_REQUIRE_FROM="$PLAYWRIGHT_ROOT/package.json"

CLOUDFLARED="$(remote_fixture_install_cloudflared "$STATE_ROOT")"
remote_import_fixture_start_tunnel webdav_failure \
  "$DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_PORT" "$STATE_FILE" "$CLOUDFLARED"
remote_import_fixture_start_tunnel webdav \
  "$DEVE_REMOTE_IMPORT_WEBDAV_PORT" "$STATE_FILE" "$CLOUDFLARED"
remote_import_fixture_start_tunnel s3 \
  "$DEVE_REMOTE_IMPORT_S3_PORT" "$STATE_FILE" "$CLOUDFLARED"

WEBDAV_FAILURE_EDGE_MAPPING="$(
  remote_import_edge_select_ipv4 webdav-failure \
    "$DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_ORIGIN" \
    "$DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_ORIGIN/$WEBDAV_PREFIX/" PROPFIND
)"
read -r DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_HOST \
  DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_EDGE_IP <<<"$WEBDAV_FAILURE_EDGE_MAPPING"
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_HOST
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_EDGE_IP
WEBDAV_EDGE_MAPPING="$(
  remote_import_edge_select_ipv4 webdav \
    "$DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN" \
    "$DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN/$WEBDAV_PREFIX/" PROPFIND
)"
read -r DEVE_REMOTE_IMPORT_WEBDAV_HOST DEVE_REMOTE_IMPORT_WEBDAV_EDGE_IP \
  <<<"$WEBDAV_EDGE_MAPPING"
export DEVE_REMOTE_IMPORT_WEBDAV_HOST DEVE_REMOTE_IMPORT_WEBDAV_EDGE_IP
S3_EDGE_MAPPING="$(
  remote_import_edge_select_ipv4 s3 "$DEVE_REMOTE_IMPORT_S3_ORIGIN" \
    "$DEVE_REMOTE_IMPORT_S3_ORIGIN/minio/health/live" GET
)"
read -r DEVE_REMOTE_IMPORT_S3_HOST DEVE_REMOTE_IMPORT_S3_EDGE_IP \
  <<<"$S3_EDGE_MAPPING"
export DEVE_REMOTE_IMPORT_S3_HOST DEVE_REMOTE_IMPORT_S3_EDGE_IP

export DEVE_REMOTE_IMPORT_WEBDAV_LOCATOR="webdav+$DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN/$WEBDAV_PREFIX/"
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_LOCATOR="webdav+$DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_ORIGIN/$WEBDAV_PREFIX/"
export DEVE_REMOTE_IMPORT_S3_LOCATOR="s3+$DEVE_REMOTE_IMPORT_S3_ORIGIN/$DEVE_REMOTE_IMPORT_S3_BUCKET/$DEVE_REMOTE_IMPORT_S3_PREFIX"

remote_import_fixture_compose stop webdav-failure
remote_import_fixture_compose up -d deve-webdav-failure
remote_import_fixture_verify_candidate_container deve-webdav-failure
remote_import_fixture_wait_url webdav-failure-candidate \
  "http://127.0.0.1:$DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_APP_PORT/api/node/role" 180
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_APP_CONTAINER
DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_APP_CONTAINER="$(
  remote_import_fixture_container_id deve-webdav-failure
)"
export DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_BASE_URL="http://127.0.0.1:$DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_APP_PORT"

remote_import_fixture_admit_stable_edge deve-webdav webdav \
  "$DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN" \
  "$DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN/$WEBDAV_PREFIX/" PROPFIND \
  DEVE_REMOTE_IMPORT_WEBDAV_HOST DEVE_REMOTE_IMPORT_WEBDAV_EDGE_IP \
  "http://127.0.0.1:$DEVE_REMOTE_IMPORT_WEBDAV_APP_PORT/api/node/role"

export DEVE_REMOTE_IMPORT_WEBDAV_APP_CONTAINER
DEVE_REMOTE_IMPORT_WEBDAV_APP_CONTAINER="$(remote_import_fixture_container_id deve-webdav)"
export DEVE_REMOTE_IMPORT_WEBDAV_BASE_URL="http://127.0.0.1:$DEVE_REMOTE_IMPORT_WEBDAV_APP_PORT"

node "$ROOT_DIR/scripts/smoke-docker-remote-import.mjs" webdav-failure
remote_import_fixture_stop_tunnel webdav_failure "$STATE_FILE"
node "$ROOT_DIR/scripts/smoke-docker-remote-import.mjs" webdav
remote_import_fixture_stop_tunnel webdav "$STATE_FILE"

remote_import_fixture_admit_stable_edge deve-s3 s3 \
  "$DEVE_REMOTE_IMPORT_S3_ORIGIN" \
  "$DEVE_REMOTE_IMPORT_S3_ORIGIN/minio/health/live" GET \
  DEVE_REMOTE_IMPORT_S3_HOST DEVE_REMOTE_IMPORT_S3_EDGE_IP \
  "http://127.0.0.1:$DEVE_REMOTE_IMPORT_S3_APP_PORT/api/node/role"
export DEVE_REMOTE_IMPORT_S3_APP_CONTAINER
DEVE_REMOTE_IMPORT_S3_APP_CONTAINER="$(remote_import_fixture_container_id deve-s3)"
export DEVE_REMOTE_IMPORT_S3_BASE_URL="http://127.0.0.1:$DEVE_REMOTE_IMPORT_S3_APP_PORT"

node "$ROOT_DIR/scripts/smoke-docker-remote-import.mjs" s3
remote_import_fixture_verify_candidate
remote_import_fixture_verify_candidate_container deve-webdav-failure
remote_import_fixture_verify_candidate_container deve-webdav
remote_import_fixture_verify_candidate_container deve-s3
remote_import_chrome_checkpoint_wait "$STATE_ROOT"
printf 'docker-remote-import: ok\n'
