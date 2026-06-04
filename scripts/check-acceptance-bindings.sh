#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ACCEPTANCE_DIR="$ROOT/docs/acceptance-cases"
ACCEPTANCE_BINDINGS="$ROOT/docs/acceptance-bindings.tsv"
FEATURE_OP_DIR="$ROOT/docs/features/operations"
FEATURE_OP_COVERAGE="$ROOT/docs/features/operation-coverage.md"
CODE_DIRS=("$ROOT/crates" "$ROOT/apps")

errors=0
automated=0
feature=0
manual=0
unbound=0
declare -A case_set=()
declare -A manual_map=()
declare -A automated_map=()
declare -A feature_map=()
case_pattern_file=""

cleanup() {
  [[ -n "$case_pattern_file" ]] && rm -f "$case_pattern_file"
}
trap cleanup EXIT

record_error() {
  echo "ERROR: $*"
  errors=$((errors + 1))
}

record_stale_command_if_present() {
  local pattern="$1"
  if rg --quiet --fixed-strings -- "$pattern" "$ACCEPTANCE_DIR"; then
    record_error "stale acceptance command remains: $pattern"
  fi
}

[[ -d "$ACCEPTANCE_DIR" ]] || {
  record_error "missing acceptance directory: docs/acceptance-cases"
  exit 1
}

while IFS= read -r case_id; do
  [[ -n "$case_id" ]] || continue
  case_set["$case_id"]=1
done < <(grep -RhoE 'case_id: [A-Z][A-Z0-9]*(-[A-Z0-9]+)*' "$ACCEPTANCE_DIR"/*.md 2>/dev/null | awk '{ print $2 }' | sort -u)

case_pattern_file="$(mktemp)"
printf '%s\n' "${!case_set[@]}" | sort > "$case_pattern_file"

if [[ -f "$ACCEPTANCE_BINDINGS" ]]; then
  while IFS='|' read -r case_id binding evidence note; do
    case_id="$(echo "${case_id:-}" | xargs)"
    binding="$(echo "${binding:-}" | xargs)"
    evidence="$(echo "${evidence:-}" | xargs)"
    [[ -z "$case_id" || "$case_id" == \#* ]] && continue
    [[ -n "${case_set[$case_id]:-}" ]] || {
      record_error "acceptance binding references unknown case: $case_id"
      continue
    }
    [[ -z "${manual_map[$case_id]:-}" ]] || {
      record_error "duplicate acceptance binding: $case_id"
      continue
    }
    case "$binding" in
      manual-chrome|manual-cli|manual-doc|manual-network|manual-security) ;;
      *)
        record_error "invalid acceptance binding type for $case_id: $binding"
        continue ;;
    esac
    evidence_path="${evidence%%#*}"
    [[ -n "$evidence_path" && -f "$ROOT/$evidence_path" ]] || {
      record_error "acceptance binding evidence missing for $case_id: $evidence"
      continue
    }
    manual_map["$case_id"]="$binding"
  done < "$ACCEPTANCE_BINDINGS"
fi

while IFS= read -r case_id; do
  [[ -n "$case_id" && -n "${case_set[$case_id]:-}" ]] || continue
  automated_map["$case_id"]=1
done < <(rg --no-filename --only-matching --fixed-strings --file "$case_pattern_file" "${CODE_DIRS[@]}" "$ROOT/tests" "$ROOT/scripts" 2>/dev/null | sort -u || true)

feature_targets=()
[[ -f "$FEATURE_OP_COVERAGE" ]] && feature_targets+=("$FEATURE_OP_COVERAGE")
[[ -d "$FEATURE_OP_DIR" ]] && feature_targets+=("$FEATURE_OP_DIR")
if [[ "${#feature_targets[@]}" -gt 0 ]]; then
  while IFS= read -r case_id; do
    [[ -n "$case_id" && -n "${case_set[$case_id]:-}" ]] || continue
    feature_map["$case_id"]=1
  done < <(rg --no-filename --only-matching --fixed-strings --file "$case_pattern_file" "${feature_targets[@]}" 2>/dev/null | sort -u || true)
fi

while IFS= read -r case_id; do
  [[ -n "$case_id" ]] || continue
  if [[ -n "${automated_map[$case_id]:-}" ]]; then
    automated=$((automated + 1))
  elif [[ -n "${feature_map[$case_id]:-}" ]]; then
    feature=$((feature + 1))
  elif [[ -n "${manual_map[$case_id]:-}" ]]; then
    manual=$((manual + 1))
  else
    echo "unbound case: $case_id"
    unbound=$((unbound + 1))
  fi
done < <(printf '%s\n' "${!case_set[@]}" | sort)

record_stale_command_if_present "deve dump --doc"
record_stale_command_if_present "deve merge --peer"
record_stale_command_if_present "deve auth decode-jwt"
record_stale_command_if_present "deve api call"
record_stale_command_if_present "cargo test -p deve_core path_normalize_structure -- --nocapture"
if rg --quiet -- '--field (doc_id|last_op)' "$ACCEPTANCE_DIR"; then
  record_error "stale acceptance dump --field command remains"
fi

echo "automated acceptance bindings: $automated"
echo "feature walkthrough bindings: $feature"
echo "manual acceptance bindings: $manual"
echo "unbound acceptance cases (soft): $unbound"

[[ "$errors" -eq 0 ]]
