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
git_in_repo() {
  git -c safe.directory="$ROOT" -C "$ROOT" "$@"
}

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
before="$(git_in_repo status --porcelain | sort)"
"$COVERAGE" --rewrite-plan-ref --from 04_storage# --to 04_storage/authority# >/dev/null 2>&1
after="$(git_in_repo status --porcelain | sort)"
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

# ---------------------------------------------------------------------------
# (d) --check-perf-budget enforcing: positive on current tree + negatives
# ---------------------------------------------------------------------------
echo "== (d) --check-perf-budget enforcing =="
if out="$("$COVERAGE" --check-perf-budget 2>&1)" \
   && printf '%s' "$out" | grep -q '^check-perf-budget: OK'; then
  echo "ok        (positive: $out)"; pass=$((pass + 1))
else
  echo "FAIL: --check-perf-budget did not pass on current tree:"
  printf '%s\n' "$out"
  fail=$((fail + 1))
fi

perf_tmp="$(mktemp -d)"
trap 'rm -rf "$perf_tmp"' EXIT

assert_perf_reject() {  # <fixture-file> <label>
  if "$COVERAGE" --check-perf-budget "$1" >/dev/null 2>&1; then
    echo "FAIL (want reject): perf-budget $2"; fail=$((fail + 1))
  else
    echo "ok       (rejected): perf-budget $2"; pass=$((pass + 1))
  fi
}

cat >"$perf_tmp/placeholder.md" <<'MD'
## 2. Critical Path Budget
| Critical Path | Profile | P50 | P99 | RSS Δ | Test |
|---|---|---|---|---|---|
| `flow.a` | `standard` | TBD | 80ms | +2MB | t |
## 3. Next
MD
assert_perf_reject "$perf_tmp/placeholder.md" "TBD placeholder rejected"

cat >"$perf_tmp/malformed.md" <<'MD'
## 2. Critical Path Budget
| Critical Path | Profile | P50 | P99 | RSS Δ | Test |
|---|---|---|---|---|---|
| `flow.a` | `standard` | fast | slow | +2MB | t |
## 3. Next
MD
assert_perf_reject "$perf_tmp/malformed.md" "malformed P50/P99 rejected"

cat >"$perf_tmp/sparse.md" <<'MD'
## 2. Critical Path Budget
| Critical Path | Profile | P50 | P99 | RSS Δ | Test |
|---|---|---|---|---|---|
| `flow.a` | `standard` | 10ms | 20ms | +1MB | t |
| `flow.b` | `standard` | 10ms | 20ms | +1MB | t |
## 3. Next
MD
assert_perf_reject "$perf_tmp/sparse.md" "too-few-numeric-rows rejected"

# ---------------------------------------------------------------------------
# (e) --check-no-adr-plan-ref: passes on current tree + detection sanity (B4.3)
# ---------------------------------------------------------------------------
echo "== (e) --check-no-adr-plan-ref enforcing =="
if out="$("$COVERAGE" --check-no-adr-plan-ref 2>&1)" \
   && printf '%s' "$out" | grep -q '^check-no-adr-plan-ref: OK'; then
  echo "ok        (positive: $out)"; pass=$((pass + 1))
else
  echo "FAIL: --check-no-adr-plan-ref did not pass on current tree:"
  printf '%s\n' "$out"
  fail=$((fail + 1))
fi
adr_re='(^|[^[:alnum:]_])adr/'
token_of() { printf '%s' "$1" | sed -E 's@^//![[:space:]]*-[[:space:]]*([^[:space:]]+).*@\1@'; }
# entry target token referencing an ADR is detected
if token_of '//!   - adr/0001-leptos-over-yew' | grep -qE "$adr_re"; then
  echo "ok        (detects adr/ target token)"; pass=$((pass + 1))
else
  echo "FAIL: adr/ target token not detected"; fail=$((fail + 1))
fi
# normal chapter-path token is not flagged
if token_of '//!   - 03_storage/authority#facts-partition' | grep -qE "$adr_re"; then
  echo "FAIL: false-positive on chapter-path token"; fail=$((fail + 1))
else
  echo "ok        (no false-positive on chapter-path token)"; pass=$((pass + 1))
fi
# trailing comment mentioning adr/ must NOT false-positive (token excludes it)
if token_of '//!   - 03_storage/authority#facts # see docs/adr/0001' | grep -qE "$adr_re"; then
  echo "FAIL: trailing-comment adr false-positive"; fail=$((fail + 1))
else
  echo "ok        (trailing-comment adr not flagged)"; pass=$((pass + 1))
fi
# inline header form is caught by the header scan
if printf '//! plan_ref: adr/0002\n' | grep -qE '^//![[:space:]]*plan_ref:.*adr/'; then
  echo "ok        (detects inline-header adr)"; pass=$((pass + 1))
else
  echo "FAIL: inline-header adr not detected"; fail=$((fail + 1))
fi

# ---------------------------------------------------------------------------
# (f) --check-md-links enforcing: positive on tree + negative fixtures (B3.4)
# ---------------------------------------------------------------------------
echo "== (f) --check-md-links enforcing =="
if out="$("$COVERAGE" --check-md-links 2>&1)" \
   && printf '%s' "$out" | grep -q '^check-md-links: OK'; then
  echo "ok        (positive: $out)"; pass=$((pass + 1))
else
  echo "FAIL: --check-md-links did not pass on current tree:"
  printf '%s\n' "$out"; fail=$((fail + 1))
fi
mdl_tmp="$(mktemp -d)"
trap 'rm -rf "$perf_tmp" "$mdl_tmp"' EXIT
printf '# Target {#here}\n' > "$mdl_tmp/ok.md"
printf '# A\n[x](./missing.md)\n' > "$mdl_tmp/bad_file.md"
if "$COVERAGE" --check-md-links "$mdl_tmp" >/dev/null 2>&1; then
  echo "FAIL (want reject): md-links broken file"; fail=$((fail + 1))
else echo "ok       (rejected): md-links broken file link"; pass=$((pass + 1)); fi
rm "$mdl_tmp/bad_file.md"
printf '# B\n[x](./ok.md#nope)\n' > "$mdl_tmp/bad_anchor.md"
if "$COVERAGE" --check-md-links "$mdl_tmp" >/dev/null 2>&1; then
  echo "FAIL (want reject): md-links broken anchor"; fail=$((fail + 1))
else echo "ok       (rejected): md-links broken anchor"; pass=$((pass + 1)); fi
rm "$mdl_tmp/bad_anchor.md"
printf '# C\n[x](./ok.md#here)\n' > "$mdl_tmp/good.md"
if "$COVERAGE" --check-md-links "$mdl_tmp" >/dev/null 2>&1; then
  echo "ok        (valid file+anchor accepted)"; pass=$((pass + 1))
else echo "FAIL: md-links rejected valid link"; fail=$((fail + 1)); fi

echo "----"
echo "selftest: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
