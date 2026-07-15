#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-json.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-json.sh"

fail() {
  printf 'remote-browser-fixture.test: %s\n' "$*" >&2
  exit 1
}

assert_fails() {
  if "$@" >/dev/null 2>&1; then
    fail "command unexpectedly succeeded: $*"
  fi
}

[[ "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_VERSION" == "2026.7.2" ]] || fail "cloudflared version drift"
[[ "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_LINUX_AMD64_SHA256" =~ ^[0-9a-f]{64}$ ]] || fail "invalid pinned SHA-256"
remote_fixture_assert_https_origin "https://fixture.example.invalid"
remote_fixture_assert_https_origin "https://fixture.example.invalid:8443"
assert_fails remote_fixture_assert_https_origin "http://fixture.example.invalid"
assert_fails remote_fixture_assert_https_origin "https://fixture.example.invalid/path"
assert_fails remote_fixture_assert_https_origin "https://user@fixture.example.invalid"

temporary="$(mktemp -d)"
owned_pid=""
secondary_pid=""
cleanup() {
  [[ -z "$owned_pid" ]] || kill -KILL "$owned_pid" 2>/dev/null || true
  [[ -z "$secondary_pid" ]] || kill -KILL "$secondary_pid" 2>/dev/null || true
  rm -rf -- "$temporary"
}
trap cleanup EXIT

remote_fixture_write_environment \
  "$temporary/fixture-env.json" \
  "https://fixture.example.invalid" \
  "$temporary/credentials.json" \
  "$temporary/fixture-state.json"
node -e '
const v=require(process.argv[1]);
if(v.https_origin!=="https://fixture.example.invalid" || !v.credentials_file.endsWith("credentials.json") || !v.state_file.endsWith("fixture-state.json")) process.exit(1);
' "$temporary/fixture-env.json"

mkdir -p -- "$temporary/real"
ln -s -- "$temporary/real" "$temporary/link"
if [[ -L "$temporary/link" ]]; then
  assert_fails remote_fixture_canonical_dir "$temporary/link"
else
  printf 'remote-browser-fixture.test: symlink assertion unavailable on this host\n' >&2
fi

printf 'fixture-user\n' >"$temporary/username"
printf 'fixture-password\n' >"$temporary/password"
printf 'fixture-auth-secret\n' >"$temporary/auth-secret"
remote_fixture_write_credentials "$temporary/credentials.json" "$temporary/username" "$temporary/password" "$temporary/auth-secret"
node -e 'const v=require(process.argv[1]);if(v.username!=="fixture-user"||v.password!=="fixture-password"||v.auth_secret!=="fixture-auth-secret")process.exit(1);' "$temporary/credentials.json"
permissions="$(stat -c '%a' "$temporary/credentials.json")"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *) [[ "$permissions" == "600" ]] || fail "credentials permissions are $permissions instead of 600" ;;
esac

sleep 60 &
owned_pid="$!"
token="$(remote_fixture_process_token "$owned_pid")"
assert_fails remote_fixture_stop_pid test "$owned_pid" "wrong-token"
kill -0 "$owned_pid" 2>/dev/null || fail "mismatched token stopped an unowned process"
remote_fixture_stop_pid test "$owned_pid" "$token"
kill -0 "$owned_pid" 2>/dev/null && fail "owned process survived cleanup"
owned_pid=""

remote_fixture_run_bounded "parallel drain test" 5 100000 \
  "$temporary/bounded-drain.stdout" "$temporary/bounded-drain.stderr" -- \
  node -e 'process.stdout.write("o".repeat(32768));process.stderr.write("e".repeat(32768));'
bounded_bytes=$(( $(wc -c <"$temporary/bounded-drain.stdout") + $(wc -c <"$temporary/bounded-drain.stderr") ))
[[ "$bounded_bytes" == "65536" ]] || fail "bounded process did not drain stdout/stderr concurrently"

assert_fails remote_fixture_run_bounded "output limit test" 10 4096 \
  "$temporary/bounded-limit.stdout" "$temporary/bounded-limit.stderr" -- \
  node -e 'setInterval(()=>{process.stdout.write("o".repeat(1024));process.stderr.write("e".repeat(1024));},10);'
bounded_limit_bytes=$(( $(wc -c <"$temporary/bounded-limit.stdout") + $(wc -c <"$temporary/bounded-limit.stderr") ))
((bounded_limit_bytes <= 4096)) || fail "bounded process retained output beyond its cap"

cat >"$temporary/bounded-timeout-tree.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
sleep 60 &
printf '%s\n' "$!" >"$1"
wait
SH
chmod +x "$temporary/bounded-timeout-tree.sh"
assert_fails remote_fixture_run_bounded "timeout tree test" 1 4096 \
  "$temporary/bounded-timeout.stdout" "$temporary/bounded-timeout.stderr" -- \
  "$temporary/bounded-timeout-tree.sh" "$temporary/bounded-grandchild.pid"
[[ -f "$temporary/bounded-grandchild.pid" ]] || fail "timeout tree test did not start its grandchild"
bounded_grandchild_pid="$(<"$temporary/bounded-grandchild.pid")"
sleep 0.2
remote_fixture_pid_active "$bounded_grandchild_pid" && fail "timed-out bounded process left a grandchild alive"

grep -Fq -- '--env-file "$docker_env_file"' "$ROOT_DIR/scripts/remote-browser-fixture.sh" \
  || fail "Docker backend must consume secrets through an env file"
if grep -Eq -- '--env "AUTH_(USER|PASS|SECRET)=' "$ROOT_DIR/scripts/remote-browser-fixture.sh"; then
  fail "secret-bearing Docker argv regression"
fi
grep -Fq -- "serve --port '{port}' --loopback-only" "$ROOT_DIR/scripts/remote-browser-fixture.sh" \
  || fail "executable fixture must use loopback-only release serve"
grep -Fq -- 'remote_fixture_run_bounded "password hasher"' "$ROOT_DIR/scripts/remote-browser-fixture.sh" \
  || fail "password hasher must use bounded process infra"
grep -Fq -- 'remote_fixture_run_bounded "exact-HEAD backend init"' "$ROOT_DIR/scripts/remote-browser-fixture.sh" \
  || fail "backend init must use bounded process infra"
grep -Fq -- '--max-time "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_TIMEOUT_SECONDS"' \
  "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh" \
  || fail "cloudflared download must have a bounded timeout"
grep -Fq -- '--max-filesize "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_LIMIT_BYTES"' \
  "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh" \
  || fail "cloudflared download must have a bounded size"

failure_state="$temporary/failed-start"
mkdir -p -- "$failure_state"
printf '%s' "$(git -C "$ROOT_DIR" rev-parse HEAD)" >"$temporary/head-proof"
printf '#!/usr/bin/env bash\nexit 1\n' >"$temporary/fail-hasher"
printf '#!/usr/bin/env bash\nexit 0\n' >"$temporary/fake-backend"
chmod +x "$temporary/fail-hasher" "$temporary/fake-backend"
assert_fails "$ROOT_DIR/scripts/remote-browser-fixture.sh" start \
  --state-dir "$failure_state" \
  --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
  --backend-executable "$temporary/fake-backend" \
  --backend-head-file "$temporary/head-proof" \
  --password-hasher "$temporary/fail-hasher"
for leaked in .fixture-owner fixture-state.json fixture-env.json credentials.json .username .password .auth-secret .auth-pass .backend.env; do
  [[ ! -e "$failure_state/$leaked" ]] || fail "failed start leaked $leaked"
done

multi_state="$temporary/multi-stop"
mkdir -p -- "$multi_state"
fixture_id="$(remote_fixture_random_hex 16)"
printf '%s' "$fixture_id" >"$multi_state/.fixture-owner"
printf '{}\n' >"$multi_state/credentials.json"
printf '{}\n' >"$multi_state/fixture-env.json"
sleep 60 & owned_pid="$!"
sleep 60 & secondary_pid="$!"
backend_token="$(remote_fixture_process_token "$owned_pid")"
tunnel_token="$(remote_fixture_process_token "$secondary_pid")"
STATE_FILE_VALUE="$multi_state/fixture-state.json" FIXTURE_ID="$fixture_id" \
BACKEND_PID="$owned_pid" BACKEND_TOKEN="$backend_token" TUNNEL_PID="$secondary_pid" \
TUNNEL_TOKEN="$tunnel_token" node <<'NODE'
const fs = require("fs");
const path = process.env.STATE_FILE_VALUE;
fs.writeFileSync(path, JSON.stringify({
  schema: 1, fixture_id: process.env.FIXTURE_ID, expected_head: "a".repeat(40), source_kind: "test",
  https_origin: "https://fixture.example.invalid",
  credentials_file: path.replace(/fixture-state\.json$/, "credentials.json"),
  environment_file: path.replace(/fixture-state\.json$/, "fixture-env.json"),
  backend_pid: Number(process.env.BACKEND_PID), backend_process_token: process.env.BACKEND_TOKEN,
  tunnel_pid: Number(process.env.TUNNEL_PID), tunnel_process_token: `wrong-${process.env.TUNNEL_TOKEN}`,
  container_name: null, created_at: new Date().toISOString(),
}, null, 2) + "\n");
NODE
assert_fails "$ROOT_DIR/scripts/remote-browser-fixture.sh" stop --state-dir "$multi_state"
kill -0 "$secondary_pid" 2>/dev/null || fail "mismatched first resource was unexpectedly stopped"
kill -0 "$owned_pid" 2>/dev/null && fail "later owned backend cleanup was skipped"
owned_pid=""
[[ -e "$multi_state/.fixture-owner" && -e "$multi_state/fixture-state.json" ]] \
  || fail "failed multi-resource cleanup removed ownership state"
[[ ! -e "$multi_state/credentials.json" && ! -e "$multi_state/fixture-env.json" ]] \
  || { sed -n '1,120p' "$multi_state/fixture-state.json" >&2; fail "normal stop did not remove fixed secret files first"; }
kill -KILL "$secondary_pid" 2>/dev/null || true
wait "$secondary_pid" 2>/dev/null || true
secondary_pid=""

fake_bin="$temporary/fake-bin"
mkdir -p -- "$fake_bin"
cat >"$fake_bin/docker" <<'SH'
#!/usr/bin/env bash
case "${FAKE_DOCKER_MODE:-absent}" in
  present) printf 'fixture\n'; exit 0 ;;
  absent) exit 0 ;;
  error) exit 42 ;;
esac
SH
chmod +x "$fake_bin/docker"
if PATH="$fake_bin:$PATH" FAKE_DOCKER_MODE=absent remote_fixture_container_presence fixture; then
  fail "absent container was reported present"
else
  status=$?
  [[ "$status" == "1" ]] || fail "absent container returned $status instead of 1"
fi
PATH="$fake_bin:$PATH" FAKE_DOCKER_MODE=present remote_fixture_container_presence fixture \
  || fail "present container was not detected"
if PATH="$fake_bin:$PATH" FAKE_DOCKER_MODE=error remote_fixture_container_presence fixture >/dev/null 2>&1; then
  fail "Docker inspection error was treated as container absence"
else
  status=$?
  [[ "$status" == "2" ]] || fail "Docker inspection error returned $status instead of 2"
fi

printf 'remote-browser-fixture.test: ok\n'
