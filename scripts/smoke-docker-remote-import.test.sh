#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/docker-remote-import-fixture.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-fixture.sh"
# shellcheck source=scripts/lib/docker-remote-import-edge.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-edge.sh"
# shellcheck source=scripts/lib/docker-remote-import-chrome-checkpoint.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-chrome-checkpoint.sh"

# The self-test may run inside a live producer. Never let inherited ownership
# identities enter its synthetic state or cleanup paths.
unset DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_PID
unset DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_TOKEN
unset DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_ORIGIN
unset DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_PID
unset DEVE_REMOTE_IMPORT_WEBDAV_TUNNEL_TOKEN
unset DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN
unset DEVE_REMOTE_IMPORT_S3_TUNNEL_PID
unset DEVE_REMOTE_IMPORT_S3_TUNNEL_TOKEN
unset DEVE_REMOTE_IMPORT_S3_ORIGIN

fail() {
  printf 'smoke-docker-remote-import.test: %s\n' "$*" >&2
  exit 1
}

for file in \
  scripts/smoke-docker-remote-import.sh \
  scripts/cleanup-docker-remote-import.sh \
  scripts/lib/docker-remote-import-chrome-checkpoint.sh \
  scripts/lib/docker-remote-import-edge.sh \
  scripts/lib/docker-remote-import-fixture.sh; do
  bash -n "$ROOT_DIR/$file"
done

grep -Fq 'DEVE_ACCEPTANCE_PRODUCER_STATE_DIR' \
  "$ROOT_DIR/scripts/lib/docker-remote-import-fixture.sh" \
  || fail "fixture must use the runner-owned state directory"
grep -Fq 'remote_fixture_stop_pid' \
  "$ROOT_DIR/scripts/lib/docker-remote-import-fixture.sh" \
  || fail "tunnel cleanup must bind process identity"
grep -Fq 'docker-remote-import-absence.sh' \
  "$ROOT_DIR/scripts/lib/docker-remote-import-fixture.sh" \
  || fail "cleanup must delegate bounded Docker absence verification"
grep -Fq 'DEVE_RELEASE_CANDIDATE_IMAGE_ID' \
  "$ROOT_DIR/scripts/smoke-docker-remote-import.sh" \
  || fail "producer must bind the exact candidate image ID"
# The acceptance runner grants one finally step 60 seconds. Three tunnel
# TERM/KILL paths and two bounded-command termination paths must fit beneath it.
cleanup_worst_case_seconds=$((
  3 * 7
  + DEVE_REMOTE_IMPORT_DOCKER_CLEANUP_TIMEOUT_SECONDS + 5
  + DEVE_REMOTE_IMPORT_DOCKER_ABSENCE_TIMEOUT_SECONDS + 5
))
((cleanup_worst_case_seconds < 60)) \
  || fail "cleanup worst-case budget exceeds the acceptance finally timeout"
if remote_import_edge_ipv4_candidates \
  "https://unowned.example.invalid" >/dev/null 2>&1; then
  fail "DoH resolver accepted an unowned tunnel origin"
fi
if remote_fixture_process_token 2147483647 >/dev/null 2>&1; then
  fail "missing process identity returned a successful token"
fi

temporary="$(mktemp -d)"
cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT
state_file="$temporary/state/fixture-state.json"
export DEVE_REMOTE_IMPORT_PROJECT="deve-remote-import-123456789abc"
export DEVE_REMOTE_IMPORT_COMPOSE_FILE="$ROOT_DIR/docker-compose.remote-import.yml"
remote_import_fixture_write_state "$state_file"
node -e '
const fs = require("node:fs");
const state = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const tunnels = [
  state.webdav_failure_tunnel,
  state.webdav_tunnel,
  state.s3_tunnel,
];
if (
  state.schema !== 1 ||
  state.project !== "deve-remote-import-123456789abc" ||
  tunnels.some((tunnel) => tunnel.pid !== null || tunnel.token !== null)
) {
  process.exit(1);
}
' "$state_file" || fail "atomic fixture state was not valid JSON"
if compgen -G "$state_file.tmp-*" >/dev/null; then
  fail "atomic fixture state left a temporary file"
fi

standalone_state_root="$temporary/standalone-runner-state"
standalone_checkpoint_root="$standalone_state_root/docker-remote-import"
mkdir -p -- "$standalone_checkpoint_root"
printf '%s\n' '{"auth_password":"must-be-removed"}' \
  >"$standalone_checkpoint_root/chrome-checkpoint.json"
printf '%s\n' '{"auth_password":"interrupted-write"}' \
  >"$standalone_checkpoint_root/chrome-checkpoint.json.tmp-123"
printf '%s\n' 'raw-password' >"$standalone_checkpoint_root/.auth-password"
printf '%s\n' 'password-hash' >"$standalone_checkpoint_root/.auth-pass"
touch "$standalone_checkpoint_root/chrome-checkpoint.release"
DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$standalone_state_root" \
  bash "$ROOT_DIR/scripts/cleanup-docker-remote-import.sh"
[[ ! -e "$standalone_checkpoint_root/chrome-checkpoint.json" \
  && ! -e "$standalone_checkpoint_root/chrome-checkpoint.release" \
  && ! -e "$standalone_checkpoint_root/chrome-checkpoint.json.tmp-123" \
  && ! -e "$standalone_checkpoint_root/.auth-password" \
  && ! -e "$standalone_checkpoint_root/.auth-pass" ]] \
  || fail "standalone cleanup retained Chrome credentials without fixture state"

checkpoint_root="$temporary/checkpoint"
mkdir -p -- "$checkpoint_root"
export DEVE_REMOTE_IMPORT_WEBDAV_BASE_URL="http://127.0.0.1:3101"
export DEVE_REMOTE_IMPORT_S3_BASE_URL="http://127.0.0.1:3102"
export DEVE_REMOTE_IMPORT_AUTH_USER="checkpoint-user"
export DEVE_REMOTE_IMPORT_AUTH_PASSWORD="checkpoint-password"
export DEVE_REMOTE_IMPORT_CHROME_CHECKPOINT=0
remote_import_chrome_checkpoint_wait "$checkpoint_root"
[[ ! -e "$checkpoint_root/chrome-checkpoint.json" ]] \
  || fail "disabled Chrome checkpoint wrote state"

export DEVE_REMOTE_IMPORT_CHROME_CHECKPOINT=1
export DEVE_REMOTE_IMPORT_CHROME_WAIT_SECONDS=5
(
  while [[ ! -f "$checkpoint_root/chrome-checkpoint.json" ]]; do
    sleep 0.1
  done
  node -e '
const fs = require("node:fs");
const checkpoint = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
if (checkpoint.webdav_base_url !== "http://127.0.0.1:3101"
  || checkpoint.s3_base_url !== "http://127.0.0.1:3102"
  || checkpoint.auth_user !== "checkpoint-user"
  || checkpoint.auth_password !== "checkpoint-password") {
  process.exit(1);
}
' "$checkpoint_root/chrome-checkpoint.json"
  touch "$checkpoint_root/chrome-checkpoint.release"
) &
remote_import_chrome_checkpoint_wait "$checkpoint_root"
wait
[[ ! -e "$checkpoint_root/chrome-checkpoint.json" \
  && ! -e "$checkpoint_root/chrome-checkpoint.release" ]] \
  || fail "released Chrome checkpoint retained credentials"

export DEVE_REMOTE_IMPORT_CHROME_WAIT_SECONDS=1
if remote_import_chrome_checkpoint_wait "$checkpoint_root" >/dev/null 2>&1; then
  fail "Chrome checkpoint timeout returned success"
fi
[[ ! -e "$checkpoint_root/chrome-checkpoint.json" ]] \
  || fail "timed-out Chrome checkpoint retained credentials"
unset DEVE_REMOTE_IMPORT_CHROME_CHECKPOINT

mkdir -p -- "$temporary/bin"
cat >"$temporary/bin/docker" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$CAPTURE_ARGS"
if [[ "${1:-}" == "compose" && "$*" == *" ps -q "* ]]; then
  printf '%s\n' "fake-container"
  exit 0
fi
if [[ "${1:-}" == "exec" ]]; then
  count=0
  [[ ! -f "$CAPTURE_STATUS_COUNT" ]] || count="$(<"$CAPTURE_STATUS_COUNT")"
  count=$((count + 1))
  printf '%s\n' "$count" >"$CAPTURE_STATUS_COUNT"
  if ((count == 2)); then
    printf '%s' "530"
  else
    printf '%s' "200"
  fi
  exit 0
fi
if [[ "${1:-}" == "run" ]]; then
  : "${CAPTURE_EDGE_PROBE_SUCCESS_AT:=999999}"
  count=0
  [[ ! -f "$CAPTURE_EDGE_PROBE_COUNT" ]] || count="$(<"$CAPTURE_EDGE_PROBE_COUNT")"
  count=$((count + 1))
  printf '%s\n' "$count" >"$CAPTURE_EDGE_PROBE_COUNT"
  if ((count >= CAPTURE_EDGE_PROBE_SUCCESS_AT)); then
    printf '%s' "200"
  else
    printf '%s' "530"
  fi
  exit 0
fi
if [[ "${1:-}" == "rm" || "${1:-}" == "container" ]]; then
  exit 0
fi
if [[ "${1:-}" == "compose" ]]; then
  printf '%s\n' "${DEVE_REMOTE_IMPORT_AUTH_USER:-}" >"$CAPTURE_AUTH_USER"
fi
SH
chmod +x "$temporary/bin/docker"
export CAPTURE_ARGS="$temporary/args"
export CAPTURE_AUTH_USER="$temporary/auth-user"
export CAPTURE_STATUS_COUNT="$temporary/status-count"
export CAPTURE_EDGE_PROBE_COUNT="$temporary/edge-probe-count"
probe_state="$temporary/probe-state"
mkdir -p -- "$probe_state/docker-remote-import"
DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$probe_state" \
  PATH="$temporary/bin:$PATH" remote_import_fixture_wait_container_url \
  deve-webdav test-tunnel https://fixture.invalid/ 5 3 PROPFIND
[[ "$(<"$CAPTURE_STATUS_COUNT")" == "5" ]] \
  || fail "candidate network probe did not reset its stability window"

# Bounded edge-route propagation window: re-sweeps must reuse the exact probe
# acceptance criterion and stay fail-closed once the window is exhausted.
[[ "$(DEVE_REMOTE_IMPORT_EDGE_PROPAGATION_WINDOW_SECS=banana \
  remote_import_edge_propagation_window_secs 2>/dev/null)" == "180" ]] \
  || fail "invalid propagation window must fall back to 180s"
[[ "$(DEVE_REMOTE_IMPORT_EDGE_PROPAGATION_WINDOW_SECS=601 \
  remote_import_edge_propagation_window_secs 2>/dev/null)" == "180" ]] \
  || fail "oversized propagation window must fall back to 180s"
edge_stderr="$temporary/edge-select.stderr"
export DEVE_RELEASE_CANDIDATE_IMAGE_ID="sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
export DEVE_REMOTE_IMPORT_DOCKER_BIN="$temporary/bin/docker"
remote_import_edge_ipv4_candidates() {
  printf '%s\n' "edge-window.trycloudflare.com 198.51.100.7"
}
rm -f -- "$CAPTURE_EDGE_PROBE_COUNT"
export CAPTURE_EDGE_PROBE_SUCCESS_AT=6
edge_mapping="$(
  DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$probe_state" \
  DEVE_REMOTE_IMPORT_EDGE_PROPAGATION_WINDOW_SECS=120 \
  remote_import_edge_select_ipv4 edge-window \
    "https://edge-window.trycloudflare.com" \
    "https://edge-window.trycloudflare.com/probe/" PROPFIND 2>"$edge_stderr"
)" || fail "propagation window did not recover an edge that answered after re-sweep"
[[ "$edge_mapping" == "edge-window.trycloudflare.com 198.51.100.7" ]] \
  || fail "re-sweep selection returned an unexpected edge mapping"
[[ "$(<"$CAPTURE_EDGE_PROBE_COUNT")" == "6" ]] \
  || fail "re-sweep did not preserve the exact bounded probe sequence"
grep -Fq "waiting for edge-window tunnel edge route propagation (sweep 1)" "$edge_stderr" \
  || fail "re-sweep did not report bounded propagation waiting"
rm -f -- "$CAPTURE_EDGE_PROBE_COUNT"
export CAPTURE_EDGE_PROBE_SUCCESS_AT=999999
if DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$probe_state" \
  DEVE_REMOTE_IMPORT_EDGE_PROPAGATION_WINDOW_SECS=0 \
  remote_import_edge_select_ipv4 edge-window \
    "https://edge-window.trycloudflare.com" \
    "https://edge-window.trycloudflare.com/probe/" PROPFIND \
    >/dev/null 2>"$edge_stderr"; then
  fail "exhausted propagation window must remain fail-closed"
fi
[[ "$(<"$CAPTURE_EDGE_PROBE_COUNT")" == "5" ]] \
  || fail "zero propagation window must keep the single-sweep probe budget"
grep -Fq "no tunnel edge passed the exact-candidate PROPFIND probe for edge-window" "$edge_stderr" \
  || fail "window exhaustion lost the fail-closed terminal error"
grep -Fq "waiting for edge-window tunnel edge route propagation" "$edge_stderr" \
  && fail "zero propagation window must not report a re-sweep"
unset CAPTURE_EDGE_PROBE_SUCCESS_AT
unset DEVE_RELEASE_CANDIDATE_IMAGE_ID
unset -f remote_import_edge_ipv4_candidates

PATH="$temporary/bin:$PATH" remote_import_fixture_cleanup "$state_file"
expected_compose="$(
  node -e '
const path = require("node:path");
process.stdout.write(path.resolve(process.argv[1]).replaceAll("\\", "/"));
' "$DEVE_REMOTE_IMPORT_COMPOSE_FILE"
)"
grep -Fq -- \
  "compose -f $expected_compose -p deve-remote-import-123456789abc down --timeout 5 -v --remove-orphans" \
  "$temporary/args" \
  || fail "cleanup compose arguments lost their command boundary"
[[ "$(<"$temporary/auth-user")" == "cleanup" ]] \
  || fail "cleanup compose is missing its placeholder auth user"
grep -Fq -- \
  "network ls --filter label=com.docker.compose.project=deve-remote-import-123456789abc" \
  "$temporary/args" \
  || fail "cleanup did not complete bounded absence verification"

if grep -Eq -- '--(password|secret|access-key)(=|[[:space:]])' \
  "$ROOT_DIR/scripts/smoke-docker-remote-import.sh"; then
  fail "secret-bearing process argv regression"
fi

printf 'smoke-docker-remote-import.test: ok\n'
