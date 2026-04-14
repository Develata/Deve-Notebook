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
  mapfile -t op_ids < <(awk 'match($0, /^### `([^`]+)`/, m) { print m[1] }' "$op_file")
  [[ ${#op_ids[@]} -gt 0 ]] || fail "operation file has no operation IDs: $base"
  for op_id in "${op_ids[@]}"; do
    rg -Fq ":id $op_id " "$DOC_LISP" || fail "operation ID missing in doc lisp: $op_id"
    rg -Fq ":id $op_id " "$CODE_LISP" || fail "operation ID missing in code lisp: $op_id"
  done
done < <(find "$OPS_DIR" -maxdepth 1 -type f -name '*.md' | sort)

echo "architecture-registry-check: ok (${#flows[@]} flows, ${#drifts[@]} active drift)"
