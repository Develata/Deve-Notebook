#!/bin/sh
set -eu

# Test-only container bootstrap for the static two-peer mesh fixture.
# The Projection Locator remains the only authority for the workspace path.

repo_id="${DEVE_DOCKER_P2P_MESH_REPO_ID:?missing mesh RepoId}"
repo_key="${DEVE_DOCKER_P2P_MESH_REPO_KEY:?missing mesh RepoKey}"
locator="${DEVE_DOCKER_P2P_MESH_LOCATOR_FILE:-/data/ledger/.host/projection-locators.toml}"
expected_base="${DEVE_DOCKER_P2P_MESH_EXPECTED_PROJECTION_BASE:-/notes}"
initialized=0

if [ ! -f "$locator" ]; then
  deve_cli init --repo default --repo-id "$repo_id" \
    --repo-url urn:mesh:default --projection-base "$expected_base" --path /data
  initialized=1
fi

binding="$(
  awk -F "['\"]" -v repo_id="$repo_id" '
    function finish_record() {
      if (!in_record || record_repo != repo_id) {
        return
      }
      matches += 1
      selected_base = record_base
      selected_segment = record_segment
    }
    /^[[:space:]]*\[\[locators\]\][[:space:]]*$/ {
      finish_record()
      in_record = 1
      record_repo = ""
      record_base = ""
      record_segment = ""
      next
    }
    in_record && $1 ~ /^[[:space:]]*repo_id[[:space:]]*=[[:space:]]*$/ {
      record_repo = $2
      next
    }
    in_record && $1 ~ /^[[:space:]]*workspace_segment[[:space:]]*=[[:space:]]*$/ {
      record_segment = $2
      next
    }
    in_record && $1 ~ /^[[:space:]]*projection_base_abs[[:space:]]*=[[:space:]]*$/ {
      record_base = $2
      next
    }
    END {
      finish_record()
      if (matches != 1 || selected_base == "" || selected_segment == "") {
        exit 2
      }
      printf "%s\n%s\n", selected_base, selected_segment
    }
  ' "$locator"
)" || {
  echo "docker-p2p-mesh-bootstrap: locator binding is missing, duplicate, or incomplete" >&2
  exit 1
}

projection_base="$(printf '%s\n' "$binding" | sed -n '1p')"
workspace_segment="$(printf '%s\n' "$binding" | sed -n '2p')"
[ "$(printf '%s\n' "$binding" | sed -n '$=')" -eq 2 ] \
  || { echo "docker-p2p-mesh-bootstrap: locator binding is not singular" >&2; exit 1; }
[ "$projection_base" = "$expected_base" ] \
  || { echo "docker-p2p-mesh-bootstrap: projection base mismatch" >&2; exit 1; }
printf '%s\n' "$workspace_segment" \
  | grep -Eq '^([A-Za-z0-9._-]+--)?[0-9a-f-]+$' \
  || { echo "docker-p2p-mesh-bootstrap: unsafe workspace segment" >&2; exit 1; }
[ "$workspace_segment" != "." ] && [ "$workspace_segment" != ".." ] \
  || { echo "docker-p2p-mesh-bootstrap: dot workspace segment rejected" >&2; exit 1; }
[ "$(printf '%s' "$repo_key" | wc -c | tr -d ' ')" -eq 32 ] \
  || { echo "docker-p2p-mesh-bootstrap: RepoKey must be exactly 32 bytes" >&2; exit 1; }

workspace="$projection_base/$workspace_segment"
[ -d "$workspace" ] && [ ! -L "$workspace" ] \
  || { echo "docker-p2p-mesh-bootstrap: workspace is missing or linked" >&2; exit 1; }
[ -d "$workspace/.notegit" ] && [ ! -L "$workspace/.notegit" ] \
  || { echo "docker-p2p-mesh-bootstrap: .notegit is missing or linked" >&2; exit 1; }
identity_path="$workspace/.notegit/identity.toml"
[ -f "$identity_path" ] && [ ! -L "$identity_path" ] \
  || { echo "docker-p2p-mesh-bootstrap: identity marker is missing or linked" >&2; exit 1; }
awk -F "['\"]" -v repo_id="$repo_id" '
  /^[[:space:]]*version[[:space:]]*=[[:space:]]*1[[:space:]]*$/ {
    versions += 1
    next
  }
  $1 ~ /^[[:space:]]*repo_id[[:space:]]*=[[:space:]]*$/ {
    repo_ids += 1
    selected_repo = $2
  }
  END { exit !(versions == 1 && repo_ids == 1 && selected_repo == repo_id) }
' "$identity_path" \
  || { echo "docker-p2p-mesh-bootstrap: workspace identity mismatch" >&2; exit 1; }

key_dir="$workspace/.notegit/keys"
if [ -e "$key_dir" ] || [ -L "$key_dir" ]; then
  [ -d "$key_dir" ] && [ ! -L "$key_dir" ] \
    || { echo "docker-p2p-mesh-bootstrap: key directory is not a plain directory" >&2; exit 1; }
else
  mkdir -m 700 "$key_dir"
fi
key_path="$key_dir/repo.key"
if [ -e "$key_path" ] || [ -L "$key_path" ]; then
  [ -f "$key_path" ] && [ ! -L "$key_path" ] \
    || { echo "docker-p2p-mesh-bootstrap: repo key is not a plain file" >&2; exit 1; }
  if printf '%s' "$repo_key" | cmp -s - "$key_path"; then
    chmod 600 "$key_path"
    exec deve_cli serve --port 3001
  fi
  [ "$initialized" -eq 1 ] \
    || { echo "docker-p2p-mesh-bootstrap: existing repo key mismatch" >&2; exit 1; }
fi

umask 077
key_tmp=""
cleanup_key_tmp() {
  if [ -n "$key_tmp" ]; then
    rm -f -- "$key_tmp"
  fi
}
forward_signal() {
  signal="$1"
  status="$2"
  trap - "$signal"
  cleanup_key_tmp
  kill "-$signal" "$$"
  exit "$status"
}
trap cleanup_key_tmp EXIT
trap 'forward_signal HUP 129' HUP
trap 'forward_signal INT 130' INT
trap 'forward_signal TERM 143' TERM
key_tmp="$(mktemp "$key_dir/.repo.key.tmp.XXXXXX")"
[ -f "$key_tmp" ] && [ ! -L "$key_tmp" ] \
  || { echo "docker-p2p-mesh-bootstrap: temporary key is not a plain file" >&2; exit 1; }
printf '%s' "$repo_key" >"$key_tmp"
chmod 600 "$key_tmp"
mv -f -- "$key_tmp" "$key_path"
key_tmp=""
trap - HUP INT TERM
trap - EXIT

exec deve_cli serve --port 3001
