#!/usr/bin/env bash
# plan-coverage-selftest.sh — B0.5 Anchor Contract Upgrade unit tests.
#
# Covers the two acceptance checks for B0.5:
#   (a) the canonical plan_ref pattern accepts both basename and chapter-path
#       anchor forms (and rejects malformed refs);
#   (b) `--rewrite-plan-ref` in dry-run mode does not modify any tracked file.
#
# The pattern under test is extracted from plan-coverage.sh (single source of
# truth) so this test never drifts from the implementation.
#
# Exit codes: 0 — all assertions passed; 1 — at least one assertion failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COVERAGE="$SCRIPT_DIR/plan-coverage.sh"

pass=0
fail=0

# ---------------------------------------------------------------------------
# (a) Canonical pattern accepts basename + chapter-path anchors
# ---------------------------------------------------------------------------
core="$(sed -n "s/^PLAN_REF_CORE='\(.*\)'\$/\1/p" "$COVERAGE" | head -1)"
if [ -z "$core" ]; then
  echo "FAIL: cannot extract PLAN_REF_CORE from $COVERAGE"
  exit 1
fi
re="^${core}\$"

assert_match() {
  if [[ "$1" =~ $re ]]; then
    echo "ok        (match): $1  -- $2"; pass=$((pass + 1))
  else
    echo "FAIL  (want match): $1  -- $2"; fail=$((fail + 1))
  fi
}
assert_reject() {
  if [[ "$1" =~ $re ]]; then
    echo "FAIL (want reject): $1  -- $2"; fail=$((fail + 1))
  else
    echo "ok       (reject): $1  -- $2"; pass=$((pass + 1))
  fi
}

echo "== (a) plan_ref pattern: basename + chapter-path =="
assert_match  "04_repository#repo-selector-resolution-contract"          "basename anchor"
assert_match  "03_storage/authority#facts-partition"                     "chapter-path anchor"
assert_match  "11_ui_design/02_desktop#desktop-native-adapter-contract"  "chapter-path numeric sub"
assert_match  "22_reliability_observability#telemetry-schema"            "long basename"
assert_reject "storage#facts-partition"                                  "missing 2-digit prefix"
assert_reject "03_storage/a/b#x"                                         "two-level sub not allowed"
assert_reject "03_storage/authority#"                                    "empty anchor"
assert_reject "03_storage/authority"                                     "missing anchor"

# ---------------------------------------------------------------------------
# (b) --rewrite-plan-ref dry-run must not modify tracked files
# ---------------------------------------------------------------------------
echo "== (b) --rewrite-plan-ref dry-run leaves files untouched =="
before="$(git -C "$ROOT" status --porcelain | sort)"
"$COVERAGE" --rewrite-plan-ref --from 04_storage# --to 04_storage/authority# >/dev/null 2>&1
after="$(git -C "$ROOT" status --porcelain | sort)"
if [ "$before" = "$after" ]; then
  echo "ok        (dry-run wrote no changes)"; pass=$((pass + 1))
else
  echo "FAIL: --rewrite-plan-ref dry-run modified tracked files:"
  diff <(printf '%s\n' "$before") <(printf '%s\n' "$after") || true
  fail=$((fail + 1))
fi

# ---------------------------------------------------------------------------
# (c) --check-metadata-completeness passes on the current tree (B0 enforcing)
# ---------------------------------------------------------------------------
echo "== (c) --check-metadata-completeness enforcing =="
if out="$("$COVERAGE" --check-metadata-completeness 2>&1)" \
   && printf '%s' "$out" | grep -q '^check-metadata-completeness: OK'; then
  echo "ok        ($out)"; pass=$((pass + 1))
else
  echo "FAIL: --check-metadata-completeness did not pass on current tree:"
  printf '%s\n' "$out"
  fail=$((fail + 1))
fi

echo "----"
echo "selftest: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
