#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'release-tag-binding-check: %s\n' "$*" >&2
  exit 1
}

[[ $# -eq 3 ]] || fail "usage: $0 <tag-name> <tag-object-sha> <peeled-commit-sha>"
tag_name="$1"
expected_tag_object="${2,,}"
expected_commit="${3,,}"
[[ -n "${GITHUB_REPOSITORY:-}" && -n "${GH_TOKEN:-}" ]] || fail "GitHub repository and token are required"
[[ "$expected_tag_object" =~ ^[0-9a-f]{40}$ && "$expected_commit" =~ ^[0-9a-f]{40}$ ]] || fail "expected Git identities must be 40 lowercase hex"

remote_ref="$(gh api "repos/$GITHUB_REPOSITORY/git/ref/tags/$tag_name")"
remote_tag_object="$(jq -er '.object.sha | ascii_downcase' <<<"$remote_ref")"
[[ "$remote_tag_object" == "$expected_tag_object" ]] || fail "remote annotated tag object changed"
remote_tag="$(gh api "repos/$GITHUB_REPOSITORY/git/tags/$remote_tag_object")"
[[ "$(jq -er '.object.type' <<<"$remote_tag")" == commit ]] || fail "annotated tag must directly target a commit"
[[ "$(jq -er '.object.sha | ascii_downcase' <<<"$remote_tag")" == "$expected_commit" ]] || fail "remote annotated tag no longer targets the candidate HEAD"

printf 'release-tag-binding-check: ok\n'
