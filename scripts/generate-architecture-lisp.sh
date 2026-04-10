#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LISP_DIR="$ROOT_DIR/docs/overview/lisp"
DOC_FRAG_DIR="$LISP_DIR/doc_fragments"
CODE_FRAG_DIR="$LISP_DIR/code_fragments"
OUT_DOC="$ROOT_DIR/docs/overview/architecture-doc.lisp"
OUT_CODE="$ROOT_DIR/docs/overview/architecture-code.lisp"

concat_fragments() {
  local frag_dir="$1"
  local output="$2"
  local label="$3"
  local -a fragments

  if [[ ! -d "$frag_dir" ]]; then
    echo "$label fragment directory not found: $frag_dir" >&2
    exit 1
  fi

  mapfile -t fragments < <(find "$frag_dir" -maxdepth 1 -type f -name '*.lispfrag' | sort)

  if [[ ${#fragments[@]} -eq 0 ]]; then
    echo "no $label fragments found in: $frag_dir" >&2
    exit 1
  fi

  {
    for fragment in "${fragments[@]}"; do
      cat "$fragment"
      printf '\n'
    done
  } > "$output"
}

concat_fragments "$DOC_FRAG_DIR" "$OUT_DOC" "doc"
concat_fragments "$CODE_FRAG_DIR" "$OUT_CODE" "code"

echo "generated:"
echo "  $OUT_DOC"
echo "  $OUT_CODE"
