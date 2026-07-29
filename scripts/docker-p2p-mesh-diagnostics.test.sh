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
    "logs --no-color --tail 1000") printf '%s\n' "$MOCK_LOGS" ;;
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

MOCK_LOGS="unsafe $TOKEN_A material"
docker_p2p_mesh_diagnose 2>"$output" \
  || fail_test "token-bearing logs should be suppressed without aborting safe diagnostics"
grep -Fq 'compose logs suppressed because token material was detected' "$output" \
  || fail_test "token suppression was not reported"
if grep -Fq "$TOKEN_A" "$output"; then
  fail_test "token material escaped diagnostic suppression"
fi

echo "docker-p2p-mesh-diagnostics.test: ok"
