#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
shift || true

fail() {
  echo "release-image-tags: $*" >&2
  exit 1
}

[[ -n "$VERSION" ]] || fail "metadata version is empty"
version_without_build="${VERSION%%+*}"
registry_version="${VERSION/+/_build_}"
if [[ "$version_without_build" == *-* ]]; then
  [[ "$#" -eq 1 ]] || fail "prerelease images must not update latest"
else
  [[ "$#" -eq 2 ]] || fail "stable releases require exactly version and latest image tags"
fi

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

[[ -n "$version_tag" ]] || fail "release tags must contain one version tag"
[[ "$version_tag" == *:"$registry_version" ]] || fail "version tag does not preserve the Docker-safe metadata mapping: $version_tag expected=$registry_version"
if [[ "$version_without_build" != *-* ]]; then
  [[ -n "$latest_tag" ]] || fail "stable release tags must contain latest"
  [[ "${latest_tag%:latest}" == "${version_tag%:"$registry_version"}" ]] || fail "release tags do not share one repository: $latest_tag $version_tag"
fi

printf '%s\n' "$version_tag"
[[ -z "$latest_tag" ]] || printf '%s\n' "$latest_tag"
