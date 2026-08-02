#!/usr/bin/env bash
# shellcheck shell=bash

# HTTP readiness for the Unix RemoteBrowser fixture. This helper only observes
# ephemeral release infrastructure; product and receipt authority remain in the
# exact candidate backend.

readonly DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_DEFAULT_SECS="180"
readonly DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_MAX_SECS="600"

remote_fixture_edge_propagation_window_secs() {
  local window="${DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_SECS:-$DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_DEFAULT_SECS}"
  if ! [[ "$window" =~ ^[1-9][0-9]{0,2}$ ]] \
    || ((window > DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_MAX_SECS)); then
    printf 'remote-browser-fixture: invalid edge propagation window; using %ss\n' \
      "$DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_DEFAULT_SECS" >&2
    window="$DEVE_REMOTE_FIXTURE_EDGE_PROPAGATION_WINDOW_DEFAULT_SECS"
  fi
  printf '%s\n' "$window"
}

remote_fixture_now_millis() {
  local now="${EPOCHREALTIME:-}"
  if [[ ! "$now" =~ ^([0-9]+)\.([0-9]+)$ ]]; then
    remote_fixture_fail "Bash EPOCHREALTIME is required for bounded HTTP probes"
    return 1
  fi
  local whole="${BASH_REMATCH[1]}"
  local fraction="${BASH_REMATCH[2]}000"
  printf '%s\n' "$((10#$whole * 1000 + 10#${fraction:0:3}))"
}

remote_fixture_timeout_from_millis() {
  local millis="$1"
  ((millis > 0)) || return 1
  printf '%d.%03d\n' "$((millis / 1000))" "$((millis % 1000))"
}

remote_fixture_http_status() {
  local url="$1"
  local connect_timeout="$2"
  local max_time="$3"
  local status
  status="$(curl --silent --show-error --output /dev/null \
    --write-out '%{http_code}' --connect-timeout "$connect_timeout" \
    --max-time "$max_time" --max-redirs 0 --http1.1 "$url" 2>/dev/null || true)"
  [[ "$status" =~ ^[0-9]{3}$ ]] || status="000"
  printf '%s\n' "$status"
}

remote_fixture_assert_tunnel_role_url() {
  local url="$1"
  if [[ ! "$url" =~ ^https://[A-Za-z0-9-]+\.trycloudflare\.com/api/node/role$ ]]; then
    remote_fixture_fail "refusing a non-exact quick-tunnel role probe"
    return 1
  fi
}

remote_fixture_wait_http() {
  local url="$1"
  local pid="$2"
  local log_path="$3"
  local attempt status="000"
  for attempt in $(seq 1 120); do
    status="$(remote_fixture_http_status "$url" 1 2)"
    if [[ "$status" =~ ^2[0-9]{2}$ ]]; then
      return 0
    fi
    if ! remote_fixture_pid_active "$pid"; then
      remote_fixture_fail \
        "process exited before health check succeeded; last_status=$status; log: $log_path"
      return 1
    fi
    sleep 0.25
  done
  remote_fixture_fail "timed out waiting for $url; last_status=$status; log: $log_path"
}

remote_fixture_wait_tunnel_http() {
  local url="$1"
  local pid="$2"
  local expected_token="$3"
  local log_path="$4"
  local window deadline now remaining probe_millis sleep_millis timeout status="000"
  remote_fixture_assert_tunnel_role_url "$url"
  window="$(remote_fixture_edge_propagation_window_secs)"
  now="$(remote_fixture_now_millis)"
  deadline=$((now + window * 1000))
  while :; do
    if ! remote_fixture_pid_active "$pid" \
      || [[ "$(remote_fixture_process_token "$pid" 2>/dev/null || true)" != "$expected_token" ]]; then
      remote_fixture_fail \
        "tunnel ownership ended before exact role probe succeeded; last_status=$status; log: $log_path"
      return 1
    fi
    now="$(remote_fixture_now_millis)"
    remaining=$((deadline - now))
    ((remaining > 0)) || break
    probe_millis="$remaining"
    ((probe_millis <= 8000)) || probe_millis=8000
    timeout="$(remote_fixture_timeout_from_millis "$probe_millis")"
    status="$(remote_fixture_http_status "$url" "$timeout" "$timeout")"
    if ! remote_fixture_pid_active "$pid" \
      || [[ "$(remote_fixture_process_token "$pid" 2>/dev/null || true)" != "$expected_token" ]]; then
      remote_fixture_fail \
        "tunnel ownership ended during exact role probe; last_status=$status; log: $log_path"
      return 1
    fi
    now="$(remote_fixture_now_millis)"
    if [[ "$status" =~ ^2[0-9]{2}$ ]]; then
      ((now <= deadline)) && return 0
      break
    fi
    remaining=$((deadline - now))
    ((remaining > 0)) || break
    sleep_millis="$remaining"
    ((sleep_millis <= 250)) || sleep_millis=250
    sleep "$(remote_fixture_timeout_from_millis "$sleep_millis")"
  done
  remote_fixture_fail \
    "timed out waiting for exact quick-tunnel role probe; last_status=$status; log: $log_path"
}

remote_fixture_wait_tunnel_origin() {
  local pid="$1"
  local stdout_log="$2"
  local stderr_log="$3"
  local attempt origin
  for attempt in $(seq 1 120); do
    origin="$(sed -nE 's#.*(https://[A-Za-z0-9-]+\.trycloudflare\.com).*#\1#p' \
      "$stdout_log" "$stderr_log" 2>/dev/null | head -n 1)"
    if [[ -n "$origin" ]]; then
      remote_fixture_assert_https_origin "$origin"
      printf '%s\n' "$origin"
      return 0
    fi
    if ! remote_fixture_pid_active "$pid"; then
      remote_fixture_fail "cloudflared exited before publishing an HTTPS origin"
      return 1
    fi
    sleep 0.25
  done
  remote_fixture_fail "timed out waiting for cloudflared quick-tunnel origin"
}
