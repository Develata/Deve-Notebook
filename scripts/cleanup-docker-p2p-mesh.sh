#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOCKER_BIN="${DEVE_DOCKER_BIN:-docker}"
if [[ -n "${DEVE_ACCEPTANCE_PRODUCER_STATE_DIR:-}" ]]; then
  if [[ -n "${DEVE_DOCKER_BIN:-}" && "$DEVE_DOCKER_BIN" != "docker" ]]; then
    echo "docker-p2p-mesh-cleanup: receipt Docker binary override rejected" >&2
    exit 1
  fi
  if [[ -n "${DEVE_DOCKER_P2P_MESH_CLEANUP_TOTAL_SECONDS:-}" \
    || -n "${DEVE_DOCKER_P2P_MESH_CLEANUP_COMPOSE_SECONDS:-}" \
    || -n "${DEVE_DOCKER_P2P_MESH_CLEANUP_PROBE_SECONDS:-}" \
    || -n "${DEVE_DOCKER_P2P_MESH_CLEANUP_KILL_AFTER_SECONDS:-}" ]]; then
    echo "docker-p2p-mesh-cleanup: receipt cleanup budget override rejected" >&2
    exit 1
  fi
  canonical_state_file="$DEVE_ACCEPTANCE_PRODUCER_STATE_DIR/docker-p2p-mesh/fixture-state"
  if [[ -n "${DEVE_DOCKER_P2P_MESH_STATE_FILE:-}" \
    && "$DEVE_DOCKER_P2P_MESH_STATE_FILE" != "$canonical_state_file" ]]; then
    echo "docker-p2p-mesh-cleanup: receipt state override rejected" >&2
    exit 1
  fi
  STATE_FILE="$canonical_state_file"
else
  STATE_FILE="${DEVE_DOCKER_P2P_MESH_STATE_FILE:-${TMPDIR:-/tmp}/deve-docker-p2p-mesh-cleanup/docker-p2p-mesh/fixture-state}"
fi
readonly CLEANUP_TOTAL_SECONDS="${DEVE_DOCKER_P2P_MESH_CLEANUP_TOTAL_SECONDS:-45}"
readonly COMPOSE_TIMEOUT_SECONDS="${DEVE_DOCKER_P2P_MESH_CLEANUP_COMPOSE_SECONDS:-30}"
readonly PROBE_TIMEOUT_SECONDS="${DEVE_DOCKER_P2P_MESH_CLEANUP_PROBE_SECONDS:-5}"
readonly KILL_AFTER_SECONDS="${DEVE_DOCKER_P2P_MESH_CLEANUP_KILL_AFTER_SECONDS:-2}"
for budget in "$CLEANUP_TOTAL_SECONDS" "$COMPOSE_TIMEOUT_SECONDS" \
  "$PROBE_TIMEOUT_SECONDS" "$KILL_AFTER_SECONDS"; do
  [[ "$budget" =~ ^[1-9][0-9]*$ ]] \
    || { echo "docker-p2p-mesh-cleanup: invalid cleanup budget" >&2; exit 1; }
done
(( CLEANUP_TOTAL_SECONDS < 60 )) \
  || { echo "docker-p2p-mesh-cleanup: cleanup deadline must stay below runner timeout" >&2; exit 1; }

docker_bounded() {
  local maximum="$1"
  local now remaining seconds
  shift
  now="$(date +%s)"
  remaining="$(( CLEANUP_DEADLINE - now - KILL_AFTER_SECONDS ))"
  (( remaining > 0 )) || return 124
  seconds="$maximum"
  (( seconds <= remaining )) || seconds="$remaining"
  timeout --kill-after="${KILL_AFTER_SECONDS}s" "$seconds" "$DOCKER_BIN" "$@"
}

validate_identity() {
  local project="$1"
  local compose_file="$2"
  [[ "$project" =~ ^deve-p2p-mesh-[A-Za-z0-9][A-Za-z0-9_.-]*$ ]] \
    || { echo "docker-p2p-mesh-cleanup: invalid project identity" >&2; return 1; }
  [[ "$compose_file" == "$ROOT_DIR/docker-compose.mesh.yml" ]] \
    || { echo "docker-p2p-mesh-cleanup: unexpected compose file" >&2; return 1; }
}

validate_cookie_paths() {
  local cookie_a="$1"
  local cookie_b="$2"
  local state_dir="$3"
  [[ "$cookie_a" == "$state_dir/peer-a.cookie" ]] \
    || { echo "docker-p2p-mesh-cleanup: unexpected peer-a cookie path" >&2; return 1; }
  [[ "$cookie_b" == "$state_dir/peer-b.cookie" ]] \
    || { echo "docker-p2p-mesh-cleanup: unexpected peer-b cookie path" >&2; return 1; }
  [[ "$cookie_a" != "$cookie_b" ]] \
    || { echo "docker-p2p-mesh-cleanup: duplicate cookie paths" >&2; return 1; }
}

write_state() {
  local project="$1"
  local compose_file="$2"
  local cookie_a="$3"
  local cookie_b="$4"
  local state_dir temporary
  validate_identity "$project" "$compose_file"
  state_dir="$(dirname -- "$STATE_FILE")"
  validate_cookie_paths "$cookie_a" "$cookie_b" "$state_dir"
  if [[ -e "$state_dir" || -L "$state_dir" ]]; then
    [[ -d "$state_dir" && ! -L "$state_dir" ]] \
      || { echo "docker-p2p-mesh-cleanup: state directory is not a plain directory" >&2; return 1; }
  else
    (umask 077; mkdir -m 700 -p -- "$state_dir")
  fi
  if [[ -e "$STATE_FILE" || -L "$STATE_FILE" ]]; then
    echo "docker-p2p-mesh-cleanup: state already exists" >&2
    return 1
  fi
  temporary="$(umask 077; mktemp "$state_dir/.fixture-state.tmp.XXXXXX")"
  [[ -f "$temporary" && ! -L "$temporary" ]] \
    || { echo "docker-p2p-mesh-cleanup: state temporary is not a plain file" >&2; return 1; }
  if ! (umask 077; printf 'project=%s\ncompose_file=%s\ncookie_a=%s\ncookie_b=%s\n' \
      "$project" "$compose_file" "$cookie_a" "$cookie_b" >"$temporary"); then
    rm -f -- "$temporary"
    return 1
  fi
  mv -n -- "$temporary" "$STATE_FILE"
  if [[ -e "$temporary" || -L "$temporary" ]]; then
    rm -f -- "$temporary"
    echo "docker-p2p-mesh-cleanup: concurrent state publication rejected" >&2
    return 1
  fi
}

if [[ "${1:-}" == "write" ]]; then
  (( $# == 5 )) \
    || { echo "docker-p2p-mesh-cleanup: write requires project, compose, and cookie paths" >&2; exit 1; }
  write_state "$2" "$3" "$4" "$5"
  exit
fi
(( $# == 0 )) \
  || { echo "docker-p2p-mesh-cleanup: unexpected arguments" >&2; exit 1; }

if [[ ! -e "$STATE_FILE" && ! -L "$STATE_FILE" ]]; then
  exit 0
fi
[[ -f "$STATE_FILE" && ! -L "$STATE_FILE" ]] \
  || { echo "docker-p2p-mesh-cleanup: state is not a plain file" >&2; exit 1; }
mapfile -t state <"$STATE_FILE"
(( ${#state[@]} == 4 )) || { echo "docker-p2p-mesh-cleanup: invalid state field count" >&2; exit 1; }
[[ "${state[0]}" == project=* && "${state[1]}" == compose_file=* \
  && "${state[2]}" == cookie_a=* && "${state[3]}" == cookie_b=* ]] \
  || { echo "docker-p2p-mesh-cleanup: invalid state fields" >&2; exit 1; }
project="${state[0]#project=}"
compose_file="${state[1]#compose_file=}"
cookie_a="${state[2]#cookie_a=}"
cookie_b="${state[3]#cookie_b=}"
state_dir="$(dirname -- "$STATE_FILE")"
validate_identity "$project" "$compose_file"
validate_cookie_paths "$cookie_a" "$cookie_b" "$state_dir"

status=0
for cookie in "$cookie_a" "$cookie_b"; do
  if [[ -e "$cookie" || -L "$cookie" ]]; then
    rm -f -- "$cookie" || status=1
  fi
  if [[ -e "$cookie" || -L "$cookie" ]]; then
    echo "docker-p2p-mesh-cleanup: cookie remains after cleanup" >&2
    status=1
  fi
done

CLEANUP_DEADLINE="$(( $(date +%s) + CLEANUP_TOTAL_SECONDS ))"
AUTH_SECRET=deve_docker_p2p_cleanup_secret_32_bytes \
AUTH_USER=admin \
AUTH_PASS='$argon2id$v=19$m=8,t=1,p=1$Y2xlYW51cA$Y2xlYW51cA' \
DEVE_DOCKER_P2P_MESH_IMAGE=deve-notebook:cleanup-placeholder \
  docker_bounded "$COMPOSE_TIMEOUT_SECONDS" \
    compose -f "$compose_file" -p "$project" down -v --remove-orphans \
    >/dev/null 2>&1 || status=1

for resource in ps network volume; do
  if [[ "$resource" == "ps" ]]; then
    remaining="$(docker_bounded "$PROBE_TIMEOUT_SECONDS" ps -aq \
      --filter "label=com.docker.compose.project=$project")" || {
      echo "docker-p2p-mesh-cleanup: container absence probe failed" >&2
      status=1
      continue
    }
  else
    remaining="$(docker_bounded "$PROBE_TIMEOUT_SECONDS" "$resource" ls -q \
      --filter "label=com.docker.compose.project=$project")" || {
      echo "docker-p2p-mesh-cleanup: $resource absence probe failed" >&2
      status=1
      continue
    }
  fi
  if [[ -n "$remaining" ]]; then
    echo "docker-p2p-mesh-cleanup: $resource resources remain for $project" >&2
    status=1
  fi
done

(( status == 0 )) || exit "$status"
rm -f -- "$STATE_FILE"
echo "docker-p2p-mesh-cleanup: ok"
