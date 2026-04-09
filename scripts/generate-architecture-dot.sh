#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FRAG_DIR="$ROOT_DIR/docs/overview/graph/fragments"
OUT_DOT="$ROOT_DIR/docs/overview/architecture.dot"
OUT_SVG="$ROOT_DIR/docs/overview/architecture.svg"

if [[ ! -d "$FRAG_DIR" ]]; then
  echo "fragment directory not found: $FRAG_DIR" >&2
  exit 1
fi

mapfile -t fragments < <(find "$FRAG_DIR" -maxdepth 1 -type f -name '*.dotfrag' | sort)

if [[ ${#fragments[@]} -eq 0 ]]; then
  echo "no dot fragments found in: $FRAG_DIR" >&2
  exit 1
fi

{
  for fragment in "${fragments[@]}"; do
    cat "$fragment"
    printf '\n'
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
