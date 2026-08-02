#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-start-supervisor.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture-start-supervisor.sh"

fail() {
  printf 'remote-browser-fixture-start-supervisor.test: %s\n' "$*" >&2
  exit 1
}

temporary="$(mktemp -d)"
trap 'rm -rf -- "$temporary"' EXIT

start_fixture_worker() {
  trap 'printf cleaned >"$DEVE_REMOTE_FIXTURE_TEST_CLEANED"; exit 143' TERM
  kill -TERM "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}"
  sleep 0.1
  kill -USR1 "$DEVE_REMOTE_FIXTURE_START_PARENT_PID"
  while :; do sleep 0.1; done
}
stop_fixture() {
  fail "pending pre-admission signal unexpectedly published fixture state"
}
pending_status=0
DEVE_REMOTE_FIXTURE_TEST_CLEANED="$temporary/pending-cleaned" \
  start_fixture --state-dir "$temporary/pending" --expected-head "$(printf 'a%.0s' {1..40})" \
  || pending_status=$?
[[ "$pending_status" == "143" ]] || fail "pending TERM returned $pending_status"
[[ -f "$temporary/pending-cleaned" ]] || fail "pending TERM was not forwarded after readiness"

start_fixture_worker() {
  kill -USR1 "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}"
  kill -TERM "$DEVE_REMOTE_FIXTURE_START_PARENT_PID"
  return 0
}
stop_fixture() {
  [[ "${1:-}" == "--state-dir" && "${2:-}" == "$temporary/published" ]] \
    || fail "successful-publication rollback received the wrong state directory"
  printf stopped >"$temporary/published-stopped"
}
published_status=0
start_fixture --state-dir "$temporary/published" --expected-head "$(printf 'b%.0s' {1..40})" \
  || published_status=$?
[[ "$published_status" == "143" ]] || fail "publication-boundary TERM returned $published_status"
[[ -f "$temporary/published-stopped" ]] \
  || fail "publication-boundary TERM did not roll back successful state"

start_fixture_worker() {
  kill -USR1 "${DEVE_REMOTE_FIXTURE_START_PARENT_PID:?}"
  kill -TERM "$DEVE_REMOTE_FIXTURE_START_PARENT_PID"
  return 0
}
stop_fixture() {
  printf preserved >"$temporary/rollback-state-preserved"
  printf 'rollback-failed\n' >&2
  return 55
}
rollback_status=0
start_fixture --state-dir "$temporary/rollback-failed" \
  --expected-head "$(printf 'c%.0s' {1..40})" \
  2>"$temporary/rollback-failed.stderr" || rollback_status=$?
[[ "$rollback_status" == "143" ]] \
  || fail "rollback failure replaced TERM with status $rollback_status"
grep -Fq 'rollback-failed' "$temporary/rollback-failed.stderr" \
  || fail "rollback failure lost the Stop diagnostic"
grep -Fq 'signal_status=143; cleanup_status=55' "$temporary/rollback-failed.stderr" \
  || fail "rollback failure lost cancellation or cleanup status"
[[ -f "$temporary/rollback-state-preserved" ]] \
  || fail "rollback failure falsely removed ownership state"

printf 'remote-browser-fixture-start-supervisor.test: ok\n'
