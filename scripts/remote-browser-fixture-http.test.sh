#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"

fail() {
  printf 'remote-browser-fixture-http.test: %s\n' "$*" >&2
  exit 1
}

assert_fails() {
  if "$@" >/dev/null 2>&1; then fail "command unexpectedly succeeded: $*"; fi
}

[[ "$DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_DEFAULT_SECS" == "180" ]] \
  || fail "edge propagation default drift"
[[ "$DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_MAX_SECS" == "600" ]] \
  || fail "edge propagation maximum drift"
[[ "$(DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=600 remote_fixture_edge_propagation_window_secs)" == "600" ]] \
  || fail "edge propagation parser rejected its maximum"
for invalid_window in 0 601 -1 invalid; do
  [[ "$(DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS="$invalid_window" \
    remote_fixture_edge_propagation_window_secs 2>/dev/null)" == "180" ]] \
    || fail "edge propagation parser accepted invalid value $invalid_window"
done
remote_fixture_assert_tunnel_role_url "https://fixture-route.trycloudflare.com/api/node/role"
assert_fails remote_fixture_assert_tunnel_role_url \
  "https://fixture-route.trycloudflare.com/api/node/role?secret=value"
assert_fails remote_fixture_assert_tunnel_role_url \
  "https://fixture.example.invalid/api/node/role"

temporary="$(mktemp -d)"
owned_pid=""
cleanup() {
  [[ -z "$owned_pid" ]] || kill -KILL "$owned_pid" 2>/dev/null || true
  rm -rf -- "$temporary"
}
trap cleanup EXIT

fake_bin="$temporary/fake-bin"
mkdir -p -- "$fake_bin"
cat >"$fake_bin/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
max_time=""
while (($#)); do
  case "$1" in
    --max-time) max_time="${2:?}"; shift 2 ;;
    *) shift ;;
  esac
done
case "${DEVE_REMOTE_FIXTURE_FAKE_CURL_MODE:-status}" in
  stall) sleep "${max_time:?}"; printf '000'; exit 28 ;;
  late-success) sleep "${max_time:?}"; printf '200'; exit 0 ;;
  kill-then-success)
    kill -TERM "${DEVE_REMOTE_FIXTURE_FAKE_CURL_KILL_PID:?}" 2>/dev/null || true
    sleep 0.1
    printf '200'
    exit 0
    ;;
esac
count_file="${DEVE_REMOTE_FIXTURE_FAKE_CURL_COUNT:?}"
count=0
[[ ! -f "$count_file" ]] || count="$(<"$count_file")"
count=$((count + 1))
printf '%s\n' "$count" >"$count_file"
if ((count < ${DEVE_REMOTE_FIXTURE_FAKE_CURL_SUCCEED_AFTER:-999999})); then
  printf '%s' "${DEVE_REMOTE_FIXTURE_FAKE_CURL_STATUS:-530}"
  exit 22
fi
printf '200'
SH
chmod +x "$fake_bin/curl"

probe_count="$temporary/probe.count"
current_token="$(remote_fixture_process_token "$$")"
DEVE_REMOTE_FIXTURE_FAKE_CURL_COUNT="$probe_count" \
DEVE_REMOTE_FIXTURE_FAKE_CURL_SUCCEED_AFTER=2 \
DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=10 \
PATH="$fake_bin:$PATH" remote_fixture_wait_tunnel_http \
  "https://fixture-route.trycloudflare.com/api/node/role" "$$" "$current_token" \
  "$temporary/tunnel.log"
[[ "$(<"$probe_count")" == "2" ]] \
  || fail "tunnel role probe did not condition-wait until exact HTTP success"

if DEVE_REMOTE_FIXTURE_FAKE_CURL_COUNT="$probe_count" \
  DEVE_REMOTE_FIXTURE_FAKE_CURL_SUCCEED_AFTER=1 \
  DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=5 \
  PATH="$fake_bin:$PATH" remote_fixture_wait_tunnel_http \
    "https://fixture-route.trycloudflare.com/api/node/role" "$$" "wrong-$current_token" \
    "$temporary/tunnel.log" >/dev/null 2>&1; then
  fail "tunnel role probe accepted a mismatched process token"
fi

sleep 60 &
owned_pid="$!"
token="$(remote_fixture_process_token "$owned_pid")"
if DEVE_REMOTE_FIXTURE_FAKE_CURL_MODE=kill-then-success \
  DEVE_REMOTE_FIXTURE_FAKE_CURL_KILL_PID="$owned_pid" \
  DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=5 \
  PATH="$fake_bin:$PATH" remote_fixture_wait_tunnel_http \
    "https://fixture-route.trycloudflare.com/api/node/role" "$owned_pid" "$token" \
    "$temporary/tunnel.log" >/dev/null 2>&1; then
  fail "tunnel role probe accepted HTTP success after process ownership ended"
fi
wait "$owned_pid" 2>/dev/null || true
owned_pid=""

probe_started="$(remote_fixture_now_millis)"
if DEVE_REMOTE_FIXTURE_FAKE_CURL_MODE=stall \
  DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=5 \
  PATH="$fake_bin:$PATH" remote_fixture_wait_tunnel_http \
    "https://fixture-route.trycloudflare.com/api/node/role" "$$" "$current_token" \
    "$temporary/tunnel.log" >/dev/null 2>&1; then
  fail "stalled tunnel role probe unexpectedly passed"
fi
probe_elapsed=$(( $(remote_fixture_now_millis) - probe_started ))
((probe_elapsed < 7000)) \
  || fail "stalled tunnel role probe exceeded its hard deadline: ${probe_elapsed}ms"

if DEVE_REMOTE_FIXTURE_FAKE_CURL_MODE=late-success \
  DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=5 \
  PATH="$fake_bin:$PATH" remote_fixture_wait_tunnel_http \
    "https://fixture-route.trycloudflare.com/api/node/role" "$$" "$current_token" \
    "$temporary/tunnel.log" >/dev/null 2>&1; then
  fail "tunnel role probe accepted a 2xx returned after its deadline"
fi

rm -f -- "$probe_count"
if DEVE_REMOTE_FIXTURE_FAKE_CURL_COUNT="$probe_count" \
  DEVE_REMOTE_FIXTURE_FAKE_CURL_STATUS=530 \
  DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=5 \
  PATH="$fake_bin:$PATH" remote_fixture_wait_tunnel_http \
    "https://fixture-route.trycloudflare.com/api/node/role" "$$" "$current_token" \
    "$temporary/tunnel.log" 2>"$temporary/tunnel-probe.stderr"; then
  fail "unready tunnel route unexpectedly passed the exact role probe"
fi
grep -Fq 'last_status=530' "$temporary/tunnel-probe.stderr" \
  || fail "tunnel failure omitted its allowlisted final HTTP status"
if grep -Fq 'trycloudflare.com' "$temporary/tunnel-probe.stderr"; then
  fail "tunnel failure exposed its ephemeral origin"
fi
if grep -Fq 'response_body=' "$temporary/tunnel-probe.stderr"; then
  fail "tunnel failure exposed a response body"
fi

printf 'remote-browser-fixture-http.test: ok\n'
