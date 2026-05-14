#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GH_BIN="${DEVE_GH_BIN:-gh}"
WORKFLOW_FILE="native-target-host.yml"
RUN_ID="${DEVE_NATIVE_TARGET_HOST_RUN_ID:-${GITHUB_RUN_ID:-}}"
ARTIFACTS="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS:-deve-native-target-host-evidence-macos,deve-native-target-host-evidence-windows,deve-native-target-host-evidence-ios}"
OUT_DIR="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR:-$ROOT_DIR/target/native-target-host-evidence-download}"
COLLECT="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT:-0}"
REF="${DEVE_NATIVE_TARGET_HOST_REF:-}"
REPOSITORY="${DEVE_NATIVE_TARGET_HOST_REPOSITORY:-${GITHUB_REPOSITORY:-}}"
TOKEN="${DEVE_GITHUB_TOKEN:-${GH_TOKEN:-${GITHUB_TOKEN:-}}}"

fail() {
  echo "native-target-host-evidence-collect: $*" >&2
  exit 1
}

resolve_repository() {
  local repo="$REPOSITORY"
  local remote_url

  if [[ -z "$repo" ]]; then
    remote_url="$(git -C "$ROOT_DIR" remote get-url origin 2>/dev/null || true)"
    case "$remote_url" in
      https://github.com/*)
        repo="${remote_url#https://github.com/}"
        repo="${repo%.git}"
        ;;
      git@github.com:*)
        repo="${remote_url#git@github.com:}"
        repo="${repo%.git}"
        ;;
      ssh://git@github.com/*)
        repo="${remote_url#ssh://git@github.com/}"
        repo="${repo%.git}"
        ;;
    esac
  fi

  case "$repo" in
    */*) printf '%s\n' "$repo" ;;
    *) return 1 ;;
  esac
}

python_bin() {
  if command -v python3 >/dev/null 2>&1; then
    command -v python3
    return 0
  fi
  command -v python
}

urlencode() {
  local value="$1"
  local python

  python="$(python_bin)" || fail "python3 or python is required for GitHub API URL encoding"
  "$python" - "$value" <<'PY'
import sys
import urllib.parse

print(urllib.parse.quote(sys.argv[1], safe=""))
PY
}

artifact_names() {
  local artifact
  local -a artifact_array

  IFS=',' read -r -a artifact_array <<<"$ARTIFACTS"
  for artifact in "${artifact_array[@]}"; do
    artifact="${artifact#"${artifact%%[![:space:]]*}"}"
    artifact="${artifact%"${artifact##*[![:space:]]}"}"
    [[ -n "$artifact" ]] && printf '%s\n' "$artifact"
  done
}

artifact_download_url() {
  local list_json="$1"
  local name="$2"
  local python

  python="$(python_bin)" || fail "python3 or python is required for GitHub API artifact parsing"
  "$python" - "$list_json" "$name" <<'PY'
import json
import sys

path, wanted = sys.argv[1:]
with open(path, "r", encoding="utf-8") as handle:
    payload = json.load(handle)
for artifact in payload.get("artifacts", []):
    if artifact.get("name") == wanted:
        print(artifact.get("archive_download_url", ""))
        break
PY
}

latest_run_id_from_json() {
  local runs_json="$1"
  local python

  python="$(python_bin)" || fail "python3 or python is required for GitHub API run parsing"
  "$python" - "$runs_json" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    payload = json.load(handle)
runs = payload.get("workflow_runs", [])
if runs:
    print(runs[0].get("id", ""))
PY
}

validate_artifact_dir() {
  local artifact_dir="$1"
  local found=0
  local report

  while IFS= read -r -d '' report; do
    "$ROOT_DIR/scripts/check-native-target-host-evidence.sh" "$report"
    found=1
  done < <(find "$artifact_dir" -type f -name '*.md' -print0)

  (( found == 1 )) || fail "no evidence Markdown files found in ${artifact_dir#$ROOT_DIR/}"
}

latest_run_with_gh() {
  local repo="$1"

  "$GH_BIN" run list \
    --repo "$repo" \
    --workflow "$WORKFLOW_FILE" \
    --branch "$REF" \
    --event workflow_dispatch \
    --limit 1 \
    --json databaseId \
    --jq '.[0].databaseId'
}

latest_run_with_api() {
  local repo="$1"
  local encoded_ref
  local tmp_dir
  local runs_json
  local run_id

  [[ -n "$TOKEN" ]] || fail "latest run lookup requires DEVE_GITHUB_TOKEN, GH_TOKEN, or GITHUB_TOKEN"
  command -v curl >/dev/null 2>&1 || fail "latest run lookup requires curl"

  encoded_ref="$(urlencode "$REF")"
  tmp_dir="$(mktemp -d)"
  trap "rm -rf '$tmp_dir'" EXIT
  runs_json="$tmp_dir/runs.json"

  curl -fsS \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer $TOKEN" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "https://api.github.com/repos/$repo/actions/workflows/$WORKFLOW_FILE/runs?branch=$encoded_ref&event=workflow_dispatch&per_page=1" \
    >"$runs_json"
  run_id="$(latest_run_id_from_json "$runs_json")"
  [[ -n "$run_id" ]] || fail "no Native Target Host workflow_dispatch runs found for ref: $REF"
  printf '%s\n' "$run_id"
}

resolve_latest_run_id() {
  local repo="$1"

  if command -v "$GH_BIN" >/dev/null 2>&1 && "$GH_BIN" auth status >/dev/null 2>&1; then
    latest_run_with_gh "$repo"
    return
  fi

  latest_run_with_api "$repo"
}

collect_with_gh() {
  local repo="$1"
  local artifact
  local artifact_dir

  while IFS= read -r artifact; do
    artifact_dir="$OUT_DIR/$artifact"
    mkdir -p "$artifact_dir"
    "$GH_BIN" run download "$RUN_ID" --repo "$repo" --name "$artifact" --dir "$artifact_dir"
    validate_artifact_dir "$artifact_dir"
  done < <(artifact_names)

  echo "native-target-host-evidence-collect: downloaded via GitHub CLI: $OUT_DIR"
}

collect_with_api() {
  local repo="$1"
  local tmp_dir
  local list_json
  local artifact
  local artifact_dir
  local download_url
  local zip_path

  [[ -n "$TOKEN" ]] || fail "GitHub API artifact download requires DEVE_GITHUB_TOKEN, GH_TOKEN, or GITHUB_TOKEN"
  command -v curl >/dev/null 2>&1 || fail "GitHub API artifact download requires curl"
  command -v unzip >/dev/null 2>&1 || fail "GitHub API artifact download requires unzip"

  tmp_dir="$(mktemp -d)"
  trap "rm -rf '$tmp_dir'" EXIT
  list_json="$tmp_dir/artifacts.json"

  curl -fsS \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer $TOKEN" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "https://api.github.com/repos/$repo/actions/runs/$RUN_ID/artifacts?per_page=100" \
    >"$list_json"

  while IFS= read -r artifact; do
    download_url="$(artifact_download_url "$list_json" "$artifact")"
    [[ -n "$download_url" ]] || fail "missing artifact in workflow run: $artifact"
    artifact_dir="$OUT_DIR/$artifact"
    zip_path="$tmp_dir/$artifact.zip"
    mkdir -p "$artifact_dir"
    curl -fsSL \
      -H "Accept: application/vnd.github+json" \
      -H "Authorization: Bearer $TOKEN" \
      -H "X-GitHub-Api-Version: 2022-11-28" \
      "$download_url" \
      -o "$zip_path"
    unzip -q -o "$zip_path" -d "$artifact_dir"
    validate_artifact_dir "$artifact_dir"
  done < <(artifact_names)

  echo "native-target-host-evidence-collect: downloaded via GitHub API: $OUT_DIR"
}

if [[ -z "$REF" ]]; then
  REF="$(git -C "$ROOT_DIR" branch --show-current 2>/dev/null || true)"
fi
if [[ -z "$REF" ]]; then
  REF="main"
fi

repo="$(resolve_repository 2>/dev/null || true)"
display_run_id="${RUN_ID:-<run-id>}"

echo "native-target-host-evidence-collect: run_id=$display_run_id"
if [[ "$RUN_ID" == "latest" || -z "$RUN_ID" ]]; then
  if [[ -n "$repo" ]]; then
    encoded_ref="$(urlencode "$REF" 2>/dev/null || printf '%s' "$REF")"
    echo "native-target-host-evidence-collect: latest command: $GH_BIN run list --repo $repo --workflow $WORKFLOW_FILE --branch $REF --event workflow_dispatch --limit 1 --json databaseId"
    echo "native-target-host-evidence-collect: latest api: GET https://api.github.com/repos/$repo/actions/workflows/$WORKFLOW_FILE/runs?branch=$encoded_ref&event=workflow_dispatch&per_page=1"
  else
    echo "native-target-host-evidence-collect: latest command: set DEVE_NATIVE_TARGET_HOST_REPOSITORY=owner/repo for latest run lookup"
  fi
fi
echo "native-target-host-evidence-collect: output=${OUT_DIR#$ROOT_DIR/}"
while IFS= read -r artifact; do
  if [[ -n "$repo" ]]; then
    echo "native-target-host-evidence-collect: command: $GH_BIN run download $display_run_id --repo $repo --name $artifact --dir ${OUT_DIR#$ROOT_DIR/}/$artifact"
  else
    echo "native-target-host-evidence-collect: command: $GH_BIN run download $display_run_id --name $artifact --dir ${OUT_DIR#$ROOT_DIR/}/$artifact"
  fi
done < <(artifact_names)

if [[ "$COLLECT" != "1" ]]; then
  echo "native-target-host-evidence-collect: dry-run; set DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 and DEVE_NATIVE_TARGET_HOST_RUN_ID=<run-id|latest> to download"
  exit 0
fi

[[ -n "$repo" ]] || fail "cannot resolve GitHub repository; set DEVE_NATIVE_TARGET_HOST_REPOSITORY=owner/repo"
[[ -n "$RUN_ID" ]] || fail "DEVE_NATIVE_TARGET_HOST_RUN_ID is required; use a run id or latest"
if [[ "$RUN_ID" == "latest" ]]; then
  RUN_ID="$(resolve_latest_run_id "$repo")"
  echo "native-target-host-evidence-collect: resolved latest run_id=$RUN_ID"
fi
mkdir -p "$OUT_DIR"

if command -v "$GH_BIN" >/dev/null 2>&1 && "$GH_BIN" auth status >/dev/null 2>&1; then
  collect_with_gh "$repo"
  exit 0
fi

collect_with_api "$repo"
