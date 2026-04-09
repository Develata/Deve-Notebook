#!/usr/bin/env bash
# plan-coverage.sh — Plan-Code Bijection Enforcement
#
# Implements Layer 2 (CI Coverage Check) and minimum automated checks
# defined in `docs/plan/AGENTS.md §Plan-Code Bijection Enforcement`.
#
# Exit codes:
#   0 — all checks passed
#   1 — blocking violations found (size fuse, dangling plan_ref, i18n leak)
#   2 — usage / environment error
#
# Output:
#   stdout — human-readable report
#   scripts/plan-coverage.txt (when --write-report) — CI artifact

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLAN_DIR="$ROOT/docs/plan"
CODE_DIRS=("$ROOT/crates" "$ROOT/apps")
FUSE_LINES=250
SOFT_LINES=130
ALLOWLIST="$ROOT/scripts/plan-coverage-allowlist.txt"

is_allowlisted() {
  local rel="$1"
  [ -f "$ALLOWLIST" ] || return 1
  grep -Fxq "$rel" <(grep -v '^\s*#' "$ALLOWLIST" | grep -v '^\s*$')
}
REPORT=""
WRITE_REPORT=0

for arg in "$@"; do
  case "$arg" in
    --write-report) WRITE_REPORT=1 ;;
    -h|--help)
      echo "usage: plan-coverage.sh [--write-report]"; exit 0 ;;
  esac
done

log() { echo "$@"; REPORT+="$*"$'\n'; }
err() { echo "ERROR: $*" >&2; REPORT+="ERROR: $*"$'\n'; }

blocking=0
soft_warnings=0

# ---------------------------------------------------------------------------
# Check 1 — Single-file size fuse (< 250 hard, < 130 soft)
# ---------------------------------------------------------------------------
log "== Check 1: single-file size fuse =="
while IFS= read -r f; do
  lines=$(wc -l < "$f")
  rel="${f#$ROOT/}"
  if [ "$lines" -gt "$FUSE_LINES" ]; then
    if is_allowlisted "$rel"; then
      log "fuse-allowlisted($lines): $rel"
      soft_warnings=$((soft_warnings + 1))
    else
      err "FUSE($lines): $rel"
      blocking=$((blocking + 1))
    fi
  elif [ "$lines" -gt "$SOFT_LINES" ]; then
    log "soft($lines): $f"
    soft_warnings=$((soft_warnings + 1))
  fi
done < <(find "${CODE_DIRS[@]}" -type f -name '*.rs' 2>/dev/null)
log "fuse violations: $blocking, soft warnings: $soft_warnings"
log ""

# ---------------------------------------------------------------------------
# Check 2 — plan_ref annotation scan
# ---------------------------------------------------------------------------
log "== Check 2: plan_ref annotations =="
missing_refs=0
dangling_refs=0
declare -A plan_coverage_map=()

# Build set of valid plan chapter files
valid_chapters=$(find "$PLAN_DIR" -maxdepth 1 -type f -name '*.md' -exec basename {} \;)

while IFS= read -r f; do
  # Only check files declared as modules (mod.rs or lib.rs or named files > 20 lines)
  lines=$(wc -l < "$f")
  [ "$lines" -lt 20 ] && continue
  case "$f" in
    */tests/*|*/target/*) continue ;;
  esac

  if ! grep -q '^//! plan_ref:' "$f" 2>/dev/null; then
    # Soft warning — not blocking yet (bootstrap phase)
    missing_refs=$((missing_refs + 1))
    continue
  fi

  # Extract referenced chapters and verify they exist
  while IFS= read -r ref_line; do
    # Expected shape: "//!   - 04_storage.md §Section Name"
    chapter=$(echo "$ref_line" | sed -n 's|.*- \([0-9][0-9]_[^ ]*\.md\).*|\1|p')
    [ -z "$chapter" ] && continue
    if ! echo "$valid_chapters" | grep -qx "$chapter"; then
      err "dangling plan_ref in $f: $chapter not found in plan"
      dangling_refs=$((dangling_refs + 1))
      blocking=$((blocking + 1))
    else
      section=$(echo "$ref_line" | sed -n 's|.*§\(.*\)$|\1|p')
      key="$chapter§$section"
      plan_coverage_map["$key"]+="$f "
    fi
  done < <(awk '/^\/\/! plan_ref:/{flag=1;next} flag && /^\/\/!/{print; next} {flag=0}' "$f")
done < <(find "${CODE_DIRS[@]}" -type f -name '*.rs' 2>/dev/null)

log "modules without plan_ref (soft): $missing_refs"
log "dangling plan_ref (blocking): $dangling_refs"
log ""

# ---------------------------------------------------------------------------
# Check 3 — i18n facade leak (hardcoded CJK/English in web components)
# ---------------------------------------------------------------------------
log "== Check 3: i18n facade leak =="
i18n_leaks=0
if [ -d "$ROOT/apps/web/src/components" ]; then
  # Heuristic: string literals containing CJK chars outside t::/tr!/L10n macros
  while IFS= read -r hit; do
    echo "$hit" | grep -qE '(//|t::|tr!|L10n|plan_ref|include_str!|r#")' && continue
    log "i18n-leak: $hit"
    i18n_leaks=$((i18n_leaks + 1))
  done < <(grep -rnP '"[^"]*[\x{4e00}-\x{9fff}][^"]*"' "$ROOT/apps/web/src/components" --include='*.rs' 2>/dev/null || true)
fi
# i18n leaks are soft warnings during bootstrap; flip to blocking after cleanup
log "i18n leaks (soft): $i18n_leaks"
log ""

# ---------------------------------------------------------------------------
# Check 4 — Acceptance case ↔ test binding
# ---------------------------------------------------------------------------
log "== Check 4: acceptance case bindings =="
unbound_cases=0
if [ -d "$PLAN_DIR/acceptance-cases" ]; then
  while IFS= read -r case_file; do
    case_id=$(basename "$case_file" .md | tr '[:upper:]' '[:lower:]')
    # Match rust test fn named acc_<id>_* anywhere under code dirs
    if ! grep -rqE "fn ${case_id}(_[a-z0-9_]+)?\s*\(" "${CODE_DIRS[@]}" 2>/dev/null; then
      log "unbound case: $case_id"
      unbound_cases=$((unbound_cases + 1))
    fi
  done < <(find "$PLAN_DIR/acceptance-cases" -maxdepth 1 -type f -name '*.md' 2>/dev/null)
fi
log "unbound acceptance cases (soft): $unbound_cases"
log ""

# ---------------------------------------------------------------------------
# Reverse coverage matrix
# ---------------------------------------------------------------------------
log "== Reverse coverage matrix (plan §section → files) =="
for key in "${!plan_coverage_map[@]}"; do
  log "$key"
  for f in ${plan_coverage_map[$key]}; do
    rel="${f#$ROOT/}"
    log "    $rel"
  done
done
log ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
log "== Summary =="
log "blocking violations: $blocking"
log "soft warnings: $((soft_warnings + missing_refs + i18n_leaks + unbound_cases))"

if [ "$WRITE_REPORT" = "1" ]; then
  printf '%s' "$REPORT" > "$ROOT/scripts/plan-coverage.txt"
  echo "report written to scripts/plan-coverage.txt"
fi

if [ "$blocking" -gt 0 ]; then
  echo "FAILED: $blocking blocking violations" >&2
  exit 1
fi
exit 0
