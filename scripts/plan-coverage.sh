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
FUSE_LINES=500
SOFT_LINES=250
ALLOWLIST="$ROOT/scripts/plan-coverage-allowlist.txt"
I18N_ALLOWLIST="$ROOT/scripts/i18n-coverage-allowlist.txt"

is_allowlisted() {
  local rel="$1"
  [ -f "$ALLOWLIST" ] || return 1
  grep -Fxq "$rel" <(grep -v '^\s*#' "$ALLOWLIST" | grep -v '^\s*$')
}

is_i18n_allowlisted() {
  local rel_hit="$1"
  [ -f "$I18N_ALLOWLIST" ] || return 1
  grep -Fxq "$rel_hit" <(grep -v '^\s*#' "$I18N_ALLOWLIST" | grep -v '^\s*$')
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
# Check 1 — Single-file size fuse (> 500 hard, > 250 soft)
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

  # Extract `<chapter_basename>#<stable-anchor-id>` refs and verify both parts.
  while IFS= read -r ref_line; do
    ref=$(echo "$ref_line" | sed -n 's|^//! *- *\([^[:space:]]\+\).*$|\1|p')
    [ -z "$ref" ] && continue
    [ "$ref" = "infra" ] && continue

    if ! [[ "$ref" =~ ^[0-9][0-9]_[A-Za-z0-9_]+#[A-Za-z0-9_-]+$ ]]; then
      err "invalid plan_ref in ${f#$ROOT/}: $ref"
      dangling_refs=$((dangling_refs + 1))
      blocking=$((blocking + 1))
      continue
    fi

    chapter="${ref%%#*}"
    anchor="${ref#*#}"
    chapter_file="$PLAN_DIR/$chapter.md"
    if [ ! -f "$chapter_file" ]; then
      err "dangling plan_ref in ${f#$ROOT/}: $chapter.md not found in plan"
      dangling_refs=$((dangling_refs + 1))
      blocking=$((blocking + 1))
    elif ! grep -Fq "{#$anchor}" "$chapter_file"; then
      err "dangling plan_ref in ${f#$ROOT/}: anchor $ref not found"
      dangling_refs=$((dangling_refs + 1))
      blocking=$((blocking + 1))
    else
      key="$ref"
      plan_coverage_map["$key"]+="$f "
    fi
  done < <(awk '/^\/\/! plan_ref:/{flag=1;next} flag && /^\/\/! *- /{print; next} flag {flag=0}' "$f")
done < <(find "${CODE_DIRS[@]}" -type f -name '*.rs' 2>/dev/null)

log "modules without plan_ref (soft): $missing_refs"
log "dangling plan_ref (blocking): $dangling_refs"
log ""

# ---------------------------------------------------------------------------
# Check 3 — i18n facade leak (hardcoded CJK/English in web components)
# ---------------------------------------------------------------------------
log "== Check 3: i18n facade leak =="
i18n_leaks=0
i18n_allowlisted=0
if [ -d "$ROOT/apps/web/src/components" ]; then
  # Heuristic: string literals containing CJK chars outside t::/tr!/L10n macros
  while IFS= read -r hit; do
    echo "$hit" | grep -qE '(//|t::|tr!|L10n|plan_ref|include_str!|r#")' && continue
    rel_hit="${hit#$ROOT/}"
    if is_i18n_allowlisted "$rel_hit"; then
      log "i18n-allowlisted: $rel_hit"
      i18n_allowlisted=$((i18n_allowlisted + 1))
    else
      err "i18n-leak: $rel_hit"
      i18n_leaks=$((i18n_leaks + 1))
      blocking=$((blocking + 1))
    fi
  done < <(grep -rnP '"[^"]*[\x{4e00}-\x{9fff}][^"]*"' "$ROOT/apps/web/src/components" --include='*.rs' 2>/dev/null || true)
fi
log "i18n leaks (blocking): $i18n_leaks"
log "i18n allowlisted debt: $i18n_allowlisted"
log ""

# ---------------------------------------------------------------------------
# Check 4 — Acceptance case ↔ test binding
# ---------------------------------------------------------------------------
log "== Check 4: acceptance case bindings =="
unbound_cases=0
binding_status=0
binding_report="$(bash "$ROOT/scripts/check-acceptance-bindings.sh" 2>&1)" || binding_status=$?
while IFS= read -r line; do
  [ -z "$line" ] && continue
  if [[ "$line" == ERROR:* ]]; then
    err "${line#ERROR: }"
  else
    log "$line"
  fi
done <<< "$binding_report"
unbound_cases="$(printf '%s\n' "$binding_report" | awk -F': ' '/^unbound acceptance cases/ { print $2; exit }')"
unbound_cases="${unbound_cases:-0}"
if [ "$binding_status" -ne 0 ]; then
  blocking=$((blocking + 1))
fi
log ""

# ---------------------------------------------------------------------------
# Reverse coverage matrix
# ---------------------------------------------------------------------------
log "== Reverse coverage matrix (plan anchor → files) =="
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
