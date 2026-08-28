#!/usr/bin/env bash
# shellcheck shell=bash

# Narrow lifecycle helpers for the RemoteBrowser target-host fixture. Product
# authority remains in deve_cli; this file only owns ephemeral host processes.

readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_VERSION="2026.7.2"
readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_LINUX_AMD64_SHA256="ec905ea7b7e327ff8abdde8cb64697a2152de74dbcdbf6aec9db8364eb3886cd"
readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_WINDOWS_AMD64_SHA256="cdb5d4432f6ae1595654a692a51308b69d2bf7af961f5578d9391837cf072df9"
readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_TIMEOUT_SECONDS="180"
readonly DEVE_REMOTE_FIXTURE_CLOUDFLARED_DOWNLOAD_LIMIT_BYTES="134217728"

REMOTE_FIXTURE_PLATFORM="$(uname -s)" || {
  printf 'remote-browser-fixture: failed to read host platform identity\n' >&2
  return 1
}
readonly REMOTE_FIXTURE_PLATFORM

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

REMOTE_FIXTURE_LINUX_PROCESS_STAT=""
remote_fixture_read_linux_process_stat() {
  local pid="$1"
  local process_stat=""
  REMOTE_FIXTURE_LINUX_PROCESS_STAT=""
  [[ -r "/proc/$pid/stat" ]] || return 1
  # read -d '' consumes the complete proc record, including a legal newline in
  # comm. EOF is expected; an empty value means the PID disappeared before the
  # guarded open/read completed.
  if ! IFS= read -r -d '' process_stat 2>/dev/null <"/proc/$pid/stat"; then
    [[ -n "$process_stat" ]] || return 1
  fi
  REMOTE_FIXTURE_LINUX_PROCESS_STAT="$process_stat"
}

remote_fixture_process_token() {
  local pid="$1"
  if [[ "$REMOTE_FIXTURE_PLATFORM" == MINGW* || "$REMOTE_FIXTURE_PLATFORM" == MSYS* \
    || "$REMOTE_FIXTURE_PLATFORM" == CYGWIN* ]]; then
    # MSYS synthesizes /proc start ticks from Windows data and the value can
    # drift for a live process. Bind the stable native PID, start time, and
    # executable path exposed by ps -W instead.
    local token
    token="$(ps -W 2>/dev/null | awk -v pid="$pid" \
      'NR > 1 && $1 == pid { print $4 ":" $7 ":" $8; exit }')" || return 1
    [[ -n "$token" ]] || return 1
    printf '%s\n' "$token"
    return 0
  fi
  local process_stat process_tail
  local -a process_fields
  remote_fixture_read_linux_process_stat "$pid" || return 1
  process_stat="$REMOTE_FIXTURE_LINUX_PROCESS_STAT"
  # comm is parenthesized and may legally contain whitespace or ')'. Fields
  # after the final ") " have stable positions: state is field 3 and
  # starttime is field 22, hence index 19 in this tail array.
  process_tail="${process_stat##*) }"
  read -r -a process_fields <<<"$process_tail"
  [[ "${process_fields[0]:-}" =~ ^[A-Za-z]$ ]] || return 1
  [[ "${process_fields[19]:-}" =~ ^[0-9]+$ ]] || return 1
  printf '%s\n' "${process_fields[19]}"
}

remote_fixture_live_pid_matches_token() {
  local pid="$1"
  local token="$2"
  remote_fixture_pid_active "$pid" || return 1
  local actual_token
  actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
  [[ -n "$actual_token" && "$actual_token" == "$token" ]]
}

remote_fixture_wait_stable_process_token() {
  local label="$1"
  local pid="$2"
  local previous=""
  local current=""
  local attempt
  for attempt in $(seq 1 100); do
    current="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
    if [[ -n "$current" && "$current" == "$previous" ]]; then
      printf '%s\n' "$current"
      return 0
    fi
    previous="$current"
    if ! remote_fixture_pid_active "$pid" && ! remote_fixture_job_active "$pid"; then
      remote_fixture_fail "$label exited before its process identity stabilized"
      return 1
    fi
    sleep 0.01
  done
  remote_fixture_fail "$label process identity did not stabilize before deadline"
  return 1
}

remote_fixture_pid_exists() {
  kill -0 "$1" 2>/dev/null
}

remote_fixture_pid_active() {
  local pid="$1"
  remote_fixture_pid_exists "$pid" || return 1
  case "$REMOTE_FIXTURE_PLATFORM" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
    *)
      local state process_stat process_tail
      remote_fixture_read_linux_process_stat "$pid" || return 1
      process_stat="$REMOTE_FIXTURE_LINUX_PROCESS_STAT"
      process_tail="${process_stat##*) }"
      state="${process_tail%% *}"
      [[ "$state" != "Z" && "$state" != "X" ]] || return 1
      ;;
  esac
  return 0
}

remote_fixture_pid_terminal() {
  local pid="$1"
  local process_stat process_tail state
  remote_fixture_read_linux_process_stat "$pid" || return 1
  process_stat="$REMOTE_FIXTURE_LINUX_PROCESS_STAT"
  process_tail="${process_stat##*) }"
  state="${process_tail%% *}"
  [[ "$state" == Z || "$state" == X ]]
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

  actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
  if [[ -z "$actual_token" ]]; then
    remote_fixture_fail "refusing to signal $label PID $pid without a live token proof"
    return 1
  fi
  [[ "$actual_token" == "$expected_token" ]] || return 0
  kill -TERM "$pid" 2>/dev/null || true
  local attempt
  for attempt in $(seq 1 50); do
    remote_fixture_pid_active "$pid" || return 0
    actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
    if [[ -z "$actual_token" ]]; then
      remote_fixture_fail "lost $label process token proof during bounded cleanup"
      return 1
    fi
    [[ "$actual_token" == "$expected_token" ]] || return 0
    sleep 0.1
  done
  actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
  if [[ -z "$actual_token" ]]; then
    remote_fixture_fail "refusing to force-stop $label PID $pid without a live token proof"
    return 1
  fi
  [[ "$actual_token" == "$expected_token" ]] || return 0
  kill -KILL "$pid" 2>/dev/null || true
  for attempt in $(seq 1 20); do
    remote_fixture_pid_active "$pid" || return 0
    actual_token="$(remote_fixture_process_token "$pid" 2>/dev/null || true)"
    if [[ -z "$actual_token" ]]; then
      remote_fixture_fail "lost $label process token proof after force-stop"
      return 1
    fi
    [[ "$actual_token" == "$expected_token" ]] || return 0
    sleep 0.1
  done
  remote_fixture_fail "$label PID $pid survived bounded cleanup"
}

REMOTE_FIXTURE_PROCESS_TREE_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-process-tree.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-process-tree.sh
source "$REMOTE_FIXTURE_PROCESS_TREE_LIB"

REMOTE_FIXTURE_BOUNDED_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-bounded.sh"
# shellcheck source=scripts/lib/remote-browser-fixture-bounded.sh
source "$REMOTE_FIXTURE_BOUNDED_LIB"

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
  local platform executable expected asset
  platform="$REMOTE_FIXTURE_PLATFORM"
  case "$platform:$(uname -m)" in
    Linux:x86_64)
      executable="$state_dir/tools/cloudflared"
      expected="$DEVE_REMOTE_FIXTURE_CLOUDFLARED_LINUX_AMD64_SHA256"
      asset="cloudflared-linux-amd64"
      ;;
    MINGW*:x86_64 | MSYS*:x86_64 | CYGWIN*:x86_64)
      executable="$state_dir/tools/cloudflared.exe"
      expected="$DEVE_REMOTE_FIXTURE_CLOUDFLARED_WINDOWS_AMD64_SHA256"
      asset="cloudflared-windows-amd64.exe"
      ;;
    *)
      remote_fixture_fail "pinned cloudflared fixture supports Linux or Windows x86_64 only"
      return 1
      ;;
  esac
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
      "https://github.com/cloudflare/cloudflared/releases/download/${DEVE_REMOTE_FIXTURE_CLOUDFLARED_VERSION}/${asset}" \
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

# shellcheck source=scripts/lib/remote-browser-fixture-http.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/remote-browser-fixture-http.sh"
