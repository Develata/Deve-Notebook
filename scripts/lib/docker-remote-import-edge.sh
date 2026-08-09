#!/usr/bin/env bash
# shellcheck shell=bash

# Resolve and verify public quick-tunnel edges from the exact candidate image.
# The selected mapping remains ephemeral test infrastructure; product locator
# admission and Remote Import authority stay inside the candidate backend.

remote_import_edge_fail() {
  printf 'docker-remote-import: %s\n' "$*" >&2
  return 1
}

remote_import_edge_ipv4_candidates() {
  local origin="$1"
  node -e '
const origin = new URL(process.argv[1]);
if (
  origin.protocol !== "https:" ||
  !origin.hostname.endsWith(".trycloudflare.com") ||
  origin.username ||
  origin.password ||
  origin.port
) {
  throw new Error("refusing DoH resolution for an unowned tunnel origin");
}
function isPublicIpv4(value) {
  const octets = value.split(".").map(Number);
  if (
    octets.length !== 4 ||
    octets.some((part) => !Number.isInteger(part) || part < 0 || part > 255)
  ) return false;
  const [a, b, c] = octets;
  return !(
    a === 0 || a === 10 || a === 127 || a >= 224 ||
    (a === 100 && b >= 64 && b <= 127) ||
    (a === 169 && b === 254) ||
    (a === 172 && b >= 16 && b <= 31) ||
    (a === 192 && b === 168) ||
    (a === 192 && b === 0 && (c === 0 || c === 2)) ||
    (a === 198 && (b === 18 || b === 19)) ||
    (a === 198 && b === 51 && c === 100) ||
    (a === 203 && b === 0 && c === 113)
  );
}
const names = [origin.hostname, "trycloudflare.com"];
const addresses = new Set();
let lastError;
const deadline = Date.now() + 120000;
while (addresses.size === 0 && Date.now() < deadline) {
  for (const name of names) {
    try {
      const response = await fetch(
        `https://cloudflare-dns.com/dns-query?name=${encodeURIComponent(name)}&type=A`,
        {
          headers: { accept: "application/dns-json" },
          signal: AbortSignal.timeout(2000),
        },
      );
      if (!response.ok) throw new Error(`DoH returned HTTP ${response.status}`);
      const payload = await response.json();
      if (payload.Status === 0 && Array.isArray(payload.Answer)) {
        for (const answer of payload.Answer) {
          if (answer.type === 1 && isPublicIpv4(answer.data)) {
            addresses.add(answer.data);
          }
        }
      }
    } catch (error) {
      lastError = error;
    }
  }
  if (addresses.size === 0 && Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
}
if (addresses.size === 0) {
  throw new Error(
    `DoH did not return a public tunnel-edge IPv4 address before timeout${
      lastError ? `: ${lastError.message}` : ""
    }`,
  );
}
for (const address of addresses) {
  process.stdout.write(`${origin.hostname} ${address}\n`);
}
' "$origin"
}

remote_import_edge_validate_probe() {
  local origin="$1"
  local probe_url="$2"
  local method="$3"
  node -e '
const origin = new URL(process.argv[1]);
const probe = new URL(process.argv[2]);
const method = process.argv[3];
if (
  origin.origin !== probe.origin ||
  origin.protocol !== "https:" ||
  !origin.hostname.endsWith(".trycloudflare.com") ||
  origin.username ||
  origin.password ||
  probe.username ||
  probe.password ||
  probe.hash ||
  !["GET", "PROPFIND"].includes(method)
) {
  throw new Error("refusing an invalid tunnel-edge probe");
}
process.stdout.write(`${origin.hostname}\n`);
' "$origin" "$probe_url" "$method"
}

# Account-less quick tunnels have no propagation guarantee: the edge can keep
# answering 530 (error 1033, no route to tunnel) for a minute or more after
# cloudflared registers. The window below only re-runs the unchanged probe
# sweep against freshly resolved edges; the acceptance criterion for one probe
# (exact-candidate curl, pinned resolve, HTTP 2xx) must never be relaxed.
remote_import_edge_propagation_window_secs() {
  local window="${DEVE_REMOTE_IMPORT_EDGE_PROPAGATION_WINDOW_SECS:-180}"
  if ! [[ "$window" =~ ^(0|[1-9][0-9]{0,2})$ ]] || ((window > 600)); then
    printf 'docker-remote-import: invalid edge propagation window; using 180s\n' >&2
    window=180
  fi
  printf '%s\n' "$window"
}

remote_import_edge_is_excluded() {
  local candidate_ip="$1"
  local excluded_csv="${2:-}"
  local excluded_ip
  local -a excluded_ips=()
  IFS=',' read -r -a excluded_ips <<<"$excluded_csv"
  for excluded_ip in "${excluded_ips[@]}"; do
    [[ -z "$excluded_ip" || "$candidate_ip" != "$excluded_ip" ]] || return 0
  done
  return 1
}

remote_import_edge_select_ipv4() {
  local label="$1"
  local origin="$2"
  local probe_url="$3"
  local method="$4"
  local excluded_csv="${5:-}"
  local host
  host="$(remote_import_edge_validate_probe "$origin" "$probe_url" "$method")" \
    || return 1
  [[ "${DEVE_RELEASE_CANDIDATE_IMAGE_ID:-}" =~ ^sha256:[0-9a-f]{64}$ ]] \
    || remote_import_edge_fail "exact candidate image ID is required for edge selection"
  [[ "${DEVE_REMOTE_IMPORT_PROJECT:-}" =~ ^deve-remote-import-[0-9a-f]{12}$ ]] \
    || remote_import_edge_fail "valid project identity is required for edge selection"

  local state_root probe_stdout probe_stderr probe_container
  state_root="$(dirname -- "$(remote_import_fixture_state_file)")"
  probe_stdout="$state_root/${label}-edge-probe.stdout.log"
  probe_stderr="$state_root/${label}-edge-probe.stderr.log"
  probe_container="${DEVE_REMOTE_IMPORT_PROJECT}-${label}-edge"
  local candidate_host candidate_ip probe_attempt run_status status remaining
  local propagation_deadline sweep
  propagation_deadline=$((SECONDS + $(remote_import_edge_propagation_window_secs)))
  sweep=0
  while :; do
    sweep=$((sweep + 1))
    while read -r candidate_host candidate_ip; do
      [[ "$candidate_host" == "$host" && -n "$candidate_ip" ]] || continue
      if remote_import_edge_is_excluded "$candidate_ip" "$excluded_csv"; then
        printf 'docker-remote-import: skipping failed %s edge ip=%s\n' \
          "$label" "$candidate_ip" >&2
        continue
      fi
      local -a curl_args=(
        --silent
        --show-error
        --output /dev/null
        --write-out '%{http_code}'
        --connect-timeout 3
        --max-time 8
        --http1.1
        --request "$method"
        --resolve "$host:443:$candidate_ip"
      )
      if [[ "$method" == "PROPFIND" ]]; then
        curl_args+=(
          --header "Depth: 1"
          --header "Content-Type: application/xml; charset=utf-8"
          --data-binary '<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/></d:prop></d:propfind>'
        )
      fi
      for probe_attempt in 1 2 3 4 5; do
        run_status=0
        remote_fixture_run_bounded \
          "$label tunnel-edge $method probe attempt $probe_attempt" \
          15 1048576 "$probe_stdout" "$probe_stderr" -- \
          "$DEVE_REMOTE_IMPORT_DOCKER_BIN" run --rm --pull never \
            --name "$probe_container" --entrypoint curl \
            "$DEVE_RELEASE_CANDIDATE_IMAGE_ID" "${curl_args[@]}" "$probe_url" \
          || run_status=$?
        "$DEVE_REMOTE_IMPORT_DOCKER_BIN" rm -f "$probe_container" \
          >/dev/null 2>&1 || true
        if ! remaining="$("$DEVE_REMOTE_IMPORT_DOCKER_BIN" container ls -aq \
          --filter "name=^${probe_container}$")"; then
          remote_import_edge_fail "could not verify edge probe container cleanup"
          return 1
        fi
        if [[ -n "$remaining" ]]; then
          remote_import_edge_fail "edge probe container remained after cleanup"
          return 1
        fi
        status=""
        [[ ! -f "$probe_stdout" ]] || status="$(<"$probe_stdout")"
        if ((run_status == 0)) && [[ "$status" =~ ^2[0-9][0-9]$ ]]; then
          printf 'docker-remote-import: selected %s edge host=%s ip=%s method=%s attempt=%s\n' \
            "$label" "$host" "$candidate_ip" "$method" "$probe_attempt" >&2
          printf '%s %s\n' "$host" "$candidate_ip"
          return 0
        fi
        if ((probe_attempt < 5)); then
          printf 'docker-remote-import: retrying %s edge ip=%s method=%s status=%s attempt=%s\n' \
            "$label" "$candidate_ip" "$method" "${status:-none}" "$probe_attempt" >&2
          sleep 1
        fi
      done
      printf 'docker-remote-import: rejected %s edge ip=%s method=%s status=%s\n' \
        "$label" "$candidate_ip" "$method" "${status:-none}" >&2
    done < <(remote_import_edge_ipv4_candidates "$origin")
    ((SECONDS < propagation_deadline)) || break
    printf 'docker-remote-import: waiting for %s tunnel edge route propagation (sweep %s)\n' \
      "$label" "$sweep" >&2
    sleep 5
  done
  remote_import_edge_fail \
    "no tunnel edge passed the exact-candidate $method probe for $label"
}
