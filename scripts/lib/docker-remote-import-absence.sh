#!/usr/bin/env bash
set -euo pipefail

project="${1:-}"
[[ "$project" =~ ^deve-remote-import-[0-9a-f]{12}$ ]] || {
  printf 'docker-remote-import: invalid project identity for absence check\n' >&2
  exit 1
}
docker_bin="${DEVE_REMOTE_IMPORT_DOCKER_BIN:-docker}"

containers="$("$docker_bin" ps --all \
  --filter "label=com.docker.compose.project=$project" --format '{{.ID}}')"
[[ -z "$containers" ]] || {
  printf 'docker-remote-import: owned containers survived cleanup\n' >&2
  exit 1
}
volumes="$("$docker_bin" volume ls \
  --filter "label=com.docker.compose.project=$project" --format '{{.Name}}')"
[[ -z "$volumes" ]] || {
  printf 'docker-remote-import: owned volumes survived cleanup\n' >&2
  exit 1
}
networks="$("$docker_bin" network ls \
  --filter "label=com.docker.compose.project=$project" --format '{{.Name}}')"
[[ -z "$networks" ]] || {
  printf 'docker-remote-import: owned networks survived cleanup\n' >&2
  exit 1
}
