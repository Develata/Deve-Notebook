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
[[ "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_WINDOWS_AMD64_SHA256" =~ ^[0-9a-f]{64}$ ]] || fail "invalid Windows pinned SHA-256"
remote_fixture_assert_https_origin "https://fixture.example.invalid"
remote_fixture_assert_https_origin "https://fixture.example.invalid:8443"
assert_fails remote_fixture_assert_https_origin "http://fixture.example.invalid"
assert_fails remote_fixture_assert_https_origin "https://fixture.example.invalid/path"
assert_fails remote_fixture_assert_https_origin "https://user@fixture.example.invalid"

temporary="$(mktemp -d)"
fixture_start_action="start"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    fixture_start_action="__test-start"
    export DEVE_REMOTE_FIXTURE_TEST_MODE=1
    export DEVE_REMOTE_FIXTURE_TEST_ALLOW_UNGROUPED_BOUNDED=1
    ;;
esac
owned_pid=""
secondary_pid=""
zombie_parent_pid=""
cleanup() {
  [[ -z "$owned_pid" ]] || kill -KILL "$owned_pid" 2>/dev/null || true
  [[ -z "$secondary_pid" ]] || kill -KILL "$secondary_pid" 2>/dev/null || true
  [[ -z "$zombie_parent_pid" ]] || kill -KILL "$zombie_parent_pid" 2>/dev/null || true
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
sleep 0.3
[[ "$(remote_fixture_process_token "$owned_pid")" == "$token" ]] \
  || fail "live process ownership token drifted"
assert_fails remote_fixture_stop_pid test "$owned_pid" "wrong-token"
kill -0 "$owned_pid" 2>/dev/null || fail "mismatched token stopped an unowned process"
remote_fixture_stop_pid test "$owned_pid" "$token"
kill -0 "$owned_pid" 2>/dev/null && fail "owned process survived cleanup"
owned_pid=""

# If the numeric PID changes identity after TERM, bounded cleanup must treat the
# original process as gone and must never escalate KILL to the replacement. A
# Bash loop with an ignored TERM is portable across Linux and Git Bash; Node's
# signal handler is not reliable under MSYS native-process signal translation.
bash -c 'trap "" TERM; while :; do sleep 60; done; :' &
owned_pid="$!"
token="$(remote_fixture_wait_stable_process_token "PID reuse guard" "$owned_pid")"
sleep 0.1
owned_descendant_pid=""
owned_descendant_token=""
for _ in $(seq 1 50); do
  owned_descendant_pid="$(remote_fixture_descendants_deepest "$owned_pid" | head -n 1)"
  if [[ -n "$owned_descendant_pid" ]]; then
    owned_descendant_token="$(remote_fixture_process_token "$owned_descendant_pid" 2>/dev/null || true)"
    [[ -n "$owned_descendant_token" ]] && break
  fi
  sleep 0.02
done
[[ -n "$owned_descendant_token" ]] || fail "PID reuse guard descendant token was unavailable"
token_probe_counter="$temporary/token-probe-counter"
(
  remote_fixture_process_token() {
    local observed_count=0
    [[ ! -f "$token_probe_counter" ]] || observed_count="$(wc -l <"$token_probe_counter")"
    printf 'probe\n' >>"$token_probe_counter"
    if ((observed_count < 2)); then printf '%s\n' "$token"; else printf 'replacement-token\n'; fi
  }
  remote_fixture_stop_pid "PID reuse guard" "$owned_pid" "$token"
)
remote_fixture_pid_active "$owned_pid" \
  || fail "PID reuse guard escalated against the replacement identity"
remote_fixture_stop_owned_job "PID reuse guard cleanup" "$owned_pid" "$token"
remote_fixture_live_pid_matches_token "$owned_descendant_pid" "$owned_descendant_token" \
  && fail "shared cleanup deadline returned with an exact descendant alive"
owned_pid=""

# Process enumeration is part of the ownership proof. A producer failure must
# fail closed before signalling the root, not turn an empty snapshot into a
# successful tree cleanup.
bash -c 'trap "" TERM; while :; do sleep 60; done; :' &
owned_pid="$!"
token="$(remote_fixture_wait_stable_process_token "enumeration failure guard" "$owned_pid")"
process_table_definition="$(declare -f remote_fixture_tokenized_process_table)"
remote_fixture_tokenized_process_table() { return 1; }
enumeration_status=0
remote_fixture_stop_owned_job "enumeration failure guard" "$owned_pid" "$token" \
  >/dev/null 2>&1 || enumeration_status=$?
eval "$process_table_definition"
[[ "$enumeration_status" != 0 ]] || fail "process-table failure was accepted as an empty tree"
remote_fixture_live_pid_matches_token "$owned_pid" "$token" \
  || fail "process-table failure signalled the owned root"
remote_fixture_stop_owned_job "enumeration failure cleanup" "$owned_pid" "$token"
owned_pid=""

# A negative PGID signal is legal only after the same process-table snapshot
# proves root PID == root PGID and that the supervisor is outside the group.
grep -Fq 'local supervisor_pid="$BASHPID"' \
  "$ROOT_DIR/scripts/lib/remote-browser-fixture-process-table.sh" \
  || fail "PGID proof must snapshot the caller PID before command substitution"
bash -c 'trap "" TERM; while :; do sleep 60; done; :' &
owned_pid="$!"
token="$(remote_fixture_wait_stable_process_token "PGID mismatch guard" "$owned_pid")"
pgid_status=0
remote_fixture_stop_bounded_tree "PGID mismatch guard" "$owned_pid" 1 "$token" \
  >/dev/null 2>&1 || pgid_status=$?
[[ "$pgid_status" != 0 ]] || fail "non-isolated root was accepted as a process group"
remote_fixture_live_pid_matches_token "$owned_pid" "$token" \
  || fail "PGID mismatch guard signalled the non-isolated root"
remote_fixture_stop_owned_job "PGID mismatch cleanup" "$owned_pid" "$token"
owned_pid=""

# On hosts with an isolated process group, the leader may exit on TERM while a
# descendant ignores it. Completion requires the exact descendant to be gone,
# not merely the group leader.
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *)
    if command -v setsid >/dev/null 2>&1; then
      group_child_file="$temporary/group-child-pid"
      setsid bash -c '
        trap "exit 0" TERM
        node -e '\''process.on("SIGTERM",()=>{});setInterval(()=>{},1000);'\'' &
        printf "%s" "$!" >"$1"
        wait "$!"
      ' _ "$group_child_file" &
      owned_pid="$!"
      token="$(remote_fixture_wait_stable_process_token "process-group guard" "$owned_pid")"
      for _ in $(seq 1 100); do [[ -s "$group_child_file" ]] && break; sleep 0.01; done
      [[ -s "$group_child_file" ]] || fail "process-group descendant was not published"
      group_child_pid="$(<"$group_child_file")"
      group_child_token="$(remote_fixture_wait_stable_process_token \
        "process-group descendant" "$group_child_pid")"
      remote_fixture_stop_bounded_tree "process-group guard" "$owned_pid" 1 "$token"
      wait "$owned_pid" 2>/dev/null || true
      remote_fixture_live_pid_matches_token "$group_child_pid" "$group_child_token" \
        && fail "process-group cleanup returned with an exact descendant alive"
      owned_pid=""

      dynamic_survivor="$temporary/group-dynamic-survivor.sh"
      dynamic_child="$temporary/group-dynamic-child.sh"
      dynamic_leader="$temporary/group-dynamic-leader.sh"
      cat >"$dynamic_survivor" <<'SH'
#!/usr/bin/env bash
trap '' TERM
while :; do sleep 60; done
SH
      cat >"$dynamic_child" <<'SH'
#!/usr/bin/env bash
trap '"$1" & printf "%s" "$!" >"$2"; exit 0' TERM
printf 'ready' >"$3"
printf '%s' "$BASHPID" >"$4"
while :; do sleep 60; done
SH
      cat >"$dynamic_leader" <<'SH'
#!/usr/bin/env bash
trap ':' TERM
"$1" "$2" "$3" "$4" "$5" &
wait "$!" || true
while :; do sleep 60; done
SH
      chmod +x "$dynamic_survivor" "$dynamic_child" "$dynamic_leader"
      dynamic_spawned_file="$temporary/group-dynamic-spawned.pid"
      dynamic_ready_file="$temporary/group-dynamic-ready"
      dynamic_child_pid_file="$temporary/group-dynamic-child.pid"
      setsid "$dynamic_leader" "$dynamic_child" "$dynamic_survivor" \
        "$dynamic_spawned_file" "$dynamic_ready_file" "$dynamic_child_pid_file" &
      owned_pid="$!"
      token="$(remote_fixture_wait_stable_process_token "dynamic process-group guard" "$owned_pid")"
      for _ in $(seq 1 100); do [[ -s "$dynamic_ready_file" ]] && break; sleep 0.01; done
      [[ -s "$dynamic_ready_file" ]] || fail "dynamic process-group child was not ready"
      remote_fixture_stop_bounded_tree "dynamic process-group guard" "$owned_pid" 1 "$token"
      wait "$owned_pid" 2>/dev/null || true
      [[ -s "$dynamic_spawned_file" ]] || fail "TERM handler did not publish its dynamic descendant"
      dynamic_spawned_pid="$(<"$dynamic_spawned_file")"
      remote_fixture_pid_active "$dynamic_spawned_pid" \
        && fail "dynamic process-group descendant escaped final KILL"
      owned_pid=""

      # A user-space capability does not pin a kernel process-group generation.
      # After the exact leader exits, a nonempty retained PGID must fail closed
      # without signalling it, even when the old identity was pre-bound.
      rm -f -- "$dynamic_ready_file" "$dynamic_spawned_file" "$dynamic_child_pid_file"
      setsid "$dynamic_leader" "$dynamic_child" "$dynamic_survivor" \
        "$dynamic_spawned_file" "$dynamic_ready_file" "$dynamic_child_pid_file" &
      owned_pid="$!"
      token="$(remote_fixture_wait_stable_process_token "pre-bound process-group guard" "$owned_pid")"
      for _ in $(seq 1 100); do [[ -s "$dynamic_ready_file" ]] && break; sleep 0.01; done
      [[ -s "$dynamic_ready_file" ]] || fail "pre-bound process-group child was not ready"
      remote_fixture_bind_isolated_process_group "$owned_pid" "$token"
      kill -KILL "$owned_pid"
      wait "$owned_pid" 2>/dev/null || true
      prebound_status=0
      remote_fixture_stop_bounded_tree "pre-bound process-group guard" "$owned_pid" 1 "$token" \
        >/dev/null 2>&1 || prebound_status=$?
      [[ "$prebound_status" != 0 ]] \
        || fail "pre-bound retained process group was treated as a pinned kernel handle"
      [[ -s "$dynamic_child_pid_file" ]] || fail "pre-bound group did not publish its descendant"
      dynamic_child_pid="$(<"$dynamic_child_pid_file")"
      remote_fixture_pid_active "$dynamic_child_pid" \
        || fail "pre-bound retained process group was signalled after leader exit"
      kill -KILL -- "-$owned_pid" 2>/dev/null || true
      for _ in $(seq 1 100); do
        remote_fixture_pid_active "$dynamic_child_pid" || break
        sleep 0.01
      done
      remote_fixture_pid_active "$dynamic_child_pid" \
        && fail "pre-bound retained process-group test cleanup failed"
      owned_pid=""

      rm -f -- "$dynamic_ready_file" "$dynamic_spawned_file" "$dynamic_child_pid_file"
      setsid "$dynamic_leader" "$dynamic_child" "$dynamic_survivor" \
        "$dynamic_spawned_file" "$dynamic_ready_file" "$dynamic_child_pid_file" &
      owned_pid="$!"
      token="$(remote_fixture_wait_stable_process_token "unbound process-group guard" "$owned_pid")"
      for _ in $(seq 1 100); do [[ -s "$dynamic_ready_file" ]] && break; sleep 0.01; done
      [[ -s "$dynamic_ready_file" ]] || fail "unbound process-group child was not ready"
      remote_fixture_assert_isolated_process_group "$owned_pid"
      kill -KILL "$owned_pid"
      wait "$owned_pid" 2>/dev/null || true
      unbound_member="$(remote_fixture_process_group_members "$owned_pid" | head -n 1)"
      [[ -n "$unbound_member" ]] || fail "unbound retained group lost its test descendant"
      unbound_status=0
      remote_fixture_stop_bounded_tree "unbound process-group guard" "$owned_pid" 1 "$token" \
        >/dev/null 2>&1 || unbound_status=$?
      [[ "$unbound_status" != 0 ]] || fail "unbound retained process group was accepted"
      remote_fixture_pid_active "$unbound_member" \
        || fail "unbound retained process group was signalled"
      kill -KILL -- "-$owned_pid" 2>/dev/null || true
      for _ in $(seq 1 100); do
        remote_fixture_pid_active "$unbound_member" || break
        sleep 0.01
      done
      remote_fixture_pid_active "$unbound_member" \
        && fail "unbound retained process-group test cleanup failed"
      owned_pid=""

      setsid bash -c 'trap "exit 0" TERM; while :; do sleep 60; done; :' &
      owned_pid="$!"
      token="$(remote_fixture_wait_stable_process_token "unbound empty-group guard" "$owned_pid")"
      remote_fixture_assert_isolated_process_group "$owned_pid"
      kill -TERM "$owned_pid"
      wait "$owned_pid" 2>/dev/null || true
      unbound_empty_status=0
      remote_fixture_stop_bounded_tree "unbound empty-group guard" "$owned_pid" 1 "$token" \
        >/dev/null 2>&1 || unbound_empty_status=$?
      [[ "$unbound_empty_status" != 0 ]] \
        || fail "unbound empty process group authorized cleanup"
      owned_pid=""
    fi
    ;;
esac

node -e 'process.on("SIGTERM",()=>{});setInterval(()=>{},1000);' &
owned_pid="$!"
token=""
for _ in $(seq 1 50); do
  token="$(remote_fixture_process_token "$owned_pid" 2>/dev/null)" || token=""
  [[ -n "$token" ]] && break
  sleep 0.1
done
[[ -n "$token" ]] || fail "owned job token was unavailable"
remote_fixture_stop_owned_job "TERM-resistant test" "$owned_pid" "$token"
remote_fixture_pid_active "$owned_pid" && fail "owned job survived bounded cleanup"
owned_pid=""

if [[ "$(uname -s)" == Linux* ]] && command -v python3 >/dev/null 2>&1; then
  if ! PYTHONDONTWRITEBYTECODE=1 python3 - "$REMOTE_FIXTURE_BOUNDED_SUBREAPER" <<'PY'
import importlib.util
import pathlib
import sys
from types import SimpleNamespace

helper_path = pathlib.Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("remote_fixture_subreaper", helper_path)
assert spec is not None and spec.loader is not None
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

tokens = iter(("101", "202"))
module.process_token = lambda _pid: next(tokens)
module.process_parent_pid = lambda _pid: 42
assert module.observed_process_identity(7) is None

module.process_token = lambda _pid: "303"
assert module.observed_process_identity(7) == (42, "303")

module.observed_process_identity = lambda _pid: None
module.os.scandir = lambda _path: [SimpleNamespace(name="7", path="/proc/7")]
module.os.path.exists = lambda _path: True
assert module.descendant_identities(1) == ([], False)
PY
  then
    fail "subreaper ancestry/token bracket or incomplete-scan gate failed closed"
  fi
  spaced_comm_pid_file="$temporary/spaced-comm.pid"
  python3 - "$spaced_comm_pid_file" <<'PY' &
import ctypes
import os
import pathlib
import signal
import sys
import time

ctypes.CDLL(None).prctl(15, b"fixture child x", 0, 0, 0)
pathlib.Path(sys.argv[1]).write_text(str(os.getpid()), encoding="ascii")
signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
time.sleep(30)
PY
  owned_pid="$!"
  for _ in $(seq 1 100); do [[ -s "$spaced_comm_pid_file" ]] && break; sleep 0.01; done
  [[ -s "$spaced_comm_pid_file" ]] || fail "spaced comm process did not publish its PID"
  spaced_comm_pid="$(<"$spaced_comm_pid_file")"
  spaced_comm_token="$(remote_fixture_process_token "$spaced_comm_pid")"
  expected_spaced_comm_token="$(python3 - "$spaced_comm_pid" <<'PY'
import pathlib
import sys

tail = pathlib.Path(f"/proc/{sys.argv[1]}/stat").read_text(encoding="ascii").rsplit(") ", 1)[1].split()
print(tail[19])
PY
)"
  [[ "$spaced_comm_token" == "$expected_spaced_comm_token" ]] \
    || fail "Linux process token was shifted by a spaced comm"
  kill -TERM "$owned_pid"
  wait "$owned_pid" 2>/dev/null || true
  owned_pid=""
fi

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
setsid sleep 60 &
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

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *)
    # A subreaper may bind only the owner PID captured before fork/exec. An
    # already-changed parent must self-clean before it can fork the launcher.
    prebind_completion="$temporary/prebind-parent.released"
    prebind_failure="$temporary/prebind-parent.failed"
    prebind_identity="$temporary/prebind-parent.identity"
    prebind_payload="$temporary/prebind-parent.payload"
    prebind_status=0
    setsid python3 "$REMOTE_FIXTURE_BOUNDED_SUBREAPER" \
      "$prebind_completion" "$prebind_failure" "$prebind_identity" 999999 -- \
      bash -c 'printf ran >"$1"' _ "$prebind_payload" \
      >/dev/null 2>&1 || prebind_status=$?
    [[ "$prebind_status" == 143 ]] \
      || fail "subreaper parent prebind rejection returned $prebind_status instead of 143"
    [[ ! -e "$prebind_identity" && ! -e "$prebind_payload" ]] \
      || fail "subreaper forked a launcher after expected-parent mismatch"

    # A failed process-table probe is not an empty-PGID proof and must keep the
    # private controls fail-closed.
    sleep 0.01 &
    preadmission_probe_pid="$!"
    wait "$preadmission_probe_pid" 2>/dev/null || true
    process_group_members_definition="$(declare -f remote_fixture_process_group_members)"
    remote_fixture_process_group_members() { return 1; }
    preadmission_probe_status=0
    remote_fixture_abort_unadmitted_subreaper \
      "preadmission process-table failure test" "$preadmission_probe_pid" \
      >/dev/null 2>&1 || preadmission_probe_status=$?
    eval "$process_group_members_definition"
    [[ "$preadmission_probe_status" != 0 ]] \
      || fail "preadmission cleanup accepted a failed process-group probe"

    # A transiently unavailable token must terminate the exact retained child
    # before waiting. The unadmitted payload must never run.
    (
      remote_fixture_process_token() { return 1; }
      assert_fails remote_fixture_run_bounded "token unavailable test" 2 4096 \
        "$temporary/bounded-token.stdout" "$temporary/bounded-token.stderr" -- \
        bash -c 'printf ran >"$1"' _ "$temporary/bounded-token-payload-ran"
    )
    [[ ! -e "$temporary/bounded-token-payload-ran" ]] \
      || fail "bounded payload ran before process-token admission"

    cat >"$temporary/bounded-retained-child.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
sleep 60 &
printf '%s\n' "$!" >"$1"
exit 0
SH
    chmod +x "$temporary/bounded-retained-child.sh"
    assert_fails remote_fixture_run_bounded "retained payload child test" 5 4096 \
      "$temporary/bounded-retained.stdout" "$temporary/bounded-retained.stderr" -- \
      "$temporary/bounded-retained-child.sh" "$temporary/bounded-retained-child.pid"
    [[ -f "$temporary/bounded-retained-child.pid" ]] \
      || fail "bounded retained-child test did not publish its child PID"
    bounded_retained_child_pid="$(<"$temporary/bounded-retained-child.pid")"
    sleep 0.2
    remote_fixture_pid_active "$bounded_retained_child_pid" \
      && fail "bounded payload returned while its background child remained alive"

    cat >"$temporary/bounded-launcher-crash.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
setsid sleep 60 &
printf '%s\n' "$!" >"$1"
kill -KILL "$PPID"
exit 0
SH
    chmod +x "$temporary/bounded-launcher-crash.sh"
    assert_fails remote_fixture_run_bounded "bounded launcher crash test" 5 4096 \
      "$temporary/bounded-crash.stdout" "$temporary/bounded-crash.stderr" -- \
      "$temporary/bounded-launcher-crash.sh" "$temporary/bounded-crash-child.pid"
    [[ -f "$temporary/bounded-crash-child.pid" ]] \
      || fail "bounded launcher-crash test did not publish its nested child PID"
    bounded_crash_child="$(<"$temporary/bounded-crash-child.pid")"
    sleep 0.2
    remote_fixture_pid_active "$bounded_crash_child" \
      && fail "bounded subreaper crash path left a nested session alive"

    cat >"$temporary/bounded-self-cleanup.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
trap '' TERM
printf '%s\n' "${DEVE_REMOTE_FIXTURE_BOUNDED_ROOT_PID:?}" >"$1"
setsid sleep 60 &
printf '%s\n' "$!" >"$2"
wait
SH
    chmod +x "$temporary/bounded-self-cleanup.sh"
    stop_tree_definition="$(declare -f remote_fixture_stop_bounded_subreaper_tree)"
    remote_fixture_stop_bounded_subreaper_tree() { return 9; }
    self_cleanup_status=0
    remote_fixture_run_bounded "bounded self-cleanup fallback test" 1 4096 \
      "$temporary/bounded-self-cleanup.stdout" "$temporary/bounded-self-cleanup.stderr" -- \
      "$temporary/bounded-self-cleanup.sh" "$temporary/bounded-self-cleanup-root.pid" \
      "$temporary/bounded-self-cleanup-child.pid" \
      2>"$temporary/bounded-self-cleanup.diagnostic" || self_cleanup_status=$?
    eval "$stop_tree_definition"
    ((self_cleanup_status != 0)) \
      || fail "bounded self-cleanup fallback accepted a timed-out payload"
    grep -Fq "cleanup failed with status 9" "$temporary/bounded-self-cleanup.diagnostic" \
      || fail "bounded self-cleanup fallback lost the shell cleanup failure"
    bounded_self_cleanup_root="$(<"$temporary/bounded-self-cleanup-root.pid")"
    bounded_self_cleanup_child="$(<"$temporary/bounded-self-cleanup-child.pid")"
    remote_fixture_pid_active "$bounded_self_cleanup_root" \
      && fail "bounded self-cleanup fallback left its subreaper root alive"
    remote_fixture_pid_active "$bounded_self_cleanup_child" \
      && fail "bounded self-cleanup fallback left its nested session alive"

    (
      remote_fixture_run_bounded "bounded parent-death cleanup test" 30 4096 \
        "$temporary/bounded-parent-death.stdout" "$temporary/bounded-parent-death.stderr" -- \
        "$temporary/bounded-self-cleanup.sh" "$temporary/bounded-parent-death-root.pid" \
        "$temporary/bounded-parent-death-child.pid"
    ) &
    bounded_parent_owner="$!"
    for _ in $(seq 1 100); do
      [[ -f "$temporary/bounded-parent-death-root.pid" \
        && -f "$temporary/bounded-parent-death-child.pid" ]] && break
      sleep 0.02
    done
    [[ -f "$temporary/bounded-parent-death-root.pid" \
      && -f "$temporary/bounded-parent-death-child.pid" ]] \
      || fail "bounded parent-death cleanup test did not publish its identities"
    bounded_parent_root="$(<"$temporary/bounded-parent-death-root.pid")"
    bounded_parent_child="$(<"$temporary/bounded-parent-death-child.pid")"
    kill -KILL "$bounded_parent_owner"
    wait "$bounded_parent_owner" 2>/dev/null || true
    for _ in $(seq 1 100); do
      if ! remote_fixture_pid_active "$bounded_parent_root" \
        && ! remote_fixture_pid_active "$bounded_parent_child"; then
        break
      fi
      sleep 0.05
    done
    remote_fixture_pid_active "$bounded_parent_root" \
      && fail "bounded parent-death cleanup left its subreaper root alive"
    remote_fixture_pid_active "$bounded_parent_child" \
      && fail "bounded parent-death cleanup left its nested session alive"

    # Preserve the primary overflow category and hard-cap the writers even when
    # exact cleanup reports failure while the owned tree is still live.
    cat >"$temporary/bounded-live-overflow.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
trap '' XFSZ TERM
printf '%s\n' "${DEVE_REMOTE_FIXTURE_BOUNDED_ROOT_PID:?}" >"$1"
printf '%s\n' "$BASHPID" >"$2"
chunk="$(printf 'x%.0s' {1..1024})"
while :; do
  printf '%s' "$chunk" || true
  printf '%s' "$chunk" >&2 || true
done
SH
    chmod +x "$temporary/bounded-live-overflow.sh"
    stop_tree_definition="$(declare -f remote_fixture_stop_bounded_subreaper_tree)"
    eval "$(declare -f remote_fixture_stop_bounded_subreaper_tree \
      | sed '1s/remote_fixture_stop_bounded_subreaper_tree/remote_fixture_stop_bounded_subreaper_tree_actual/')"
    self_cleanup_definition="$(declare -f remote_fixture_request_subreaper_self_cleanup)"
    remote_fixture_stop_bounded_subreaper_tree() { return 9; }
    remote_fixture_request_subreaper_self_cleanup() { return 8; }
    combined_failure_status=0
    remote_fixture_run_bounded "combined bounded failure test" 10 4096 \
      "$temporary/bounded-combined.stdout" "$temporary/bounded-combined.stderr" -- \
      "$temporary/bounded-live-overflow.sh" "$temporary/bounded-combined-leader.pid" \
      "$temporary/bounded-combined-payload.pid" \
      2>"$temporary/bounded-combined.diagnostic" || combined_failure_status=$?
    eval "$stop_tree_definition"
    eval "$self_cleanup_definition"
    ((combined_failure_status != 0)) \
      || fail "combined bounded primary/cleanup failure was accepted"
    grep -Fq "exceeded the combined output limit" "$temporary/bounded-combined.diagnostic" \
      || fail "combined bounded failure lost its primary output-overflow category"
    grep -Fq "cleanup failed with status 9" "$temporary/bounded-combined.diagnostic" \
      || fail "combined bounded failure lost its cleanup category"
    bounded_combined_bytes=$(( $(wc -c <"$temporary/bounded-combined.stdout") + $(wc -c <"$temporary/bounded-combined.stderr") ))
    ((bounded_combined_bytes <= 4096)) \
      || fail "combined bounded failure retained output beyond its cap"
    sleep 0.3
    bounded_combined_later_bytes=$(( $(wc -c <"$temporary/bounded-combined.stdout") + $(wc -c <"$temporary/bounded-combined.stderr") ))
    [[ "$bounded_combined_later_bytes" == "$bounded_combined_bytes" ]] \
      || fail "live cleanup failure allowed bounded output to grow after return"
    bounded_combined_leader="$(<"$temporary/bounded-combined-leader.pid")"
    bounded_combined_token="$(remote_fixture_process_token "$bounded_combined_leader")"
    remote_fixture_stop_bounded_subreaper_tree_actual \
      "combined bounded failure recovery" "$bounded_combined_leader" \
      "$bounded_combined_token" 1
    wait "$bounded_combined_leader" 2>/dev/null || true
    unset -f remote_fixture_stop_bounded_subreaper_tree_actual
    rm -f -- "$temporary"/bounded-combined.stdout.bounded.*

    cat >"$temporary/bounded-cancel-tree.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
trap '' TERM
setsid sleep 60 &
printf '%s\n' "$!" >"$1"
wait
SH
    chmod +x "$temporary/bounded-cancel-tree.sh"
    (
      remote_fixture_run_bounded "bounded cancellation test" 30 4096 \
        "$temporary/bounded-cancel.stdout" "$temporary/bounded-cancel.stderr" -- \
        "$temporary/bounded-cancel-tree.sh" "$temporary/bounded-cancel-child.pid"
    ) &
    bounded_cancel_owner="$!"
    for _ in $(seq 1 100); do
      [[ -f "$temporary/bounded-cancel-child.pid" ]] && break
      sleep 0.02
    done
    [[ -f "$temporary/bounded-cancel-child.pid" ]] \
      || fail "bounded cancellation test did not start its payload tree"
    bounded_cancel_child="$(<"$temporary/bounded-cancel-child.pid")"
    kill -TERM "$bounded_cancel_owner"
    bounded_cancel_status=0
    wait "$bounded_cancel_owner" || bounded_cancel_status=$?
    [[ "$bounded_cancel_status" == 143 ]] \
      || fail "bounded cancellation returned $bounded_cancel_status instead of 143"
    sleep 0.2
    remote_fixture_pid_active "$bounded_cancel_child" \
      && fail "bounded cancellation propagated before reaping its nested session"
    ;;
esac

grep -Fq -- '--env-file "$docker_env_file"' "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-worker.sh" \
  || fail "Docker backend must consume secrets through an env file"
if grep -Eq -- '--env "AUTH_(USER|PASS|SECRET)=' "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-worker.sh"; then
  fail "secret-bearing Docker argv regression"
fi
grep -Fq -- "serve --port '{port}' --loopback-only" "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-worker.sh" \
  || fail "executable fixture must use loopback-only release serve"
grep -Fq -- 'remote_fixture_run_bounded "password hasher"' "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-worker.sh" \
  || fail "password hasher must use bounded process infra"
grep -Fq -- 'remote_fixture_run_bounded "exact-HEAD backend init"' "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-worker.sh" \
  || fail "backend init must use bounded process infra"
grep -Fq -- 'tunnel --no-autoupdate --protocol http2' "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-worker.sh" \
  || fail "quick tunnel must use deterministic HTTP/2 transport"
grep -Fq -- '--max-time "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_TIMEOUT_SECONDS"' \
  "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh" \
  || fail "cloudflared download must have a bounded timeout"
grep -Fq -- '--max-filesize "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_LIMIT_BYTES"' \
  "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh" \
  || fail "cloudflared download must have a bounded size"
grep -Fq -- '[[ -n "$backend_pid" ]] && remote_fixture_live_pid_matches_token "$backend_pid" "$backend_token"' \
  "$ROOT_DIR/scripts/remote-browser-fixture.sh" \
  || fail "final backend cleanup proof must use the token-bound active-process classifier"
grep -Fq -- '[[ -n "$tunnel_pid" ]] && remote_fixture_live_pid_matches_token "$tunnel_pid" "$tunnel_token"' \
  "$ROOT_DIR/scripts/remote-browser-fixture.sh" \
  || fail "final tunnel cleanup proof must use the token-bound active-process classifier"

if [[ -r /proc/self/stat ]] && command -v python3 >/dev/null 2>&1 \
  && python3 -c 'import os, sys; sys.exit(0 if hasattr(os, "fork") else 1)'; then
  zombie_pid_file="$temporary/zombie.pid"
  python3 - "$zombie_pid_file" <<'PY' &
import os
import pathlib
import sys
import time

child = os.fork()
if child == 0:
    os._exit(0)
pathlib.Path(sys.argv[1]).write_text(str(child), encoding="ascii")
time.sleep(30)
PY
  zombie_parent_pid="$!"
  for _ in $(seq 1 50); do
    [[ -s "$zombie_pid_file" ]] || { sleep 0.1; continue; }
    zombie_pid="$(<"$zombie_pid_file")"
    [[ -r "/proc/$zombie_pid/stat" ]] || { sleep 0.1; continue; }
    [[ "$(awk '{print $3}' "/proc/$zombie_pid/stat")" == "Z" ]] && break
    sleep 0.1
  done
  [[ -n "${zombie_pid:-}" && -r "/proc/$zombie_pid/stat" \
    && "$(awk '{print $3}' "/proc/$zombie_pid/stat")" == "Z" ]] \
    || fail "zombie classifier fixture did not reach zombie state"
  kill -0 "$zombie_pid" 2>/dev/null || fail "zombie fixture did not retain a process-table entry"
  remote_fixture_pid_active "$zombie_pid" \
    && fail "zombie process-table entry was classified as an active owned process"
  kill -TERM "$zombie_parent_pid" 2>/dev/null || true
  wait "$zombie_parent_pid" 2>/dev/null || true
  zombie_parent_pid=""
fi

bash "$ROOT_DIR/scripts/remote-browser-fixture-http.test.sh"
bash "$ROOT_DIR/scripts/remote-browser-fixture-start-supervisor.test.sh"

scope_fake_bin="$temporary/scope-fake-bin"
scope_state="$temporary/scope-failed-start"
scope_pids="$temporary/scope-pids"
mkdir -p -- "$scope_fake_bin" "$scope_state" "$scope_pids"
cat >"$scope_fake_bin/backend" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  init) exit 0 ;;
  serve)
    printf '%s\n' "$$" >"${DEVE_REMOTE_FIXTURE_FAKE_PID_DIR:?}/backend.pid"
    exec sleep 60
    ;;
  *) exit 2 ;;
esac
SH
cat >"$scope_fake_bin/password-hasher" <<'SH'
#!/usr/bin/env bash
printf '%s\n' '$argon2id$v=19$m=8,t=1,p=1$YQ$YQ'
SH
cat >"$scope_fake_bin/cloudflared" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$$" >"${DEVE_REMOTE_FIXTURE_FAKE_PID_DIR:?}/tunnel.pid"
printf '%s\n' 'INF https://fixture-scope.trycloudflare.com' >&2
exec sleep 60
SH
cat >"$scope_fake_bin/sha256sum" <<'SH'
#!/usr/bin/env bash
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) checksum='cdb5d4432f6ae1595654a692a51308b69d2bf7af961f5578d9391837cf072df9' ;;
  *) checksum='ec905ea7b7e327ff8abdde8cb64697a2152de74dbcdbf6aec9db8364eb3886cd' ;;
esac
printf '%s  %s\n' "$checksum" "${@: -1}"
SH
cat >"$scope_fake_bin/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
url="${@: -1}"
if [[ "$url" == http://127.0.0.1:* ]]; then
  printf '200'
  exit 0
fi
printf '530'
exit 22
SH
chmod +x "$scope_fake_bin/backend" "$scope_fake_bin/password-hasher" \
  "$scope_fake_bin/cloudflared" "$scope_fake_bin/sha256sum" "$scope_fake_bin/curl"
printf '%s' "$(git -C "$ROOT_DIR" rev-parse HEAD)" >"$temporary/scope-head-proof"

usage_state="$temporary/usage-failed-start"
mkdir -p -- "$usage_state"
override_state="$temporary/override-rejected-start"
mkdir -p -- "$override_state"
override_status=0
DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
  bash "$ROOT_DIR/scripts/remote-browser-fixture.sh" start \
    --state-dir "$override_state" \
    --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
    >"$temporary/override-start.stdout" 2>"$temporary/override-start.stderr" \
    || override_status=$?
[[ "$override_status" != 0 ]] || fail "public start accepted an ambient test override"
grep -Fq 'public fixture start/run rejects synthetic test overrides' \
  "$temporary/override-start.stderr" \
  || fail "public start did not classify the ambient test override"
[[ ! -e "$override_state/.fixture-owner" && ! -e "$override_state/startup-state.json" ]] \
  || fail "ambient test override reached resource admission"

usage_status=0
bash "$ROOT_DIR/scripts/remote-browser-fixture.sh" "$fixture_start_action" \
  --state-dir "$usage_state" \
  --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
  --external-origin "https://fixture.example.invalid" \
  >/dev/null 2>&1 || usage_status=$?
[[ "$usage_status" == "2" ]] || fail "partial external usage returned $usage_status instead of 2"
[[ ! -e "$usage_state/.fixture-owner" && ! -e "$usage_state/fixture-state.json" ]] \
  || fail "usage failure admitted fixture ownership"

scope_status=0
DEVE_REMOTE_FIXTURE_FAKE_PID_DIR="$scope_pids" \
DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=1 \
PATH="$scope_fake_bin:$PATH" \
  bash "$ROOT_DIR/scripts/remote-browser-fixture.sh" "$fixture_start_action" \
    --state-dir "$scope_state" \
    --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
    --backend-executable "$scope_fake_bin/backend" \
    --backend-head-file "$temporary/scope-head-proof" \
    --password-hasher "$scope_fake_bin/password-hasher" \
    --cloudflared-executable "$scope_fake_bin/cloudflared" \
    >"$temporary/scope-start.stdout" 2>"$temporary/scope-start.stderr" || scope_status=$?
if [[ "$scope_status" == "0" ]]; then
  sed -n '1,120p' "$temporary/scope-start.stderr" >&2 || true
  [[ ! -f "$scope_state/fixture-state.json" ]] \
    || node -e 'const v=require(process.argv[1]);console.error(JSON.stringify({source_kind:v.source_kind,has_origin:Boolean(v.https_origin)}));' \
      "$scope_state/fixture-state.json" >&2 || true
  fail "unready tunnel fixture unexpectedly started"
fi
if [[ ! -f "$scope_pids/backend.pid" || ! -f "$scope_pids/tunnel.pid" ]]; then
  sed -n '1,80p' "$temporary/scope-start.stderr" >&2 || true
  fail "failed-start scope fixture did not launch both owned processes"
fi
owned_pid="$(<"$scope_pids/backend.pid")"
secondary_pid="$(<"$scope_pids/tunnel.pid")"
grep -Fq 'last_status=530' "$temporary/scope-start.stderr" \
  || fail "failed-start fixture lost its primary tunnel status"
if grep -Fq 'trycloudflare.com' "$temporary/scope-start.stderr"; then
  fail "failed-start fixture exposed its ephemeral tunnel origin"
fi
if grep -Fq 'unbound variable' "$temporary/scope-start.stderr"; then
  fail "failed-start cleanup ran after its ownership scope was unwound"
fi
remote_fixture_pid_active "$owned_pid" && { sed -n '1,120p' "$temporary/scope-start.stderr" >&2; fail "failed-start fixture left its backend alive"; }
remote_fixture_pid_active "$secondary_pid" && { sed -n '1,120p' "$temporary/scope-start.stderr" >&2; fail "failed-start fixture left its tunnel alive"; }
owned_pid=""
secondary_pid=""
for leaked in .fixture-owner fixture-state.json startup-state.json .startup-admitted .startup-admission-decision fixture-env.json credentials.json .username .password .auth-secret .auth-pass .backend.env; do
  [[ ! -e "$scope_state/$leaked" ]] || fail "failed-start scope cleanup leaked $leaked"
done

signal_state="$temporary/signal-failed-start"
signal_pids="$temporary/signal-pids"
mkdir -p -- "$signal_state" "$signal_pids"
DEVE_REMOTE_FIXTURE_FAKE_PID_DIR="$signal_pids" \
DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=30 \
PATH="$scope_fake_bin:$PATH" \
  bash "$ROOT_DIR/scripts/remote-browser-fixture.sh" "$fixture_start_action" \
    --state-dir "$signal_state" \
    --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
    --backend-executable "$scope_fake_bin/backend" \
    --backend-head-file "$temporary/scope-head-proof" \
    --password-hasher "$scope_fake_bin/password-hasher" \
    --cloudflared-executable "$scope_fake_bin/cloudflared" \
    >"$temporary/signal-start.stdout" 2>"$temporary/signal-start.stderr" &
owned_pid="$!"
for _ in $(seq 1 120); do
  [[ -f "$signal_pids/backend.pid" && -f "$signal_pids/tunnel.pid" ]] && break
  remote_fixture_pid_active "$owned_pid" || break
  sleep 0.1
done
[[ -f "$signal_pids/backend.pid" && -f "$signal_pids/tunnel.pid" ]] \
  || fail "signal fixture did not launch both owned processes"
if ! remote_fixture_pid_active "$owned_pid"; then
  signal_status=0
  wait "$owned_pid" || signal_status=$?
  sed -n '1,120p' "$temporary/signal-start.stderr" >&2 || true
  fail "signal fixture supervisor exited before TERM with status $signal_status"
fi
if ! kill -TERM "$owned_pid"; then
  signal_status=0
  wait "$owned_pid" || signal_status=$?
  sed -n '1,120p' "$temporary/signal-start.stderr" >&2 || true
  fail "signal fixture supervisor raced TERM with status $signal_status"
fi
signal_status=0
wait "$owned_pid" || signal_status=$?
if [[ "$signal_status" != "143" ]]; then
  sed -n '1,120p' "$temporary/signal-start.stderr" >&2 || true
  fail "parent-only TERM returned $signal_status instead of 143"
fi
owned_pid=""
signal_backend_pid="$(<"$signal_pids/backend.pid")"
signal_tunnel_pid="$(<"$signal_pids/tunnel.pid")"
remote_fixture_pid_active "$signal_backend_pid" && { sed -n '1,120p' "$temporary/signal-start.stderr" >&2; fail "parent-only TERM left its backend alive"; }
remote_fixture_pid_active "$signal_tunnel_pid" && { sed -n '1,120p' "$temporary/signal-start.stderr" >&2; fail "parent-only TERM left its tunnel alive"; }
if grep -Fq 'unbound variable' "$temporary/signal-start.stderr"; then
  fail "parent-only TERM unwound ownership scope before cleanup"
fi
for leaked in .fixture-owner fixture-state.json startup-state.json .startup-admitted .startup-admission-decision fixture-env.json credentials.json .username .password .auth-secret .auth-pass .backend.env; do
  [[ ! -e "$signal_state/$leaked" ]] || fail "parent-only TERM cleanup leaked $leaked"
done

# TERM during the spawn-to-journal handoff is deferred until the child PID and
# stable token are durable. Both backend and tunnel windows must then cleanly
# consume the pending signal without orphaning the just-created resource.
for handoff_phase in backend backend-finish tunnel tunnel-finish; do
  handoff_state="$temporary/${handoff_phase}-handoff-state"
  handoff_pids="$temporary/${handoff_phase}-handoff-pids"
  mkdir -p -- "$handoff_state" "$handoff_pids"
  handoff_status=0
  DEVE_REMOTE_FIXTURE_TEST_MODE=1 \
  DEVE_REMOTE_FIXTURE_TEST_RESOURCE_HANDOFF_SIGNAL="$handoff_phase" \
  DEVE_REMOTE_FIXTURE_FAKE_PID_DIR="$handoff_pids" \
  DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=30 \
  PATH="$scope_fake_bin:$PATH" \
    bash "$ROOT_DIR/scripts/remote-browser-fixture.sh" __test-start \
      --state-dir "$handoff_state" \
      --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
      --backend-executable "$scope_fake_bin/backend" \
      --backend-head-file "$temporary/scope-head-proof" \
      --password-hasher "$scope_fake_bin/password-hasher" \
      --cloudflared-executable "$scope_fake_bin/cloudflared" \
      >"$temporary/${handoff_phase}-handoff.stdout" \
      2>"$temporary/${handoff_phase}-handoff.stderr" || handoff_status=$?
  [[ "$handoff_status" == 143 ]] || {
    sed -n '1,120p' "$temporary/${handoff_phase}-handoff.stderr" >&2 || true
    fail "$handoff_phase spawn handoff returned $handoff_status instead of 143"
  }
  [[ -f "$handoff_pids/backend.pid" ]] \
    || fail "$handoff_phase spawn handoff did not create its backend child"
  handoff_backend_pid="$(<"$handoff_pids/backend.pid")"
  remote_fixture_pid_active "$handoff_backend_pid" \
    && fail "$handoff_phase spawn handoff left its backend alive"
  if [[ "$handoff_phase" == tunnel || "$handoff_phase" == tunnel-finish ]]; then
    [[ -f "$handoff_pids/tunnel.pid" ]] \
      || fail "tunnel spawn handoff did not create its tunnel child"
    handoff_tunnel_pid="$(<"$handoff_pids/tunnel.pid")"
    remote_fixture_pid_active "$handoff_tunnel_pid" \
      && fail "tunnel spawn handoff left its tunnel alive"
  fi
  for leaked in .fixture-owner fixture-state.json startup-state.json .startup-admitted \
    .startup-admission-decision \
    fixture-env.json credentials.json .username .password .auth-secret .auth-pass .backend.env; do
    [[ ! -e "$handoff_state/$leaked" ]] \
      || fail "$handoff_phase spawn handoff cleanup leaked $leaked"
  done
done

cleanup_failure_bin="$temporary/cleanup-failure-bin"
cleanup_failure_state="$temporary/cleanup-failure-state"
cleanup_failure_pids="$temporary/cleanup-failure-pids"
mkdir -p -- "$cleanup_failure_bin" "$cleanup_failure_state" "$cleanup_failure_pids"
cat >"$cleanup_failure_bin/rm" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
for argument in "$@"; do
  [[ "$argument" != */credentials.json ]] || exit 55
done
exec "${DEVE_REMOTE_FIXTURE_REAL_RM:?}" "$@"
SH
chmod +x "$cleanup_failure_bin/rm"
cleanup_failure_status=0
DEVE_REMOTE_FIXTURE_REAL_RM="$(command -v rm)" \
DEVE_REMOTE_FIXTURE_FAKE_PID_DIR="$cleanup_failure_pids" \
DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS=1 \
PATH="$cleanup_failure_bin:$scope_fake_bin:$PATH" \
  "$ROOT_DIR/scripts/remote-browser-fixture.sh" "$fixture_start_action" \
    --state-dir "$cleanup_failure_state" \
    --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
    --backend-executable "$scope_fake_bin/backend" \
    --backend-head-file "$temporary/scope-head-proof" \
    --password-hasher "$scope_fake_bin/password-hasher" \
    --cloudflared-executable "$scope_fake_bin/cloudflared" \
    >"$temporary/cleanup-failure.stdout" 2>"$temporary/cleanup-failure.stderr" \
    || cleanup_failure_status=$?
[[ "$cleanup_failure_status" != "0" ]] || fail "cleanup-failure fixture unexpectedly started"
grep -Fq 'last_status=530' "$temporary/cleanup-failure.stderr" \
  || fail "cleanup failure replaced the primary tunnel failure"
grep -Fq 'startup failed and at least one owned resource survived cleanup' \
  "$temporary/cleanup-failure.stderr" \
  || fail "cleanup failure was not reported alongside the primary failure"
[[ -f "$cleanup_failure_state/.fixture-owner" && -f "$cleanup_failure_state/fixture-state.json" ]] \
  || fail "cleanup failure removed ownership state"
cleanup_failure_backend_pid="$(<"$cleanup_failure_pids/backend.pid")"
cleanup_failure_tunnel_pid="$(<"$cleanup_failure_pids/tunnel.pid")"
remote_fixture_pid_active "$cleanup_failure_backend_pid" && fail "cleanup failure left its backend alive"
remote_fixture_pid_active "$cleanup_failure_tunnel_pid" && fail "cleanup failure left its tunnel alive"
"$ROOT_DIR/scripts/remote-browser-fixture.sh" stop --state-dir "$cleanup_failure_state" >/dev/null

failure_state="$temporary/failed-start"
mkdir -p -- "$failure_state"
printf '%s' "$(git -C "$ROOT_DIR" rev-parse HEAD)" >"$temporary/head-proof"
printf '#!/usr/bin/env bash\nexit 1\n' >"$temporary/fail-hasher"
printf '#!/usr/bin/env bash\nexit 0\n' >"$temporary/fake-backend"
chmod +x "$temporary/fail-hasher" "$temporary/fake-backend"
assert_fails "$ROOT_DIR/scripts/remote-browser-fixture.sh" "$fixture_start_action" \
  --state-dir "$failure_state" \
  --expected-head "$(git -C "$ROOT_DIR" rev-parse HEAD)" \
  --backend-executable "$temporary/fake-backend" \
  --backend-head-file "$temporary/head-proof" \
  --password-hasher "$temporary/fail-hasher"
for leaked in .fixture-owner fixture-state.json startup-state.json fixture-env.json credentials.json .username .password .auth-secret .auth-pass .backend.env; do
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
