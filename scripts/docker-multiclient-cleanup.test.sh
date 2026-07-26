#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
fake_docker="$fixture/docker"
calls="$fixture/calls"

printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'printf "%s\n" "$*" >>"$FAKE_DOCKER_CALLS"' \
  'if [[ "$1" == "compose" && " $* " == *" down "* && "${FAKE_DOCKER_DOWN_FAIL:-0}" == "1" ]]; then exit 9; fi' \
  'if [[ "$1" =~ ^(ps|network|volume)$ && "${FAKE_DOCKER_LEFTOVER:-0}" == "1" ]]; then printf "leftover\n"; fi' \
  >"$fake_docker"
chmod +x "$fake_docker"

write_state() {
  local state_root="$1"
  local project="$2"
  mkdir -p "$state_root/docker-multiclient"
  printf 'project=%s\ncompose_file=%s\n' \
    "$project" "$ROOT_DIR/docker-compose.multiclient.yml" \
    >"$state_root/docker-multiclient/fixture-state"
}

success_state="$fixture/success"
write_state "$success_state" "deve-multiclient-success"
FAKE_DOCKER_CALLS="$calls" \
DEVE_DOCKER_BIN="$fake_docker" \
DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$success_state" \
  bash "$ROOT_DIR/scripts/cleanup-docker-multiclient.sh"
[[ ! -f "$success_state/docker-multiclient/fixture-state" ]] \
  || { echo "successful cleanup retained state" >&2; exit 1; }

failure_state="$fixture/failure"
write_state "$failure_state" "deve-multiclient-failure"
if FAKE_DOCKER_CALLS="$calls" \
    FAKE_DOCKER_DOWN_FAIL=1 \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$failure_state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-multiclient.sh"; then
  echo "failed compose down was accepted" >&2
  exit 1
fi
[[ -f "$failure_state/docker-multiclient/fixture-state" ]] \
  || { echo "failed cleanup removed retry state" >&2; exit 1; }

leftover_state="$fixture/leftover"
write_state "$leftover_state" "deve-multiclient-leftover"
if FAKE_DOCKER_CALLS="$calls" \
    FAKE_DOCKER_LEFTOVER=1 \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$leftover_state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-multiclient.sh"; then
  echo "remaining compose resources were accepted" >&2
  exit 1
fi

invalid_state="$fixture/invalid"
write_state "$invalid_state" "other-project"
before="$(wc -l <"$calls")"
if FAKE_DOCKER_CALLS="$calls" \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$invalid_state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-multiclient.sh"; then
  echo "invalid project identity was accepted" >&2
  exit 1
fi
after="$(wc -l <"$calls")"
[[ "$before" == "$after" ]] || { echo "invalid state reached Docker" >&2; exit 1; }

echo "docker-multiclient-cleanup-test: ok"
