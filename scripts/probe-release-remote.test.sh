#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d)"
cleanup() {
  rm -rf -- "$tmp_root"
}
trap cleanup EXIT INT TERM

mkdir -p "$tmp_root/bin"
cat >"$tmp_root/bin/curl" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
output=
url=
while (($#)); do
  case "$1" in
    --output)
      output="$2"
      shift 2
      ;;
    http://*|https://*)
      url="$1"
      shift
      ;;
    *) shift ;;
  esac
done
[[ -n "$output" && -n "$url" ]]

case "${DEVE_FAKE_CURL_SCENARIO:?}" in
  network)
    exit 7
    ;;
esac

if [[ "$url" == https://api.github.com/* ]]; then
  case "$DEVE_FAKE_CURL_SCENARIO" in
    github-present) printf '{"draft":true,"tag_name":"v0.1.0"}\n' >"$output"; printf 200 ;;
    github-absent) printf '{"message":"Not Found"}\n' >"$output"; printf 404 ;;
    github-transient) printf '{"message":"Unavailable"}\n' >"$output"; printf 503 ;;
    *) exit 90 ;;
  esac
elif [[ "$url" == https://ghcr.io/token* ]]; then
  case "$DEVE_FAKE_CURL_SCENARIO" in
    ghcr-token-transient) printf '{"errors":[]}\n' >"$output"; printf 503 ;;
    ghcr-*) printf '{"token":"fixture-token"}\n' >"$output"; printf 200 ;;
    *) exit 91 ;;
  esac
elif [[ "$url" == https://ghcr.io/v2/* ]]; then
  case "$DEVE_FAKE_CURL_SCENARIO" in
    ghcr-present) : >"$output"; printf 200 ;;
    ghcr-absent) : >"$output"; printf 404 ;;
    ghcr-transient) : >"$output"; printf 502 ;;
    *) exit 92 ;;
  esac
else
  exit 93
fi
FAKE
chmod +x "$tmp_root/bin/curl"

probe=(bash "$repo_root/scripts/probe-release-remote.sh")
common_env=(env "PATH=$tmp_root/bin:$PATH" GH_TOKEN=fixture-token GITHUB_ACTOR=fixture-actor)

output="$tmp_root/release.json"
state="$(DEVE_FAKE_CURL_SCENARIO=github-present "${common_env[@]}" "${probe[@]}" \
  github-tag owner/repo v0.1.0 "$output")"
[[ "$state" == present ]]
jq -e '.draft == true' "$output" >/dev/null

state="$(DEVE_FAKE_CURL_SCENARIO=github-absent "${common_env[@]}" "${probe[@]}" \
  github-latest owner/repo "$output")"
[[ "$state" == absent && ! -e "$output" ]]

if DEVE_FAKE_CURL_SCENARIO=github-transient "${common_env[@]}" "${probe[@]}" \
  github-latest owner/repo "$output" >/dev/null 2>&1; then
  echo "GitHub 503 must fail closed" >&2
  exit 1
fi
if DEVE_FAKE_CURL_SCENARIO=network "${common_env[@]}" "${probe[@]}" \
  github-latest owner/repo "$output" >/dev/null 2>&1; then
  echo "GitHub transport failure must fail closed" >&2
  exit 1
fi

state="$(DEVE_FAKE_CURL_SCENARIO=ghcr-present "${common_env[@]}" "${probe[@]}" \
  ghcr-tag owner/repo 0.1.0)"
[[ "$state" == present ]]
state="$(DEVE_FAKE_CURL_SCENARIO=ghcr-absent "${common_env[@]}" "${probe[@]}" \
  ghcr-tag owner/repo latest)"
[[ "$state" == absent ]]

for scenario in ghcr-transient ghcr-token-transient; do
  if DEVE_FAKE_CURL_SCENARIO="$scenario" "${common_env[@]}" "${probe[@]}" \
    ghcr-tag owner/repo latest >/dev/null 2>&1; then
    echo "$scenario must fail closed" >&2
    exit 1
  fi
done

echo "release remote probe tests passed"
