#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FRAG_DIR="$ROOT_DIR/docs/overview/graph/fragments"
DRIFT_MAP="$ROOT_DIR/docs/overview/graph/drift-map.tsv"
DIFF_FILE="$ROOT_DIR/docs/overview/architecture-diff.md"
DRIFT_OUT="$FRAG_DIR/65_drift_markers.dotfrag"
OUT_DOT="$ROOT_DIR/docs/overview/architecture.dot"
OUT_SVG="$ROOT_DIR/docs/overview/architecture.svg"

if [[ ! -d "$FRAG_DIR" ]]; then
  echo "fragment directory not found: $FRAG_DIR" >&2
  exit 1
fi

if [[ ! -f "$DRIFT_MAP" ]]; then
  echo "drift map not found: $DRIFT_MAP" >&2
  exit 1
fi

if [[ ! -f "$DIFF_FILE" ]]; then
  echo "diff file not found: $DIFF_FILE" >&2
  exit 1
fi

extract_registry() {
  local start="$1"
  local end="$2"
  awk '
    $0 == start { in_block = 1; next }
    $0 == end { in_block = 0 }
    in_block && match($0, /`([^`]+)`/, m) { print m[1] }
  ' start="$start" end="$end" "$DIFF_FILE"
}

mapfile -t flow_ids < <(
  extract_registry "<!-- flow-registry:start -->" "<!-- flow-registry:end -->"
)

if [[ ${#flow_ids[@]} -eq 0 ]]; then
  echo "no flow registry found in: $DIFF_FILE" >&2
  exit 1
fi

for flow_id in "${flow_ids[@]}"; do
  if ! awk -F'\t' -v id="$flow_id" '$1 == id { found = 1 } END { exit !found }' "$DRIFT_MAP"; then
    echo "flow missing from drift map: $flow_id" >&2
    exit 1
  fi
done

mapfile -t drift_ids < <(
  extract_registry "<!-- drift-registry:start -->" "<!-- drift-registry:end -->"
)

if [[ ${#drift_ids[@]} -eq 0 ]]; then
  echo "no drift registry found in: $DIFF_FILE" >&2
  exit 1
fi

if [[ ${#drift_ids[@]} -eq 1 && "${drift_ids[0]}" == "none" ]]; then
  drift_ids=()
fi

for drift_id in "${drift_ids[@]}"; do
  if ! printf '%s\n' "${flow_ids[@]}" | grep -Fxq "$drift_id"; then
    echo "drift flow missing from flow registry: $drift_id" >&2
    exit 1
  fi
done

{
  echo "    // generated drift markers from architecture-diff.md"
  if [[ ${#drift_ids[@]} -eq 0 ]]; then
    echo "    // no active drift markers"
  else
    for drift_id in "${drift_ids[@]}"; do
      spine_root="$(awk -F'\t' -v id="$drift_id" '$1 == id { print $2 }' "$DRIFT_MAP")"
      if [[ -z "$spine_root" ]]; then
        echo "unknown drift flow in registry: $drift_id" >&2
        exit 1
      fi
      echo "    drift_${spine_root} [label=\"*\", shape=circle, width=0.34, height=0.34, fixedsize=true, fillcolor=\"#dc2626\", fontcolor=\"white\", color=\"#991b1b\", fontsize=12, penwidth=1.3];"
      echo "    user_${spine_root}_spine -> drift_${spine_root} [dir=none, style=dashed, color=\"#dc2626\", constraint=false, penwidth=1.2];"
    done
  fi
} > "$DRIFT_OUT"

if [[ ${#drift_ids[@]} -eq 0 ]]; then
  legend_note='clean shared baseline;\nno active drift markers'
else
  legend_note="${#drift_ids[@]} active drift marker(s);\nsee architecture-diff.md"
fi

mapfile -t fragments < <(find "$FRAG_DIR" -maxdepth 1 -type f -name '*.dotfrag' | sort)

if [[ ${#fragments[@]} -eq 0 ]]; then
  echo "no dot fragments found in: $FRAG_DIR" >&2
  exit 1
fi

{
  first_fragment=1
  for fragment in "${fragments[@]}"; do
    if [[ "$first_fragment" -eq 0 ]]; then
      printf '\n'
    fi
    first_fragment=0
    if [[ "$(basename "$fragment")" == "70_legend.dotfrag" ]]; then
      awk -v note="$legend_note" '{ gsub(/__LEGEND_NOTE__/, note); print }' "$fragment"
    else
      cat "$fragment"
    fi
  done
} > "$OUT_DOT"

if command -v dot >/dev/null 2>&1; then
  dot -Tsvg "$OUT_DOT" -o "$OUT_SVG"
  echo "generated:"
  echo "  $OUT_DOT"
  echo "  $OUT_SVG"
else
  echo "generated dot only:"
  echo "  $OUT_DOT"
  echo "warning: graphviz 'dot' not found; svg was not regenerated" >&2
fi
