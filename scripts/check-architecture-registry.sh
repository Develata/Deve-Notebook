#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIFF_FILE="$ROOT_DIR/docs/overview/architecture-diff.md"
DRIFT_MAP="$ROOT_DIR/docs/overview/graph/drift-map.tsv"
GRAPH_FRAG_DIR="$ROOT_DIR/docs/overview/graph/fragments"
DOC_LISP="$ROOT_DIR/docs/overview/architecture-doc.lisp"
CODE_LISP="$ROOT_DIR/docs/overview/architecture-code.lisp"
OPS_DIR="$ROOT_DIR/docs/features/operations"
OP_COVERAGE="$ROOT_DIR/docs/features/operation-coverage.md"
ACCEPTANCE_DIR="$ROOT_DIR/docs/acceptance-cases"

fail() {
  echo "architecture-registry-check: $*" >&2
  exit 1
}

extract_registry() {
  local start="$1"
  local end="$2"
  awk '
    $0 == start { in_block = 1; next }
    $0 == end { in_block = 0 }
    in_block && match($0, /`([^`]+)`/, m) { print m[1] }
  ' start="$start" end="$end" "$DIFF_FILE"
}

[[ -f "$DIFF_FILE" ]] || fail "missing $DIFF_FILE"
[[ -f "$DRIFT_MAP" ]] || fail "missing $DRIFT_MAP"
[[ -d "$GRAPH_FRAG_DIR" ]] || fail "missing $GRAPH_FRAG_DIR"
[[ -f "$DOC_LISP" ]] || fail "missing $DOC_LISP"
[[ -f "$CODE_LISP" ]] || fail "missing $CODE_LISP"
[[ -d "$OPS_DIR" ]] || fail "missing $OPS_DIR"
[[ -f "$OP_COVERAGE" ]] || fail "missing $OP_COVERAGE"
[[ -d "$ACCEPTANCE_DIR" ]] || fail "missing $ACCEPTANCE_DIR"

extract_case_refs() {
  grep -Eo '[A-Z][A-Z0-9-]*-[0-9]+' || true
}

declare -A case_set
while IFS= read -r case_id; do
  [[ -n "$case_id" ]] || continue
  [[ -n "${case_set[$case_id]:-}" ]] && fail "duplicate acceptance case id: $case_id"
  case_set["$case_id"]=1
done < <(rg --no-filename 'case_id:' "$ACCEPTANCE_DIR"/*.md | awk '{ print $3 }')
[[ ${#case_set[@]} -gt 0 ]] || fail "no acceptance case ids found"

mapfile -t flows < <(extract_registry "<!-- flow-registry:start -->" "<!-- flow-registry:end -->")
[[ ${#flows[@]} -gt 0 ]] || fail "flow registry is empty"

declared_count="$(awk -F'`' '/Flow count:/ { print $2; exit }' "$DIFF_FILE")"
[[ "$declared_count" =~ ^[0-9]+$ ]] || fail "Flow count is missing or invalid"
[[ "${#flows[@]}" -eq "$declared_count" ]] || {
  fail "Flow count says $declared_count but registry has ${#flows[@]}"
}

declare -A flow_set
for flow in "${flows[@]}"; do
  [[ -n "${flow_set[$flow]:-}" ]] && fail "duplicate flow registry entry: $flow"
  flow_set["$flow"]=1
  grep -Fq ":label \"$flow\"" "$DOC_LISP" || fail "flow missing in doc lisp: $flow"
  grep -Fq ":label \"$flow\"" "$CODE_LISP" || fail "flow missing in code lisp: $flow"
done

while IFS=$'\t' read -r flow root extra; do
  [[ -n "$flow" ]] || continue
  [[ "$flow" == \#* ]] && continue
  [[ -z "${extra:-}" ]] || fail "drift map row has extra columns: $flow"
  [[ -n "${flow_set[$flow]:-}" ]] || fail "drift map flow not in registry: $flow"
  [[ -n "$root" ]] || fail "drift map root missing for: $flow"
  rg -Fq "user_${root}_spine" "$GRAPH_FRAG_DIR" || fail "spine missing for: $flow -> $root"
done < "$DRIFT_MAP"

for flow in "${flows[@]}"; do
  awk -F'\t' -v flow="$flow" '$1 == flow { found = 1 } END { exit !found }' "$DRIFT_MAP" || {
    fail "flow missing from drift map: $flow"
  }
done

mapfile -t drifts < <(extract_registry "<!-- drift-registry:start -->" "<!-- drift-registry:end -->")
[[ ${#drifts[@]} -gt 0 ]] || fail "drift registry is empty"
if [[ ${#drifts[@]} -eq 1 && "${drifts[0]}" == "none" ]]; then
  drifts=()
fi

active_count="$(awk -F'`' '/Active drift count:/ { print $2; exit }' "$DIFF_FILE")"
[[ "$active_count" =~ ^[0-9]+$ ]] || fail "Active drift count is missing or invalid"
[[ "${#drifts[@]}" -eq "$active_count" ]] || {
  fail "Active drift count says $active_count but registry has ${#drifts[@]}"
}

for drift in "${drifts[@]}"; do
  [[ -n "${flow_set[$drift]:-}" ]] || fail "drift not in flow registry: $drift"
done

while IFS= read -r op_file; do
  base="$(basename "$op_file")"
  [[ "$base" == "00_schema.md" ]] && continue
  rg -Fq "operations/$base" "$DOC_LISP" || fail "operation file not referenced by doc lisp: $base"
  rg -Fq '`Related Acceptance Cases`:' "$op_file" || fail "operation file missing acceptance refs: $base"
  flow_id="$(awk -F'`' '/Flow ID/ { print $4; exit }' "$op_file")"
  [[ -n "$flow_id" ]] || fail "operation file missing Flow ID: $base"
  rg -Fq "| \`$flow_id\` |" "$OP_COVERAGE" || fail "coverage registry missing flow: $flow_id"
  rg -Fq "operations/$base" "$OP_COVERAGE" || fail "coverage registry missing file: $base"
  acceptance_line="$(awk -F': ' '/`Related Acceptance Cases`:/ { print $2; exit }' "$op_file")"
  mapfile -t op_cases < <(printf '%s\n' "$acceptance_line" | extract_case_refs | sort -u)
  [[ ${#op_cases[@]} -gt 0 ]] || fail "operation file has no acceptance case IDs: $base"
  coverage_row="$(awk -F'|' -v id="$flow_id" '$2 ~ "`" id "`" { print; exit }' "$OP_COVERAGE")"
  mapfile -t coverage_cases < <(printf '%s\n' "$coverage_row" | extract_case_refs | sort -u)
  [[ ${#coverage_cases[@]} -gt 0 ]] || fail "coverage row has no acceptance case IDs: $flow_id"
  op_case_list="$(printf '%s\n' "${op_cases[@]}")"
  coverage_case_list="$(printf '%s\n' "${coverage_cases[@]}")"
  [[ "$op_case_list" == "$coverage_case_list" ]] || fail "coverage refs differ for: $base"
  for case_id in "${op_cases[@]}"; do
    [[ -n "${case_set[$case_id]:-}" ]] || fail "acceptance case missing: $case_id in $base"
  done
  mapfile -t op_ids < <(awk 'match($0, /^### `([^`]+)`/, m) { print m[1] }' "$op_file")
  [[ ${#op_ids[@]} -gt 0 ]] || fail "operation file has no operation IDs: $base"
  for op_id in "${op_ids[@]}"; do
    rg -Fq ":id $op_id " "$DOC_LISP" || fail "operation ID missing in doc lisp: $op_id"
    rg -Fq ":id $op_id " "$CODE_LISP" || fail "operation ID missing in code lisp: $op_id"
  done
done < <(find "$OPS_DIR" -maxdepth 1 -type f -name '*.md' | sort)

echo "architecture-registry-check: ok (${#flows[@]} flows, ${#drifts[@]} active drift)"
