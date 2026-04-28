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
#   --list-missing-plan-ref — include non-exempt missing plan_ref paths
#   --summary-missing-plan-ref — include grouped missing plan_ref counts

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLAN_DIR="$ROOT/docs/plan"
CODE_DIRS=("$ROOT/crates" "$ROOT/apps")
FUSE_LINES=500
SOFT_LINES=250
ALLOWLIST="$ROOT/scripts/plan-coverage-allowlist.txt"
I18N_ALLOWLIST="$ROOT/scripts/i18n-coverage-allowlist.txt"
I18N_CJK_SCAN_DIRS=("$ROOT/apps/web/src/components")
I18N_EXACT_SCAN_DIRS=("$ROOT/apps/web/src/components" "$ROOT/apps/web/src/editor")
I18N_FORBIDDEN_ENGLISH_LITERALS=(
  '"Pin"'
  '"Unpin"'
  '"Toggle Outline"'
  '"Spectator Mode (Read Only)"'
  '"No repo selected"'
  'docs in current repo'
  'Up/Down to navigate'
)

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

is_plan_ref_missing_exempt() {
  local rel="$1"
  case "$rel" in
    */target/*|*/tests/*|*/benches/*) return 0 ;;
    *_test.rs|*_test_*.rs|*_test_support.rs|*/tests.rs|*/test_modules.rs) return 0 ;;
    */channel_test/*|*/switcher_prepare_test/*) return 0 ;;
    */generated/*|*_generated.rs|*/vendor/*|*/public/*|*/dist/*) return 0 ;;
  esac
  return 1
}
REPORT=""
WRITE_REPORT=0
LIST_MISSING_PLAN_REF=0
SUMMARY_MISSING_PLAN_REF=0
MISSING_PLAN_REF_SUMMARY_TOP=20

usage() {
  echo "usage: plan-coverage.sh [--write-report] [--list-missing-plan-ref] [--summary-missing-plan-ref]"
}

for arg in "$@"; do
  case "$arg" in
    --write-report) WRITE_REPORT=1 ;;
    --list-missing-plan-ref) LIST_MISSING_PLAN_REF=1 ;;
    --summary-missing-plan-ref) SUMMARY_MISSING_PLAN_REF=1 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "ERROR: unknown argument: $arg" >&2
      usage >&2
      exit 2 ;;
  esac
done

log() { echo "$@"; REPORT+="$*"$'\n'; }
err() { echo "ERROR: $*" >&2; REPORT+="ERROR: $*"$'\n'; }

log_missing_plan_ref_groups() {
  local title="$1"
  local depth="$2"
  local limit="$3"

  log "$title"
  if [ "${#missing_ref_files[@]}" -eq 0 ]; then
    log "    (none)"
    return
  fi

  while IFS= read -r line; do
    [ -z "$line" ] && continue
    log "    $line"
  done < <(
    printf '%s\n' "${missing_ref_files[@]}" |
      awk -F/ -v depth="$depth" '
        NF {
          key = $1
          for (i = 2; i <= depth && i <= NF; i++) {
            key = key "/" $i
          }
          counts[key]++
        }
        END {
          for (key in counts) {
            print counts[key] " " key
          }
        }
      ' |
      sort -rn -k1,1 |
      head -n "$limit"
  )
}

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
done < <(find "${CODE_DIRS[@]}" -type f -name '*.rs' 2>/dev/null | sort)
log "fuse violations: $blocking, soft warnings: $soft_warnings"
log ""

# ---------------------------------------------------------------------------
# Check 2 — plan_ref annotation scan
# ---------------------------------------------------------------------------
log "== Check 2: plan_ref annotations =="
missing_refs=0
missing_refs_exempt=0
dangling_refs=0
annotated_refs=0
missing_ref_files=()
declare -A plan_coverage_map=()

while IFS= read -r f; do
  rel="${f#$ROOT/}"

  if ! grep -q '^//! plan_ref:' "$f" 2>/dev/null; then
    # Missing annotations are actionable only for non-test source modules.
    lines=$(wc -l < "$f")
    [ "$lines" -lt 20 ] && continue
    if is_plan_ref_missing_exempt "$rel"; then
      missing_refs_exempt=$((missing_refs_exempt + 1))
      continue
    fi
    missing_refs=$((missing_refs + 1))
    missing_ref_files+=("$rel")
    continue
  fi
  annotated_refs=$((annotated_refs + 1))

  # Extract `<chapter_basename>#<stable-anchor-id>` refs and verify both parts.
  while IFS= read -r ref_line; do
    ref=$(echo "$ref_line" | sed -n 's|^//! *- *\([^[:space:]]\+\).*$|\1|p')
    [ -z "$ref" ] && continue
    [ "$ref" = "infra" ] && continue

    if ! [[ "$ref" =~ ^[0-9][0-9]_[A-Za-z0-9_]+#[A-Za-z0-9_-]+$ ]]; then
      err "invalid plan_ref in $rel: $ref"
      dangling_refs=$((dangling_refs + 1))
      blocking=$((blocking + 1))
      continue
    fi

    chapter="${ref%%#*}"
    anchor="${ref#*#}"
    chapter_file="$PLAN_DIR/$chapter.md"
    if [ ! -f "$chapter_file" ]; then
      err "dangling plan_ref in $rel: $chapter.md not found in plan"
      dangling_refs=$((dangling_refs + 1))
      blocking=$((blocking + 1))
    elif ! grep -Fq "{#$anchor}" "$chapter_file"; then
      err "dangling plan_ref in $rel: anchor $ref not found"
      dangling_refs=$((dangling_refs + 1))
      blocking=$((blocking + 1))
    else
      key="$ref"
      plan_coverage_map["$key"]+="$f "
    fi
  done < <(awk '/^\/\/! plan_ref:/{flag=1;next} flag && /^\/\/! *- /{print; next} flag {flag=0}' "$f")
done < <(find "${CODE_DIRS[@]}" -type f -name '*.rs' 2>/dev/null | sort)

log "modules with plan_ref: $annotated_refs"
log "modules without plan_ref (soft): $missing_refs"
log "modules without plan_ref (exempt): $missing_refs_exempt"
log "dangling plan_ref (blocking): $dangling_refs"
if [ "$LIST_MISSING_PLAN_REF" = "1" ]; then
  log "missing plan_ref files (soft):"
  if [ "${#missing_ref_files[@]}" -eq 0 ]; then
    log "    (none)"
  else
    for rel in "${missing_ref_files[@]}"; do
      log "    $rel"
    done
  fi
fi
if [ "$SUMMARY_MISSING_PLAN_REF" = "1" ]; then
  log "missing plan_ref summary (soft):"
  log "total: $missing_refs"
  log_missing_plan_ref_groups "by workspace member:" 2 "$MISSING_PLAN_REF_SUMMARY_TOP"
  log_missing_plan_ref_groups "by directory depth 4 (top $MISSING_PLAN_REF_SUMMARY_TOP):" 4 "$MISSING_PLAN_REF_SUMMARY_TOP"
fi
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
  done < <(grep -rnP --include='*.rs' '"[^"]*[\x{4e00}-\x{9fff}][^"]*"' "${I18N_CJK_SCAN_DIRS[@]}" 2>/dev/null || true)

  # Exact regression guard for English literals already migrated to t::*.
  # A broad English detector is too noisy for class names and protocol strings;
  # this targeted list blocks the UI copy leaks found by the current gap scan.
  for literal in "${I18N_FORBIDDEN_ENGLISH_LITERALS[@]}"; do
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
    done < <(grep -rnF --include='*.rs' "$literal" "${I18N_EXACT_SCAN_DIRS[@]}" 2>/dev/null || true)
  done
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
log "soft warnings: $((soft_warnings + missing_refs + unbound_cases))"

if [ "$WRITE_REPORT" = "1" ]; then
  printf '%s' "$REPORT" > "$ROOT/scripts/plan-coverage.txt"
  echo "report written to scripts/plan-coverage.txt"
fi

if [ "$blocking" -gt 0 ]; then
  echo "FAILED: $blocking blocking violations" >&2
  exit 1
fi
exit 0
