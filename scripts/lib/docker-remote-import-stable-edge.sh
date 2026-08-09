#!/usr/bin/env bash
# shellcheck shell=bash

# Admit a quick-tunnel edge from the real candidate service network. A failed
# edge may be replaced once, but the exact method and 60-sample stability gate
# remain owned by docker-remote-import-fixture.sh.

remote_import_fixture_verify_retryable_edge_failure() {
  local service="$1"
  local expected_container="$2"
  local label="$3"
  local app_url="$4"
  local edge_attempt="$5"
  local current_container runtime_identity state_root
  if ! current_container="$(remote_import_fixture_container_id "$service")"; then
    return 1
  fi
  if [[ "$current_container" != "$expected_container" ]]; then
    remote_import_fixture_fail \
      "$service container identity changed during $label edge admission"
    return 1
  fi
  if ! runtime_identity="$(
    "$DEVE_REMOTE_IMPORT_DOCKER_BIN" inspect \
      --format '{{.State.Running}} {{.Image}}' "$current_container"
  )"; then
    remote_import_fixture_fail \
      "$service runtime identity unavailable after $label edge failure"
    return 1
  fi
  if [[ "$runtime_identity" != "true $DEVE_RELEASE_CANDIDATE_IMAGE_ID" ]]; then
    remote_import_fixture_fail \
      "$service was not the same running exact candidate after $label edge failure"
    return 1
  fi
  if ! remote_import_fixture_wait_url \
    "$label-candidate-after-edge-failure" "$app_url" 1; then
    remote_import_fixture_fail \
      "$service app was not immediately ready after $label edge failure"
    return 1
  fi
  state_root="$(dirname -- "$(remote_import_fixture_state_file)")"
  if ! remote_fixture_run_bounded \
    "$label candidate log before edge replacement" 15 1048576 \
    "$state_root/${label}-edge-${edge_attempt}-candidate.stdout.log" \
    "$state_root/${label}-edge-${edge_attempt}-candidate.stderr.log" -- \
    "$DEVE_REMOTE_IMPORT_DOCKER_BIN" logs --tail 160 "$current_container"; then
    remote_import_fixture_fail \
      "$service diagnostics failed before $label edge replacement"
    return 1
  fi
}

remote_import_fixture_admit_stable_edge() {
  local service="$1"
  local label="$2"
  local origin="$3"
  local probe_url="$4"
  local method="$5"
  local host_var="$6"
  local ip_var="$7"
  local app_url="$8"
  [[ "$host_var" =~ ^DEVE_REMOTE_IMPORT_[A-Z0-9_]+$ \
    && "$ip_var" =~ ^DEVE_REMOTE_IMPORT_[A-Z0-9_]+$ ]] \
    || {
      remote_import_fixture_fail "stable edge variable binding is invalid"
      return 1
    }

  local expected_host current_host current_ip mapping failed_ips=""
  expected_host="$(remote_import_edge_validate_probe "$origin" "$probe_url" "$method")" \
    || return 1
  for edge_attempt in 1 2; do
    if ((edge_attempt > 1)); then
      if ! mapping="$(
        remote_import_edge_select_ipv4 \
          "$label-recovery" "$origin" "$probe_url" "$method" "$failed_ips"
      )"; then
        remote_import_fixture_fail \
          "no distinct healthy edge remained for stable $label $method"
        return 1
      fi
      read -r current_host current_ip <<<"$mapping"
      if remote_import_edge_is_excluded "$current_ip" "$failed_ips"; then
        remote_import_fixture_fail \
          "edge selector returned a previously failed $label IP"
        return 1
      fi
      printf -v "$host_var" '%s' "$current_host"
      printf -v "$ip_var" '%s' "$current_ip"
      export "$host_var" "$ip_var"
    else
      current_host="${!host_var:-}"
      current_ip="${!ip_var:-}"
    fi
    [[ "$current_host" == "$expected_host" \
      && "$current_ip" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] \
      || {
        remote_import_fixture_fail "selected stable edge identity is invalid"
        return 1
      }

    printf 'docker-remote-import: admitting %s edge ip=%s attempt=%s/2\n' \
      "$label" "$current_ip" "$edge_attempt" >&2
    if ! remote_import_fixture_compose up -d --force-recreate "$service"; then
      remote_import_fixture_fail "$service force-recreate failed for $label edge admission"
      return 1
    fi
    if ! remote_import_fixture_verify_candidate_container "$service"; then
      return 1
    fi
    if ! remote_import_fixture_wait_url "$label-candidate" "$app_url" 180; then
      return 1
    fi
    local admitted_container
    if ! admitted_container="$(remote_import_fixture_container_id "$service")"; then
      return 1
    fi
    if remote_import_fixture_wait_container_url \
      "$service" "$label-tunnel" "$probe_url" 240 60 "$method"; then
      return 0
    fi
    if ! remote_import_fixture_verify_retryable_edge_failure \
      "$service" "$admitted_container" "$label" "$app_url" "$edge_attempt"; then
      return 1
    fi
    failed_ips="${failed_ips:+$failed_ips,}$current_ip"
    printf 'docker-remote-import: rejecting unstable %s edge ip=%s attempt=%s/2\n' \
      "$label" "$current_ip" "$edge_attempt" >&2
  done
  remote_import_fixture_fail \
    "all bounded edge candidates failed stable $label $method admission"
}
