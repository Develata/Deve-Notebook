#!/usr/bin/env bash
# Bounded, secret-aware diagnostics for the Docker FullPeer mesh fixture.

docker_p2p_mesh_diagnose() {
  local logs="" role="" service="" port="" container_id=""
  local -a container_ids=()

  echo "docker-p2p-mesh-smoke: collecting compose diagnostics" >&2
  docker_compose ps >&2 || true

  for service in peer-a peer-b; do
    if [[ "$service" == "peer-a" ]]; then
      port="$PORT_A"
    else
      port="$PORT_B"
    fi
    role="$(curl_local --connect-timeout 2 --max-time 5 -fsS \
      "http://127.0.0.1:${port}/api/node/role" 2>/dev/null \
      | head -c 32768 \
      || true)"
    echo "docker-p2p-mesh-smoke: ${service} node-role (bounded): ${role:-<unavailable>}" >&2

    container_id="$(docker_compose ps -q "$service" 2>/dev/null || true)"
    if [[ -z "$container_id" ]]; then
      echo "docker-p2p-mesh-smoke: ${service} container state: unavailable" >&2
      continue
    fi
    container_ids+=("$container_id")
    docker_cmd inspect --format \
      'name={{.Name}} status={{.State.Status}} health={{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}} restart={{.RestartCount}} oom={{.State.OOMKilled}} exit={{.State.ExitCode}}' \
      "$container_id" >&2 || true
  done

  if (( ${#container_ids[@]} > 0 )); then
    if command -v timeout >/dev/null 2>&1; then
      timeout --signal=TERM --kill-after=2s 10s \
        "$DOCKER_BIN" stats --no-stream \
        --format 'name={{.Name}} cpu={{.CPUPerc}} memory={{.MemUsage}} pids={{.PIDs}}' \
        "${container_ids[@]}" >&2 || true
    else
      echo "docker-p2p-mesh-smoke: docker stats skipped; timeout command unavailable" >&2
    fi
  fi

  logs="$(docker_compose logs --no-color --tail 1000 2>/dev/null || true)"
  if grep -qF "$TOKEN_A" <<<"$logs" || grep -qF "$TOKEN_B" <<<"$logs"; then
    echo "docker-p2p-mesh-smoke: compose logs suppressed because token material was detected" >&2
    return
  fi
  printf '%s\n' "$logs" >&2
}
