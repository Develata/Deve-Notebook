#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
shift || true

fail() {
  echo "release-image-tags: $*" >&2
  exit 1
}

[[ -n "$VERSION" ]] || fail "metadata version is empty"
[[ "$#" -eq 2 ]] || fail "expected exactly version and latest image tags before push, got $#"

latest_tag=""
version_tag=""
for tag in "$@"; do
  if [[ "$tag" == *:latest ]]; then
    [[ -z "$latest_tag" ]] || fail "duplicate latest tag: $tag"
    latest_tag="$tag"
  else
    [[ -z "$version_tag" ]] || fail "multiple version tags: $version_tag $tag"
    version_tag="$tag"
  fi
done

[[ -n "$latest_tag" && -n "$version_tag" ]] || fail "release tags must contain one latest and one version tag"
[[ "$version_tag" == *:"$VERSION" ]] || fail "version tag does not end with metadata version: $version_tag expected=$VERSION"
[[ "${latest_tag%:latest}" == "${version_tag%:"$VERSION"}" ]] || fail "release tags do not share one repository: $latest_tag $version_tag"

printf '%s\n%s\n' "$version_tag" "$latest_tag"
