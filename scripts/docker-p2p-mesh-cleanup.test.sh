#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
fake_docker="$fixture/docker"
calls="$fixture/calls"

cat >"$fake_docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_DOCKER_CALLS"
if [[ "${FAKE_DOCKER_IGNORE_ALL:-0}" == "1" ]]; then
  trap '' TERM
  sleep 60
fi
if [[ "$1" == "ps" && "${FAKE_DOCKER_IGNORE_TERM:-0}" == "1" ]]; then
  trap '' TERM
  sleep 60
fi
if [[ "$1" == "compose" && " $* " == *" down "* \
  && "${FAKE_DOCKER_DOWN_FAIL:-0}" == "1" ]]; then
  exit 9
fi
if [[ "$1" == "ps" && "${FAKE_DOCKER_PROBE_FAIL:-0}" == "1" ]]; then
  exit 8
fi
if [[ "$1" =~ ^(ps|network|volume)$ && "${FAKE_DOCKER_LEFTOVER:-0}" == "1" ]]; then
  printf 'leftover\n'
fi
EOF
chmod +x "$fake_docker"

write_state() {
  local root="$1"
  local project="$2"
  local state_dir="$root/docker-p2p-mesh"
  local cookie_a="$state_dir/peer-a.cookie"
  local cookie_b="$state_dir/peer-b.cookie"
  mkdir -p "$state_dir"
  DEVE_DOCKER_P2P_MESH_STATE_FILE="$state_dir/fixture-state" \
    bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh" \
      write "$project" "$ROOT_DIR/docker-compose.mesh.yml" "$cookie_a" "$cookie_b" \
    || return $?
  printf 'session-a' >"$cookie_a"
  printf 'session-b' >"$cookie_b"
}

success="$fixture/success"
write_state "$success" "deve-p2p-mesh-success"
if write_state "$success" "deve-p2p-mesh-replacement"; then
  echo "existing cleanup state was overwritten" >&2
  exit 1
fi
FAKE_DOCKER_CALLS="$calls" DEVE_DOCKER_BIN="$fake_docker" \
DEVE_DOCKER_P2P_MESH_STATE_FILE="$success/docker-p2p-mesh/fixture-state" \
  bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"
[[ ! -e "$success/docker-p2p-mesh/fixture-state" ]]
[[ ! -e "$success/docker-p2p-mesh/peer-a.cookie" ]]
[[ ! -e "$success/docker-p2p-mesh/peer-b.cookie" ]]

failure="$fixture/failure"
write_state "$failure" "deve-p2p-mesh-failure"
if FAKE_DOCKER_CALLS="$calls" FAKE_DOCKER_DOWN_FAIL=1 \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_DOCKER_P2P_MESH_STATE_FILE="$failure/docker-p2p-mesh/fixture-state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
  echo "failed compose cleanup was accepted" >&2
  exit 1
fi
[[ -f "$failure/docker-p2p-mesh/fixture-state" ]]

leftover="$fixture/leftover"
write_state "$leftover" "deve-p2p-mesh-leftover"
if FAKE_DOCKER_CALLS="$calls" FAKE_DOCKER_LEFTOVER=1 \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_DOCKER_P2P_MESH_STATE_FILE="$leftover/docker-p2p-mesh/fixture-state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
  echo "remaining compose resources were accepted" >&2
  exit 1
fi
[[ -f "$leftover/docker-p2p-mesh/fixture-state" ]]

probe="$fixture/probe"
write_state "$probe" "deve-p2p-mesh-probe"
if FAKE_DOCKER_CALLS="$calls" FAKE_DOCKER_PROBE_FAIL=1 \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_DOCKER_P2P_MESH_STATE_FILE="$probe/docker-p2p-mesh/fixture-state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
  echo "failed absence probe was accepted" >&2
  exit 1
fi
[[ -f "$probe/docker-p2p-mesh/fixture-state" ]]

invalid="$fixture/invalid"
mkdir -p "$invalid/docker-p2p-mesh"
printf 'project=other-project\ncompose_file=%s\ncookie_a=%s/a\ncookie_b=%s/b\n' \
  "$ROOT_DIR/docker-compose.mesh.yml" "$invalid/docker-p2p-mesh" \
  "$invalid/docker-p2p-mesh" >"$invalid/docker-p2p-mesh/fixture-state"
before="$(wc -l <"$calls")"
if FAKE_DOCKER_CALLS="$calls" DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_DOCKER_P2P_MESH_STATE_FILE="$invalid/docker-p2p-mesh/fixture-state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
  echo "invalid cleanup project was accepted" >&2
  exit 1
fi
after="$(wc -l <"$calls")"
[[ "$before" == "$after" ]]

assert_invalid_cookie_state() {
  local label="$1"
  local cookie_a="$2"
  local cookie_b="$3"
  local root="$fixture/cookie-$label"
  local state_dir="$root/docker-p2p-mesh"
  mkdir -p "$state_dir"
  printf 'project=deve-p2p-mesh-cookie-%s\ncompose_file=%s\ncookie_a=%s\ncookie_b=%s\n' \
    "$label" "$ROOT_DIR/docker-compose.mesh.yml" "$cookie_a" "$cookie_b" \
    >"$state_dir/fixture-state"
  before="$(wc -l <"$calls")"
  if FAKE_DOCKER_CALLS="$calls" DEVE_DOCKER_BIN="$fake_docker" \
      DEVE_DOCKER_P2P_MESH_STATE_FILE="$state_dir/fixture-state" \
        bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
    echo "invalid cookie state was accepted: $label" >&2
    exit 1
  fi
  after="$(wc -l <"$calls")"
  [[ "$before" == "$after" ]]
}

traversal_root="$fixture/cookie-traversal/docker-p2p-mesh"
traversal_victim="$fixture/cookie-traversal/victim.cookie"
mkdir -p "$traversal_root"
printf 'preserve-me' >"$traversal_victim"
assert_invalid_cookie_state traversal \
  "$traversal_root/../victim.cookie" "$traversal_root/peer-b.cookie"
[[ "$(cat "$traversal_victim")" == "preserve-me" ]]

self_root="$fixture/cookie-self/docker-p2p-mesh"
assert_invalid_cookie_state self \
  "$self_root/fixture-state" "$self_root/peer-b.cookie"

swapped_root="$fixture/cookie-swapped/docker-p2p-mesh"
assert_invalid_cookie_state swapped \
  "$swapped_root/peer-b.cookie" "$swapped_root/peer-a.cookie"

duplicate_root="$fixture/cookie-duplicate/docker-p2p-mesh"
assert_invalid_cookie_state duplicate \
  "$duplicate_root/peer-a.cookie" "$duplicate_root/peer-a.cookie"

symlink="$fixture/symlink"
mkdir -p "$symlink/docker-p2p-mesh"
symlink_target="$fixture/symlink-target"
printf 'preserve-me' >"$symlink_target"
if ln -s "$symlink_target" "$symlink/docker-p2p-mesh/fixture-state" 2>/dev/null \
    && [[ -L "$symlink/docker-p2p-mesh/fixture-state" ]]; then
  if write_state "$symlink" "deve-p2p-mesh-symlink"; then
    echo "symlink cleanup state was overwritten" >&2
    exit 1
  fi
  [[ "$(cat "$symlink_target")" == "preserve-me" ]]
else
  rm -f -- "$symlink/docker-p2p-mesh/fixture-state"
  echo "docker-p2p-mesh-cleanup-test: symlink case skipped on this filesystem"
fi

temp_attack="$fixture/temp-attack"
mkdir -p "$temp_attack/docker-p2p-mesh" "$temp_attack/bin"
temp_target="$fixture/temp-target"
printf 'preserve-me' >"$temp_target"
if ln -s "$temp_target" "$temp_attack/fake-tmp" 2>/dev/null \
    && [[ -L "$temp_attack/fake-tmp" ]]; then
  cat >"$temp_attack/bin/mktemp" <<EOF
#!/usr/bin/env bash
printf '%s\n' '$temp_attack/fake-tmp'
EOF
  chmod +x "$temp_attack/bin/mktemp"
  if PATH="$temp_attack/bin:$PATH" \
      DEVE_DOCKER_P2P_MESH_STATE_FILE="$temp_attack/docker-p2p-mesh/fixture-state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh" write \
        "deve-p2p-mesh-temp-attack" "$ROOT_DIR/docker-compose.mesh.yml" \
        "$temp_attack/docker-p2p-mesh/peer-a.cookie" \
        "$temp_attack/docker-p2p-mesh/peer-b.cookie"; then
    echo "symlink state temporary was accepted" >&2
    exit 1
  fi
  [[ "$(cat "$temp_target")" == "preserve-me" ]]
else
  rm -f -- "$temp_attack/fake-tmp"
  echo "docker-p2p-mesh-cleanup-test: temp symlink case skipped on this filesystem"
fi

override="$fixture/override"
if DEVE_ACCEPTANCE_PRODUCER_STATE_DIR="$override" \
    DEVE_DOCKER_P2P_MESH_STATE_FILE="$fixture/outside-state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
  echo "receipt state override was accepted" >&2
  exit 1
fi

bounded="$fixture/bounded"
write_state "$bounded" "deve-p2p-mesh-bounded"
started="$(date +%s)"
if FAKE_DOCKER_CALLS="$calls" FAKE_DOCKER_IGNORE_TERM=1 \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_DOCKER_P2P_MESH_STATE_FILE="$bounded/docker-p2p-mesh/fixture-state" \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
  echo "TERM-ignoring Docker probe was accepted" >&2
  exit 1
fi
elapsed="$(( $(date +%s) - started ))"
(( elapsed < 25 )) || {
  echo "TERM-ignoring Docker probe exceeded the hard cleanup bound: ${elapsed}s" >&2
  exit 1
}
[[ -f "$bounded/docker-p2p-mesh/fixture-state" ]]

cumulative="$fixture/cumulative"
write_state "$cumulative" "deve-p2p-mesh-cumulative"
started="$(date +%s)"
if FAKE_DOCKER_CALLS="$calls" FAKE_DOCKER_IGNORE_ALL=1 \
    DEVE_DOCKER_BIN="$fake_docker" \
    DEVE_DOCKER_P2P_MESH_STATE_FILE="$cumulative/docker-p2p-mesh/fixture-state" \
    DEVE_DOCKER_P2P_MESH_CLEANUP_TOTAL_SECONDS=6 \
    DEVE_DOCKER_P2P_MESH_CLEANUP_COMPOSE_SECONDS=3 \
    DEVE_DOCKER_P2P_MESH_CLEANUP_PROBE_SECONDS=2 \
    DEVE_DOCKER_P2P_MESH_CLEANUP_KILL_AFTER_SECONDS=1 \
      bash "$ROOT_DIR/scripts/cleanup-docker-p2p-mesh.sh"; then
  echo "cumulative cleanup timeout was accepted" >&2
  exit 1
fi
elapsed="$(( $(date +%s) - started ))"
(( elapsed < 10 )) || {
  echo "cumulative cleanup exceeded its shared deadline: ${elapsed}s" >&2
  exit 1
}
[[ ! -e "$cumulative/docker-p2p-mesh/peer-a.cookie" ]]
[[ ! -e "$cumulative/docker-p2p-mesh/peer-b.cookie" ]]
[[ -f "$cumulative/docker-p2p-mesh/fixture-state" ]]

echo "docker-p2p-mesh-cleanup-test: ok"
