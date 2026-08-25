#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/docker-p2p-mesh-diagnostics.sh"

temporary="$(mktemp -d)"
fake_docker="$temporary/docker"
output="$temporary/output"
TOKEN_A="secret-token-a"
TOKEN_B="secret-token-b"
PORT_A=3111
PORT_B=3112
DOCKER_BIN="$fake_docker"
MOCK_LOGS="safe compose log"

cleanup() {
  rm -rf -- "$temporary"
}
trap cleanup EXIT

fail_test() {
  printf 'docker-p2p-mesh-diagnostics.test: %s\n' "$*" >&2
  exit 1
}

cat >"$fake_docker" <<'SCRIPT'
#!/usr/bin/env bash
if [[ "${1:-}" == "stats" ]]; then
  printf 'name=peer-a cpu=1%% memory=10MiB / 1GiB pids=8\n'
  printf 'name=peer-b cpu=2%% memory=11MiB / 1GiB pids=9\n'
  exit 0
fi
exit 97
SCRIPT
chmod +x "$fake_docker"

docker_compose() {
  case "$*" in
    "ps") printf 'compose ps rows\n' ;;
    "ps -q peer-a") printf 'container-a\n' ;;
    "ps -q peer-b") printf 'container-b\n' ;;
    "logs --no-color --tail 160" | "logs --no-color") printf '%s\n' "$MOCK_LOGS" ;;
    *) return 98 ;;
  esac
}

docker_cmd() {
  if [[ "${1:-}" == "inspect" ]]; then
    printf 'name=%s status=running health=healthy restart=0 oom=false exit=0\n' "${*: -1}"
    return 0
  fi
  return 99
}

curl_local() {
  case "${*: -1}" in
    *":3111/"*) printf '{"p2p":{"peer":"a","state":"connected"}}\n' ;;
    *":3112/"*) printf '{"p2p":{"peer":"b","state":"connected"}}\n' ;;
    *) return 7 ;;
  esac
}

docker_p2p_mesh_diagnose 2>"$output" \
  || fail_test "safe diagnostics should complete"
grep -Fq '"peer":"a"' "$output" || fail_test "peer-a node role missing"
grep -Fq '"peer":"b"' "$output" || fail_test "peer-b node role missing"
grep -Fq 'container-a status=running health=healthy restart=0 oom=false exit=0' "$output" \
  || fail_test "peer-a container state missing"
grep -Fq 'name=peer-b cpu=2% memory=11MiB / 1GiB pids=9' "$output" \
  || fail_test "bounded docker stats missing"

MOCK_LOGS=$'Authorization: Bearer bearer-secret\nAUTH_SECRET=secret-value\nordinary diagnostic line'
bounded="$(docker_bounded_compose_logs docker_compose)"
grep -Fq -- "$DOCKER_DIAGNOSTIC_MARKER" <<<"$bounded" \
  || fail_test "bounded compose helper did not publish its fixed-tail marker"
if grep -Fq 'bearer-secret' <<<"$bounded" || grep -Fq 'secret-value' <<<"$bounded"; then
  fail_test "token-like diagnostic values were not redacted"
fi
grep -Fq '<redacted>' <<<"$bounded" \
  || fail_test "redacted diagnostic marker missing"

MOCK_LOGS="$(awk 'BEGIN {
  for (i = 1; i <= 100; i++) {
    for (j = 1; j <= 100; j++) printf "password=x "
    printf "\n"
  }
}')"
expanded_command_output="$temporary/expanded-command.log"
docker_bounded_compose_logs docker_compose >"$expanded_command_output" \
  || fail_test "command diagnostics with expanding secrets should complete"
expanded_command_bytes="$(wc -c <"$expanded_command_output" | tr -d '[:space:]')"
expanded_command_lines="$(wc -l <"$expanded_command_output" | tr -d '[:space:]')"
expanded_command_markers="$(grep -F -c -- "$DOCKER_DIAGNOSTIC_MARKER" "$expanded_command_output" || true)"
(( expanded_command_bytes <= DOCKER_DIAGNOSTIC_MAX_BYTES )) \
  || fail_test "redacted command output exceeded final byte budget: $expanded_command_bytes"
(( expanded_command_lines <= DOCKER_DIAGNOSTIC_MAX_LINES )) \
  || fail_test "redacted command output exceeded final line budget: $expanded_command_lines"
(( expanded_command_markers == 1 )) \
  || fail_test "redacted command output did not contain exactly one bounded-tail marker: $expanded_command_markers"
if grep -Fq 'password=x' "$expanded_command_output"; then
  fail_test "redacted command output retained short secret values"
fi
grep -Fq 'password=<redacted>' "$expanded_command_output" \
  || fail_test "redacted command output did not retain redacted short secrets"

MOCK_LOGS="$(awk 'BEGIN { for (i = 1; i <= 1000; i++) printf "diagnostic-line-%04d\n", i }')"
bounded="$(docker_bounded_compose_logs docker_compose)"
bounded_bytes="$(printf '%s' "$bounded" | wc -c | tr -d '[:space:]')"
bounded_lines="$(printf '%s' "$bounded" | wc -l | tr -d '[:space:]')"
(( bounded_bytes <= DOCKER_DIAGNOSTIC_MAX_BYTES )) \
  || fail_test "bounded compose output exceeded fixed byte budget: $bounded_bytes"
(( bounded_lines <= DOCKER_DIAGNOSTIC_MAX_LINES )) \
  || fail_test "bounded compose output exceeded fixed line budget: $bounded_lines"
grep -Fq 'diagnostic-line-1000' <<<"$bounded" \
  || fail_test "bounded compose output did not retain the diagnostic tail"

file_log="$temporary/fixture.log"
{
  for i in $(seq 1 1000); do
    printf 'file-diagnostic-line-%04d\n' "$i"
  done
  printf '%s\n' 'AUTH_SECRET=file-secret-value'
} >"$file_log"
bounded_file="$(docker_bounded_file_output "$file_log")"
file_bytes="$(printf '%s' "$bounded_file" | wc -c | tr -d '[:space:]')"
file_lines="$(printf '%s' "$bounded_file" | wc -l | tr -d '[:space:]')"
(( file_bytes <= DOCKER_DIAGNOSTIC_MAX_BYTES )) \
  || fail_test "bounded file output exceeded fixed byte budget: $file_bytes"
(( file_lines <= DOCKER_DIAGNOSTIC_MAX_LINES )) \
  || fail_test "bounded file output exceeded fixed line budget: $file_lines"
grep -Fq 'file-diagnostic-line-1000' <<<"$bounded_file" \
  || fail_test "bounded file output did not retain the file tail"
if grep -Fq 'file-secret-value' <<<"$bounded_file"; then
  fail_test "bounded file output leaked token-like content"
fi

expanded_file="$temporary/expanded.log"
awk 'BEGIN {
  for (i = 1; i <= 100; i++) {
    for (j = 1; j <= 100; j++) printf "password=x "
    printf "\n"
  }
}' >"$expanded_file"
expanded_file_output="$temporary/expanded-file-output.log"
docker_bounded_file_output "$expanded_file" >"$expanded_file_output" \
  || fail_test "file diagnostics with expanding secrets should complete"
expanded_file_bytes="$(wc -c <"$expanded_file_output" | tr -d '[:space:]')"
expanded_file_lines="$(wc -l <"$expanded_file_output" | tr -d '[:space:]')"
expanded_file_markers="$(grep -F -c -- "$DOCKER_DIAGNOSTIC_MARKER" "$expanded_file_output" || true)"
(( expanded_file_bytes <= DOCKER_DIAGNOSTIC_MAX_BYTES )) \
  || fail_test "redacted file output exceeded final byte budget: $expanded_file_bytes"
(( expanded_file_lines <= DOCKER_DIAGNOSTIC_MAX_LINES )) \
  || fail_test "redacted file output exceeded final line budget: $expanded_file_lines"
(( expanded_file_markers == 1 )) \
  || fail_test "redacted file output did not contain exactly one bounded-tail marker: $expanded_file_markers"
if grep -Fq 'password=x' "$expanded_file_output"; then
  fail_test "redacted file output retained short secret values"
fi
grep -Fq 'password=<redacted>' "$expanded_file_output" \
  || fail_test "redacted file output did not retain redacted short secrets"

if docker_bounded_command_output docker_compose unexpected >/dev/null 2>&1; then
  fail_test "failed compose command was treated as a passing bounded diagnostic"
fi

MOCK_LOGS="$(awk 'BEGIN {
  print "secret-token-a"
  for (i = 1; i <= 200; i++) printf "post-token-line-%03d\n", i
}')"
if docker_stream_parse_command token-scan \
  --token "$TOKEN_A" --token "$TOKEN_B" -- \
  docker_compose logs --no-color; then
  fail_test "token scanner missed an early token after a complete log stream"
fi

MOCK_LOGS="$(awk 'BEGIN {
  print "Session bound to peer peer-b and repo repo-id"
  for (i = 1; i <= 200; i++) printf "post-evidence-line-%03d\n", i
}')"
mesh_count="$(docker_stream_parse_command mesh-count peer-b repo-id -- docker_compose logs --no-color)" \
  || fail_test "mesh evidence parser rejected a complete stream"
[[ "$mesh_count" == "1" ]] || fail_test "mesh evidence parser missed early evidence"

MOCK_LOGS='{"password":"password-value","token":"token-value","secret":"secret-value","api_key":"api-value"}'
json_redacted="$(docker_bounded_compose_logs docker_compose)"
for secret in password-value token-value secret-value api-value; do
  if grep -Fq "$secret" <<<"$json_redacted"; then
    fail_test "JSON secret value leaked from bounded diagnostics: $secret"
  fi
done
grep -Fq '<redacted>' <<<"$json_redacted" \
  || fail_test "JSON secret fields were not redacted"

MOCK_LOGS='{"authorization":"Bearer json-bearer-secret"}'
json_authorization_redacted="$(docker_bounded_compose_logs docker_compose)"
if grep -Fq 'json-bearer-secret' <<<"$json_authorization_redacted"; then
  fail_test "JSON Authorization bearer leaked from bounded diagnostics"
fi
grep -Fq 'Bearer <redacted>' <<<"$json_authorization_redacted" \
  || fail_test "JSON Authorization bearer was not redacted"

producer_exit_three() {
  printf '%s\n' 'safe producer output'
  return 3
}
if docker_stream_parse_command diagnostic -- producer_exit_three >/dev/null 2>&1; then
  fail_test "producer exit 3 was treated as a passing diagnostic"
else
  producer_failure_status=$?
fi
[[ "$producer_failure_status" -eq "$DOCKER_DIAGNOSTIC_PRODUCER_FAILURE_STATUS" ]] \
  || fail_test "producer exit 3 collided with parser token status: $producer_failure_status"

fake_parser="$temporary/failing-parser"
cat >"$fake_parser" <<'SCRIPT'
#!/usr/bin/env bash
cat >/dev/null
exit 42
SCRIPT
chmod +x "$fake_parser"
if DEVE_DOCKER_LOG_PARSER_PYTHON="$fake_parser" \
  docker_bounded_compose_logs docker_compose >/dev/null 2>&1; then
  fail_test "parser/filter failure was treated as a passing diagnostic"
fi

for smoke in \
  scripts/smoke-docker-p2p-mesh.sh \
  scripts/smoke-docker-multiclient.sh \
  scripts/smoke-docker-remote-import.sh \
  scripts/smoke-docker-release.sh; do
  if [[ "$smoke" == scripts/smoke-docker-p2p-mesh.sh ]]; then
    helper='scripts/lib/docker-p2p-mesh-diagnostics.sh'
  else
    helper='scripts/lib/docker-diagnostics.sh'
  fi
  grep -Fq "$helper" "$ROOT_DIR/$smoke" \
    || fail_test "$smoke does not source the expected bounded diagnostic helper"
  if [[ "$smoke" != scripts/smoke-docker-p2p-mesh.sh ]]; then
    grep -Fq 'docker_compose logs --no-color' "$ROOT_DIR/$smoke" \
      && fail_test "$smoke has an unbounded docker_compose log read"
  fi
  grep -Fq 'remote_import_fixture_compose logs --no-color' "$ROOT_DIR/$smoke" \
    && fail_test "$smoke has an unbounded remote-import compose log read"
  grep -Fq 'MESH_LOGS=' "$ROOT_DIR/$smoke" \
    && fail_test "$smoke passes complete logs through MESH_LOGS"
  grep -Fq 'REMOTE_LOGS=' "$ROOT_DIR/$smoke" \
    && fail_test "$smoke passes complete logs through REMOTE_LOGS"
done
grep -Fq 'cat "$log"' "$ROOT_DIR/scripts/smoke-docker-remote-import.sh" \
  && fail_test "remote-import diagnostics still read the complete log file"
grep -Fq 'docker_bounded_file_output "$log"' "$ROOT_DIR/scripts/smoke-docker-remote-import.sh" \
  || fail_test "remote-import diagnostics do not use the bounded file helper"
grep -Fq 'docker_bounded_command_output docker_cmd logs --no-color' \
  "$ROOT_DIR/scripts/smoke-docker-release.sh" \
  && fail_test "Docker container logs still use the unsupported --no-color option"

MOCK_LOGS="unsafe $TOKEN_A material"
docker_p2p_mesh_diagnose 2>"$output" \
  || fail_test "token-bearing logs should be suppressed without aborting safe diagnostics"
grep -Fq 'compose logs suppressed because token material was detected' "$output" \
  || fail_test "token suppression was not reported"
if grep -Fq "$TOKEN_A" "$output"; then
  fail_test "token material escaped diagnostic suppression"
fi

echo "docker-p2p-mesh-diagnostics.test: ok"
