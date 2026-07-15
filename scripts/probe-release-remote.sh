#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage:
  probe-release-remote.sh github-tag <owner/repo> <tag> <output-json>
  probe-release-remote.sh github-latest <owner/repo> <output-json>
  probe-release-remote.sh ghcr-tag <owner/repo> <tag>

Prints exactly "present" or "absent". HTTP 404 is the only absent state;
transport, authentication, rate-limit, and server failures are fatal.
EOF
  exit 2
}

[[ $# -ge 1 ]] || usage
mode="$1"
shift

: "${GH_TOKEN:?GH_TOKEN is required}"
command -v curl >/dev/null 2>&1 || {
  echo "probe-release-remote: curl is required" >&2
  exit 1
}
command -v jq >/dev/null 2>&1 || {
  echo "probe-release-remote: jq is required" >&2
  exit 1
}

tmp_root="$(mktemp -d)"
cleanup() {
  rm -rf -- "$tmp_root"
}
trap cleanup EXIT INT TERM

require_repository() {
  local repository="$1"
  [[ "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || {
    echo "probe-release-remote: invalid repository name" >&2
    exit 2
  }
}

urlencode() {
  jq -rn --arg value "$1" '$value | @uri'
}

curl_status() {
  local output="$1"
  shift
  local status rc
  set +e
  status="$(curl \
    --silent \
    --show-error \
    --proto '=https' \
    --tlsv1.2 \
    --connect-timeout 15 \
    --max-time 60 \
    --retry 0 \
    --output "$output" \
    --write-out '%{http_code}' \
    "$@")"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    echo "probe-release-remote: transport failed (curl exit $rc)" >&2
    return 1
  fi
  [[ "$status" =~ ^[0-9]{3}$ ]] || {
    echo "probe-release-remote: curl returned an invalid HTTP status" >&2
    return 1
  }
  printf '%s\n' "$status"
}

write_github_headers() {
  local path="$1"
  umask 077
  {
    printf 'Authorization: Bearer %s\n' "$GH_TOKEN"
    printf 'Accept: application/vnd.github+json\n'
    printf 'X-GitHub-Api-Version: 2022-11-28\n'
  } >"$path"
}

probe_github() {
  local repository="$1"
  local endpoint="$2"
  local output_json="$3"
  require_repository "$repository"
  [[ -d "$(dirname -- "$output_json")" && ! -L "$output_json" ]] || {
    echo "probe-release-remote: output parent must exist and output must not be a symlink" >&2
    exit 2
  }

  local headers="$tmp_root/github.headers"
  local body="$tmp_root/github.json"
  write_github_headers "$headers"
  local status
  status="$(curl_status "$body" --header "@$headers" \
    "https://api.github.com/repos/$repository/$endpoint")"
  case "$status" in
    200)
      jq -e 'type == "object"' "$body" >/dev/null || {
        echo "probe-release-remote: GitHub returned a non-object JSON response" >&2
        exit 1
      }
      mv -- "$body" "$output_json"
      printf 'present\n'
      ;;
    404)
      rm -f -- "$output_json"
      printf 'absent\n'
      ;;
    *)
      echo "probe-release-remote: GitHub query failed with HTTP $status" >&2
      exit 1
      ;;
  esac
}

probe_ghcr() {
  local repository="${1,,}"
  local tag="$2"
  require_repository "$repository"
  : "${GITHUB_ACTOR:?GITHUB_ACTOR is required for GHCR probes}"
  [[ -n "$tag" && "$tag" != */* && "$tag" != *:* ]] || {
    echo "probe-release-remote: invalid GHCR tag" >&2
    exit 2
  }

  local basic_headers="$tmp_root/ghcr-basic.headers"
  local basic
  basic="$(printf '%s:%s' "$GITHUB_ACTOR" "$GH_TOKEN" | base64 | tr -d '\r\n')"
  umask 077
  printf 'Authorization: Basic %s\n' "$basic" >"$basic_headers"

  local token_body="$tmp_root/ghcr-token.json"
  local scope token_status
  scope="$(urlencode "repository:$repository:pull")"
  token_status="$(curl_status "$token_body" --header "@$basic_headers" \
    "https://ghcr.io/token?service=ghcr.io&scope=$scope")"
  [[ "$token_status" == 200 ]] || {
    echo "probe-release-remote: GHCR token query failed with HTTP $token_status" >&2
    exit 1
  }
  local bearer
  bearer="$(jq -er '.token // .access_token | select(type == "string" and length > 0)' "$token_body")" || {
    echo "probe-release-remote: GHCR token response did not contain a bearer token" >&2
    exit 1
  }

  local bearer_headers="$tmp_root/ghcr-bearer.headers"
  {
    printf 'Authorization: Bearer %s\n' "$bearer"
    printf 'Accept: application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json\n'
  } >"$bearer_headers"
  local manifest_body="$tmp_root/ghcr-manifest.headers"
  local manifest_status encoded_tag
  encoded_tag="$(urlencode "$tag")"
  manifest_status="$(curl_status "$manifest_body" --head --header "@$bearer_headers" \
    "https://ghcr.io/v2/$repository/manifests/$encoded_tag")"
  case "$manifest_status" in
    200) printf 'present\n' ;;
    404) printf 'absent\n' ;;
    *)
      echo "probe-release-remote: GHCR manifest query failed with HTTP $manifest_status" >&2
      exit 1
      ;;
  esac
}

case "$mode" in
  github-tag)
    [[ $# -eq 3 ]] || usage
    probe_github "$1" "releases/tags/$(urlencode "$2")" "$3"
    ;;
  github-latest)
    [[ $# -eq 2 ]] || usage
    probe_github "$1" "releases/latest" "$2"
    ;;
  ghcr-tag)
    [[ $# -eq 2 ]] || usage
    probe_ghcr "$1" "$2"
    ;;
  *) usage ;;
esac
