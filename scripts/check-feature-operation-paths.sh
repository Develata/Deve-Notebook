#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCAN_DIRS=(
  "$ROOT_DIR/docs/features/operations"
  "$ROOT_DIR/docs/acceptance-cases"
)

failures=0

while IFS= read -r raw_path; do
  rel_path="${raw_path#$ROOT_DIR/}"

  case "$rel_path" in
    *'*'*|*'{'*|*'}'*|*'['*|*']'*|*'${'*|*'<'*'>'*|*'\"'*)
      continue
      ;;
  esac

  case "$rel_path" in
    apps/*|crates/*|scripts/*|docs/*|.github/*) ;;
    *) continue ;;
  esac

  abs_path="$ROOT_DIR/$rel_path"
  if [[ "$rel_path" == */ ]]; then
    if [[ ! -d "$abs_path" ]]; then
      echo "feature-operation-path-check: missing directory: $rel_path" >&2
      failures=$((failures + 1))
    fi
    continue
  fi

  case "$rel_path" in
    *.rs|*.sh|*.md|*.yml|*.toml|*.json|*.css|*.html|*.lisp|*.tsv|*.js)
      if [[ ! -e "$abs_path" ]]; then
        echo "feature-operation-path-check: missing file: $rel_path" >&2
        failures=$((failures + 1))
      fi
      ;;
  esac
done < <(
  rg -o '`(apps|crates|scripts|docs|\.github)/[^` ]+`' "${SCAN_DIRS[@]}" -g '*.md' |
    sed -E 's/^.*`([^`]+)`.*/\1/' |
    sed 's/[,.]$//' |
    sort -u
)

if (( failures > 0 )); then
  exit 1
fi

echo "feature-operation-path-check: ok"
