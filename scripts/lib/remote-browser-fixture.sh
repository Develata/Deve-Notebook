#!/usr/bin/env bash
# shellcheck shell=bash

# Narrow lifecycle helpers for the RemoteBrowser target-host fixture. Product
# authority remains in deve_cli; this file only owns ephemeral host processes.

readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_VERSION="2026.7.2"
readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_LINUX_AMD64_SHA256="ec905ea7b7e327ff8abdde8cb64697a2152de74dbcdbf6aec9db8364eb3886cd"
readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_TIMEOUT_SECONDS="180"
readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_LIMIT_BYTES="134217728"

remote_fixture_fail() {
  printf 'remote-browser-fixture: %s\n' "$*" >&2
  return 1
}

remote_fixture_require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    remote_fixture_fail "required command not found: $1"
    return 1
  }
}

remote_fixture_reject_control_chars() {
  local value="$1"
  if [[ "$value" == *$'\n'* || "$value" == *$'\r'* || "$value" == *$'\t'* ]]; then
    remote_fixture_fail "value contains a control character"
    return 1
  fi
}

remote_fixture_canonical_dir() {
  local path="$1"
  remote_fixture_reject_control_chars "$path"
  if [[ -L "$path" ]]; then
    remote_fixture_fail "state directory must not be a symlink: $path"
    return 1
  fi
  mkdir -p -- "$path"
  (cd -- "$path" && pwd -P)
}

remote_fixture_path_key() {
  local path="$1"
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -am -- "$path"
    return
  fi
  local parent base
  parent="$(dirname -- "$path")"
  base="$(basename -- "$path")"
  printf '%s/%s\n' "$(cd -- "$parent" && pwd -P)" "$base"
}

remote_fixture_random_hex() {
  local bytes="$1"
  od -An -N "$bytes" -tx1 /dev/urandom | tr -d ' \n'
}

remote_fixture_assert_https_origin() {
  local origin="$1"
  remote_fixture_reject_control_chars "$origin"
  if [[ ! "$origin" =~ ^https://[A-Za-z0-9.-]+(:[0-9]+)?$ ]]; then
    remote_fixture_fail "expected an exact HTTPS origin, got: $origin"
    return 1
  fi
}

remote_fixture_assert_expected_head() {
  local repo_root="$1"
  local expected_head="$2"
  if [[ ! "$expected_head" =~ ^[0-9a-fA-F]{40}$ ]]; then
    remote_fixture_fail "expected HEAD must be a full 40-character commit SHA"
    return 1
  fi
  local actual_head
  actual_head="$(git -C "$repo_root" rev-parse HEAD)"
  if [[ "${actual_head,,}" != "${expected_head,,}" ]]; then
    remote_fixture_fail "workspace HEAD mismatch: expected $expected_head, observed $actual_head"
    return 1
  fi
}

remote_fixture_find_free_port() {
  remote_fixture_require_command node
  node -e 'const n=require("net");const s=n.createServer();s.listen(0,"127.0.0.1",()=>{process.stdout.write(String(s.address().port));s.close();});'
}

remote_fixture_process_token() {
  local pid="$1"
  [[ -r "/proc/$pid/stat" ]] || return 1
  awk '{print $22}' "/proc/$pid/stat"
}

remote_fixture_pid_active() {
  local pid="$1"
  kill -0 "$pid" 2>/dev/null || return 1
  if [[ -r "/proc/$pid/stat" && "$(awk '{print $3}' "/proc/$pid/stat")" == "Z" ]]; then
    return 1
  fi
  return 0
}

remote_fixture_job_active() {
  local pid="$1"
  local job_pid
  while read -r job_pid; do
    [[ "$job_pid" == "$pid" ]] && return 0
  done < <(jobs -pr)
  return 1
}

remote_fixture_stop_pid() {
  local label="$1"
  local pid="$2"
  local expected_token="$3"
  [[ "$pid" =~ ^[0-9]+$ ]] || return 0
  remote_fixture_pid_active "$pid" || return 0

  local actual_token
  actual_token="$(remote_fixture_process_token "$pid" || true)"
  if [[ -z "$actual_token" || "$actual_token" != "$expected_token" ]]; then
    remote_fixture_fail "refusing to stop reused or unowned $label PID $pid"
    return 1
  fi

  kill -TERM "$pid" 2>/dev/null || true
  local attempt
  for attempt in $(seq 1 50); do
    remote_fixture_pid_active "$pid" || return 0
    sleep 0.1
  done
  kill -KILL "$pid" 2>/dev/null || true
  for attempt in $(seq 1 20); do
    remote_fixture_pid_active "$pid" || return 0
    sleep 0.1
  done
  remote_fixture_fail "$label PID $pid survived bounded cleanup"
}

remote_fixture_stop_bounded_tree() {
  local label="$1"
  local pid="$2"
  local process_group="$3"
  remote_fixture_pid_active "$pid" || return 0

  if [[ "$process_group" == "1" ]]; then
    kill -TERM -- "-$pid" 2>/dev/null || true
  elif [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]]; then
    local child winpid
    # Windows does not expose MSYS fork children through the native parent
    # relation used by taskkill /T. Enumerate the MSYS PPID tree explicitly,
    # deepest first, and terminate each native process identity.
    while read -r child; do
      [[ -n "$child" ]] || continue
      winpid="$(ps -W 2>/dev/null | awk -v pid="$child" 'NR > 1 && $1 == pid { print $4; exit }')"
      [[ -z "$winpid" ]] || taskkill.exe //PID "$winpid" //T //F >/dev/null 2>&1 || true
    done < <(remote_fixture_msys_descendants "$pid")
    winpid="$(ps -W 2>/dev/null | awk -v pid="$pid" 'NR > 1 && $1 == pid { print $4; exit }')"
    if [[ -n "$winpid" ]] && command -v taskkill.exe >/dev/null 2>&1; then
      taskkill.exe //PID "$winpid" //T //F >/dev/null 2>&1 || true
    else
      kill -TERM "$pid" 2>/dev/null || true
    fi
  else
    # This fallback is only for hosts without setsid. Kill known descendants
    # before their parent so they cannot be orphaned by our own timeout path.
    if command -v pgrep >/dev/null 2>&1; then
      local child
      while read -r child; do
        [[ -n "$child" ]] || continue
        remote_fixture_stop_bounded_tree "$label child" "$child" 0 || true
      done < <(pgrep -P "$pid" 2>/dev/null || true)
    fi
    kill -TERM "$pid" 2>/dev/null || true
  fi

  local attempt
  for attempt in $(seq 1 30); do
    remote_fixture_pid_active "$pid" || return 0
    sleep 0.1
  done
  if [[ "$process_group" == "1" ]]; then
    kill -KILL -- "-$pid" 2>/dev/null || true
  else
    kill -KILL "$pid" 2>/dev/null || true
  fi
  for attempt in $(seq 1 20); do
    remote_fixture_pid_active "$pid" || return 0
    sleep 0.1
  done
  remote_fixture_fail "$label process tree survived bounded termination"
}

remote_fixture_msys_descendants() {
  local parent_pid="$1"
  local child_pid
  while read -r child_pid; do
    [[ -n "$child_pid" ]] || continue
    remote_fixture_msys_descendants "$child_pid"
    printf '%s\n' "$child_pid"
  done < <(ps -W 2>/dev/null | awk -v parent="$parent_pid" 'NR > 1 && $2 == parent { print $1 }')
}

remote_fixture_run_bounded() {
  local label="$1"
  local timeout_seconds="$2"
  local output_limit_bytes="$3"
  local stdout_path="$4"
  local stderr_path="$5"
  shift 5
  [[ "${1:-}" == "--" ]] || {
    remote_fixture_fail "bounded process command separator is missing"
    return 1
  }
  shift
  [[ "$timeout_seconds" =~ ^[0-9]+$ && "$timeout_seconds" -gt 0 ]] || {
    remote_fixture_fail "$label timeout must be a positive integer"
    return 1
  }
  [[ "$output_limit_bytes" =~ ^[0-9]+$ && "$output_limit_bytes" -ge 1024 ]] || {
    remote_fixture_fail "$label output limit must be at least 1024 bytes"
    return 1
  }
  (($# > 0)) || {
    remote_fixture_fail "$label command is empty"
    return 1
  }

  rm -f -- "$stdout_path" "$stderr_path"
  : >"$stdout_path"
  : >"$stderr_path"
  chmod 0600 "$stdout_path" "$stderr_path"

  local process_group=0
  if command -v setsid >/dev/null 2>&1 && [[ "$(uname -s)" != MINGW* && "$(uname -s)" != MSYS* && "$(uname -s)" != CYGWIN* ]]; then
    setsid "$@" >"$stdout_path" 2>"$stderr_path" &
    process_group=1
  else
    "$@" >"$stdout_path" 2>"$stderr_path" &
  fi
  local pid="$!"
  local started_at="$SECONDS"
  local failure=""
  local output_bytes
  # Git Bash can briefly expose a shebang child in the shell job table before
  # kill -0 can resolve its MSYS PID. Treat either observation as active so a
  # synchronous wait can never bypass the deadline.
  while remote_fixture_pid_active "$pid" || remote_fixture_job_active "$pid"; do
    output_bytes=$(( $(wc -c <"$stdout_path") + $(wc -c <"$stderr_path") ))
    if ((output_bytes > output_limit_bytes)); then
      failure="exceeded the combined output limit of $output_limit_bytes bytes"
      break
    fi
    if ((SECONDS - started_at >= timeout_seconds)); then
      failure="timed out after $timeout_seconds seconds"
      break
    fi
    sleep 0.05
  done

  if [[ -n "$failure" ]]; then
    remote_fixture_stop_bounded_tree "$label" "$pid" "$process_group" || return 1
    wait "$pid" 2>/dev/null || true
    if [[ "$failure" == exceeded* ]]; then
      remote_fixture_limit_output_files "$output_limit_bytes" "$stdout_path" "$stderr_path" || return 1
    fi
    remote_fixture_fail "$label $failure"
    return 1
  fi

  local status
  if wait "$pid"; then status=0; else status=$?; fi
  output_bytes=$(( $(wc -c <"$stdout_path") + $(wc -c <"$stderr_path") ))
  if ((output_bytes > output_limit_bytes)); then
    remote_fixture_limit_output_files "$output_limit_bytes" "$stdout_path" "$stderr_path" || return 1
    remote_fixture_fail "$label exceeded the combined output limit of $output_limit_bytes bytes"
    return 1
  fi
  return "$status"
}

remote_fixture_limit_output_files() {
  local combined_limit_bytes="$1"
  shift
  remote_fixture_require_command truncate
  local per_file_limit=$((combined_limit_bytes / $#))
  local path
  for path in "$@"; do
    [[ -f "$path" ]] || continue
    if (($(wc -c <"$path") > per_file_limit)); then
      truncate -s "$per_file_limit" -- "$path"
    fi
  done
}

remote_fixture_sha256() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -- "$path" | awk '{print tolower($1)}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -- "$path" | awk '{print tolower($1)}'
  else
    remote_fixture_fail "sha256sum or shasum is required"
  fi
}

remote_fixture_install_cloudflared() {
  local state_dir="$1"
  local supplied_path="${2:-}"
  local executable="$state_dir/tools/cloudflared"
  local expected="$DEVE_REMOTE_FIXTURE_CLOUDFLARED_LINUX_AMD64_SHA256"

  if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
    remote_fixture_fail "pinned cloudflared fixture currently supports Linux x86_64 only"
    return 1
  fi
  mkdir -p -- "$state_dir/tools"
  if [[ -n "$supplied_path" ]]; then
    if [[ ! -f "$supplied_path" || -L "$supplied_path" ]]; then
      remote_fixture_fail "supplied cloudflared must be a regular non-symlink file"
      return 1
    fi
    if (($(wc -c <"$supplied_path") > DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_LIMIT_BYTES)); then
      remote_fixture_fail "supplied cloudflared exceeds the ${DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_LIMIT_BYTES} byte limit"
      return 1
    fi
    cp -- "$supplied_path" "$executable.tmp"
  else
    remote_fixture_require_command curl
    if ! curl --fail --silent --show-error --location \
      --max-time "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_TIMEOUT_SECONDS" \
      --max-filesize "$DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_LIMIT_BYTES" \
      "https://github.com/cloudflare/cloudflared/releases/download/${DEVE_REMOTE_FIXTURE_CLOUDFLARED_VERSION}/cloudflared-linux-amd64" \
      --output "$executable.tmp"; then
      rm -f -- "$executable.tmp"
      remote_fixture_fail "cloudflared download failed within the configured time/size bounds"
      return 1
    fi
  fi
  local observed
  observed="$(remote_fixture_sha256 "$executable.tmp")"
  if [[ "$observed" != "$expected" ]]; then
    rm -f -- "$executable.tmp"
    remote_fixture_fail "cloudflared checksum mismatch: expected $expected, observed $observed"
    return 1
  fi
  chmod 0755 "$executable.tmp"
  mv -f -- "$executable.tmp" "$executable"
  printf '%s\n' "$executable"
}

remote_fixture_wait_http() {
  local url="$1"
  local pid="$2"
  local log_path="$3"
  local attempt
  for attempt in $(seq 1 120); do
    if curl --fail --silent --show-error --max-time 2 "$url" >/dev/null 2>&1; then
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      remote_fixture_fail "process exited before health check succeeded; log: $log_path"
      return 1
    fi
    sleep 0.25
  done
  remote_fixture_fail "timed out waiting for $url; log: $log_path"
}

remote_fixture_wait_tunnel_origin() {
  local pid="$1"
  local stdout_log="$2"
  local stderr_log="$3"
  local attempt origin
  for attempt in $(seq 1 120); do
    origin="$(sed -nE 's#.*(https://[A-Za-z0-9-]+\.trycloudflare\.com).*#\1#p' "$stdout_log" "$stderr_log" 2>/dev/null | head -n 1)"
    if [[ -n "$origin" ]]; then
      remote_fixture_assert_https_origin "$origin"
      printf '%s\n' "$origin"
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      remote_fixture_fail "cloudflared exited before publishing an HTTPS origin"
      return 1
    fi
    sleep 0.25
  done
  remote_fixture_fail "timed out waiting for cloudflared quick-tunnel origin"
}

remote_fixture_write_credentials() {
  local destination="$1"
  local username_file="$2"
  local password_file="$3"
  local auth_secret_file="$4"
  node - "$destination" "$username_file" "$password_file" "$auth_secret_file" <<'NODE'
const fs = require("fs");
const [destination, usernameFile, passwordFile, authSecretFile] = process.argv.slice(2);
const read = (path) => fs.readFileSync(path, "utf8").trim();
fs.writeFileSync(destination, `${JSON.stringify({
  username: read(usernameFile),
  password: read(passwordFile),
  auth_secret: authSecretFile ? read(authSecretFile) : null,
}, null, 2)}\n`, { mode: 0o600 });
NODE
  chmod 0600 "$destination"
}

remote_fixture_json_field() {
  local json_path="$1"
  local field="$2"
  node -e 'const fs=require("fs");const v=JSON.parse(fs.readFileSync(process.argv[1],"utf8"))[process.argv[2]];if(v!==null&&v!==undefined)process.stdout.write(String(v));' "$json_path" "$field"
}

remote_fixture_verify_container_owner() {
  local container="$1"
  local fixture_id="$2"
  local owner
  owner="$(docker inspect --format '{{ index .Config.Labels "deve.remote-fixture-id" }}' "$container" 2>/dev/null || true)"
  if [[ "$owner" != "$fixture_id" ]]; then
    remote_fixture_fail "refusing to remove container without matching fixture owner label: $container"
    return 1
  fi
}

# Returns 0 when the exact container exists, 1 when absent, and 2 when Docker
# could not answer. Callers must treat 2 as a cleanup failure, never absence.
remote_fixture_container_presence() {
  local container="$1"
  local names
  if ! names="$(docker ps --all --filter "name=^/${container}$" --format '{{.Names}}' 2>/dev/null)"; then
    remote_fixture_fail "failed to query Docker while checking owned container: $container"
    return 2
  fi
  [[ "$names" == "$container" ]] && return 0
  [[ -z "$names" ]] && return 1
  remote_fixture_fail "Docker returned an ambiguous exact-name result for: $container"
  return 2
}
