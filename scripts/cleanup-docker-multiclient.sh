#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOCKER_BIN="${DEVE_DOCKER_BIN:-docker}"
STATE_FILE="${DEVE_DOCKER_MULTI_STATE_FILE:-${DEVE_ACCEPTANCE_PRODUCER_STATE_DIR:-${TMPDIR:-/tmp}/deve-docker-multiclient-cleanup}/docker-multiclient/fixture-state}"
readonly CLEANUP_TIMEOUT_SECONDS=45
readonly PROBE_TIMEOUT_SECONDS=10

fail() {
  echo "docker-multiclient-cleanup: $*" >&2
  return 1
}

docker_bounded() {
  local seconds="$1"
  shift
  timeout "$seconds" "$DOCKER_BIN" "$@"
}

[[ -f "$STATE_FILE" ]] || exit 0
mapfile -t state <"$STATE_FILE"
(( ${#state[@]} == 2 )) || fail "state file must contain exactly two fields"
[[ "${state[0]}" == project=* && "${state[1]}" == compose_file=* ]] \
  || fail "state file fields are invalid"
project="${state[0]#project=}"
compose_file="${state[1]#compose_file=}"
[[ "$project" =~ ^deve-multiclient-[A-Za-z0-9][A-Za-z0-9_.-]*$ ]] \
  || fail "refusing cleanup for invalid project identity"
[[ "$compose_file" == "$ROOT_DIR/docker-compose.multiclient.yml" ]] \
  || fail "refusing cleanup for unexpected compose file"

status=0
AUTH_SECRET=deve_docker_multiclient_cleanup_secret \
AUTH_USER=admin \
AUTH_PASS='$argon2id$v=19$m=8,t=1,p=1$Y2xlYW51cA$Y2xlYW51cA' \
DEVE_DOCKER_MULTI_IMAGE=deve-notebook:cleanup-placeholder \
DEVE_DOCKER_MULTI_PORT=3101 \
  docker_bounded "$CLEANUP_TIMEOUT_SECONDS" \
    compose -f "$compose_file" -p "$project" down -v --remove-orphans \
    >/dev/null 2>&1 || status=1

for resource in ps network volume; do
  remaining="$(
    if [[ "$resource" == "ps" ]]; then
      docker_bounded "$PROBE_TIMEOUT_SECONDS" ps -aq \
        --filter "label=com.docker.compose.project=$project"
    else
      docker_bounded "$PROBE_TIMEOUT_SECONDS" "$resource" ls -q \
        --filter "label=com.docker.compose.project=$project"
    fi
  )" || {
    echo "docker-multiclient-cleanup: $resource absence probe failed" >&2
    status=1
    continue
  }
  if [[ -n "$remaining" ]]; then
    echo "docker-multiclient-cleanup: $resource resources remain for $project" >&2
    status=1
  fi
done

(( status == 0 )) || exit "$status"
rm -f -- "$STATE_FILE"
echo "docker-multiclient-cleanup: ok"
