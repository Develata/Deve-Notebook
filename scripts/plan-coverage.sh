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
RUNTIME_REGISTRY="$ROOT/docs/registry/runtime-skeleton-registry.md"
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
    */*_test/*|*/*_tests/*|*/test_*/*) return 0 ;;
    *_test.rs|*_tests.rs|*_test_*.rs|*_test_support.rs|*/tests.rs|*/test_modules.rs) return 0 ;;
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

trim() {
  local value="$1"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

strip_md_code() {
  local value
  value="$(trim "$1")"
  value="${value//\`/}"
  printf '%s' "$value"
}

path_or_glob_exists() {
  local rel="$1"
  [ -n "$rel" ] || return 1
  [ -e "$ROOT/$rel" ] && return 0
  compgen -G "$ROOT/$rel" >/dev/null
}

tracked_rust_files() {
  git -C "$ROOT" ls-files -- 'crates' 'apps' |
    grep -E '\.rs$' |
    sed "s|^|$ROOT/|"
}

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
while read -r lines f; do
  [ -n "${f:-}" ] || continue
  [ "$f" = "total" ] && continue
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
done < <(tracked_rust_files | sort | tr '\n' '\0' | xargs -0 wc -l)
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
done < <(tracked_rust_files | sort)

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
# Check 5 — Feature operation / acceptance path drift
# ---------------------------------------------------------------------------
log "== Check 5: feature operation path drift =="
path_status=0
path_report="$(bash "$ROOT/scripts/check-feature-operation-paths.sh" 2>&1)" || path_status=$?
while IFS= read -r line; do
  [ -z "$line" ] && continue
  if [[ "$line" == feature-operation-path-check:\ missing* ]]; then
    err "$line"
  else
    log "$line"
  fi
done <<< "$path_report"
if [ "$path_status" -ne 0 ]; then
  blocking=$((blocking + 1))
fi
log ""

# ---------------------------------------------------------------------------
# Check 6 — Runtime registry current module paths
# ---------------------------------------------------------------------------
log "== Check 6: runtime registry path drift =="
registry_path_warnings=0
registry_status_errors=0
if [ -f "$RUNTIME_REGISTRY" ]; then
  while IFS='|' read -r _ runtime_cell status_cell path_cell _tracking_cell _boundary_cell _rest; do
    runtime="$(strip_md_code "$runtime_cell")"
    status="$(strip_md_code "$status_cell")"
    paths="$(trim "$path_cell")"

    [ -n "$runtime" ] || continue
    [ "$runtime" = "Runtime" ] && continue
    [[ "$runtime" == ---* ]] && continue
    [[ "$runtime" == \`* ]] && runtime="$(strip_md_code "$runtime")"

    case "$status" in
      已收敛|部分承载|未启动|抽象分层) ;;
      *)
        err "runtime-registry-status-invalid: $runtime -> $status"
        registry_status_errors=$((registry_status_errors + 1))
        blocking=$((blocking + 1))
        continue
        ;;
    esac

    while IFS= read -r raw_path; do
      path="$(strip_md_code "$raw_path")"
      [ -n "$path" ] || continue
      if [ "$path" = "未启动" ]; then
        if [ "$status" != "未启动" ]; then
          log "runtime-registry-path-missing(soft): $runtime status=$status path=未启动"
          registry_path_warnings=$((registry_path_warnings + 1))
          soft_warnings=$((soft_warnings + 1))
        fi
        continue
      fi
      if ! path_or_glob_exists "$path"; then
        log "runtime-registry-path-missing(soft): $runtime -> $path"
        registry_path_warnings=$((registry_path_warnings + 1))
        soft_warnings=$((soft_warnings + 1))
      fi
    done < <(printf '%s\n' "$paths" | tr ';' '\n')
  done < <(
    awk '
      /^## Runtime Registry/ { in_registry = 1; next }
      /^## Notes/ { in_registry = 0 }
      in_registry && /^\|/ { print }
    ' "$RUNTIME_REGISTRY" | grep -v '^|---'
  )
else
  err "runtime registry not found: ${RUNTIME_REGISTRY#$ROOT/}"
  registry_status_errors=$((registry_status_errors + 1))
  blocking=$((blocking + 1))
fi
log "runtime registry path warnings (soft): $registry_path_warnings"
log "runtime registry status errors (blocking): $registry_status_errors"
log ""

# ---------------------------------------------------------------------------
# Check 7 — Plan AGENTS anchor registry drift
# ---------------------------------------------------------------------------
log "== Check 7: plan anchor registry drift =="
agents_anchor_dangling=0
agents_anchor_unused=0
agents_anchor_missing=0
declare -A agents_anchor_map=()

while IFS= read -r ref; do
  [ -n "$ref" ] || continue
  agents_anchor_map["$ref"]=1
  chapter="${ref%%#*}"
  anchor="${ref#*#}"
  chapter_file="$PLAN_DIR/$chapter.md"
  if [ ! -f "$chapter_file" ]; then
    err "agents-anchor-dangling: $ref chapter not found"
    agents_anchor_dangling=$((agents_anchor_dangling + 1))
    blocking=$((blocking + 1))
  elif ! grep -Fq "{#$anchor}" "$chapter_file"; then
    err "agents-anchor-dangling: $ref anchor not found"
    agents_anchor_dangling=$((agents_anchor_dangling + 1))
    blocking=$((blocking + 1))
  fi
done < <(grep -oE '`[0-9][0-9]_[A-Za-z0-9_]+#[A-Za-z0-9_-]+`' "$PLAN_DIR/AGENTS.md" | tr -d '`' | sort -u)

for ref in "${!agents_anchor_map[@]}"; do
  if [ -z "${plan_coverage_map[$ref]+x}" ]; then
    log "agents-anchor-unused(soft): $ref"
    agents_anchor_unused=$((agents_anchor_unused + 1))
    soft_warnings=$((soft_warnings + 1))
  fi
done

for ref in "${!plan_coverage_map[@]}"; do
  if [ -z "${agents_anchor_map[$ref]+x}" ]; then
    log "agents-anchor-missing(soft): $ref"
    agents_anchor_missing=$((agents_anchor_missing + 1))
    soft_warnings=$((soft_warnings + 1))
  fi
done

log "agents anchor dangling (blocking): $agents_anchor_dangling"
log "agents anchor unused (soft): $agents_anchor_unused"
log "agents anchor missing from registry (soft): $agents_anchor_missing"
log ""

# ---------------------------------------------------------------------------
# Check 8 — Plan metadata Primary Code Areas path drift
# ---------------------------------------------------------------------------
log "== Check 8: primary code areas path drift =="
primary_area_warnings=0
while IFS=: read -r plan_file line_no line_text; do
  while IFS= read -r area; do
    area="${area#\`}"
    area="${area%\`}"
    [ "$area" = "Primary Code Areas" ] && continue
    if ! path_or_glob_exists "$area"; then
      rel_plan="${plan_file#$ROOT/}"
      log "primary-code-area-missing(soft): $rel_plan:$line_no -> $area"
      primary_area_warnings=$((primary_area_warnings + 1))
      soft_warnings=$((soft_warnings + 1))
    fi
  done < <(printf '%s\n' "$line_text" | grep -oE '`[^`]+`' || true)
done < <(grep -Rsn --include='*.md' 'Primary Code Areas' "$PLAN_DIR")
log "primary code area warnings (soft): $primary_area_warnings"
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
