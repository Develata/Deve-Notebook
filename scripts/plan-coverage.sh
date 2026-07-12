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
GIT_BIN="${GIT_BIN:-git}"
GIT_ROOT="$ROOT"
if [[ "${GIT_BIN##*/}" == "git.exe" ]] && command -v wslpath >/dev/null 2>&1; then
  GIT_ROOT="$(wslpath -w "$ROOT")"
fi
PLAN_DIR="$ROOT/docs/plan"
RUNTIME_REGISTRY="$ROOT/docs/registry/runtime-skeleton-registry.md"
CODE_DIRS=("$ROOT/crates" "$ROOT/apps")
FUSE_LINES=500
SOFT_LINES=250
# Canonical plan_ref / AGENTS anchor core pattern (single source of truth).
# chapter-path is either a single basename (`04_repository`) or a one-level
# multi-file chapter path (`03_storage/authority`), followed by `#<anchor>`.
PLAN_REF_CORE='[0-9][0-9]_[A-Za-z0-9_]+(/[A-Za-z0-9_]+)?#[A-Za-z0-9_-]+'
ALLOWLIST="$ROOT/scripts/plan-coverage-allowlist.txt"
I18N_ALLOWLIST="$ROOT/scripts/i18n-coverage-allowlist.txt"
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
REPORT_LINES=()
WRITE_REPORT=0
LIST_MISSING_PLAN_REF=0
SUMMARY_MISSING_PLAN_REF=0
MISSING_PLAN_REF_SUMMARY_TOP=20

# B0.5 Anchor Contract Upgrade — subcommand modes.
# MODE: full (default) | rewrite | metadata-check | perf-budget-check | no-adr-check | md-links-check
MODE="full"
PERF_BUDGET_FILE=""
MD_LINKS_DIRS=()
# B4.2 — when 1, run the full report then enforce reverse coverage (every stable,
# non-skip registry anchor MUST have >=1 code-side plan_ref).
REVERSE_ENFORCE=0
REWRITE_FROM=""
REWRITE_TO=""
REWRITE_APPLY=0

usage() {
  cat <<'EOF'
usage: plan-coverage.sh [options]
  (no option)                     run full coverage report
  --write-report                  also write scripts/plan-coverage.txt CI artifact
  --list-missing-plan-ref         list non-exempt modules missing plan_ref
  --summary-missing-plan-ref      grouped missing plan_ref counts
  --rewrite-plan-ref --from P --to Q [--apply]
                                  rewrite plan_ref chapter-path prefixes
                                  (dry-run unless --apply)
  --check-metadata-completeness   verify each plan chapter Metadata declares Version + Last Review
  --check-perf-budget             verify 21_perf_budget.md budget table carries numeric P50/P99 (no TBD)
  --check-no-adr-plan-ref         fail if any code plan_ref targets an ADR (adr/...)
  --check-reverse-coverage        run full report, then fail if any stable anchor lacks a code plan_ref
  --check-md-links [dirs...]      verify relative markdown links + #anchors resolve in doc trees
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --write-report) WRITE_REPORT=1 ;;
    --list-missing-plan-ref) LIST_MISSING_PLAN_REF=1 ;;
    --summary-missing-plan-ref) SUMMARY_MISSING_PLAN_REF=1 ;;
    --rewrite-plan-ref) MODE="rewrite" ;;
    --from) REWRITE_FROM="${2:-}"; shift || true ;;
    --to) REWRITE_TO="${2:-}"; shift || true ;;
    --apply) REWRITE_APPLY=1 ;;
    --check-metadata-completeness) MODE="metadata-check" ;;
    --check-perf-budget) MODE="perf-budget-check" ;;
    --check-reverse-coverage) REVERSE_ENFORCE=1 ;;
    --check-no-adr-plan-ref) MODE="no-adr-check" ;;
    --check-md-links) MODE="md-links-check" ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      # --check-md-links takes trailing positional dirs; --check-perf-budget an optional fixture path.
      if [ "$MODE" = "md-links-check" ]; then
        MD_LINKS_DIRS+=("$1")
      elif [ "$MODE" = "perf-budget-check" ]; then
        PERF_BUDGET_FILE="$1"  # optional fixture path override (used by selftest negatives)
      else
        echo "ERROR: unknown argument: $1" >&2
        usage >&2
        exit 2
      fi ;;
  esac
  shift || true
done

append_report() {
  [ "$WRITE_REPORT" = "1" ] || return 0
  REPORT_LINES+=("$1")
}

log() {
  echo "$@"
  append_report "$*"
}

err() {
  echo "ERROR: $*" >&2
  append_report "ERROR: $*"
}

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

git_in_repo() {
  "$GIT_BIN" -c safe.directory="$GIT_ROOT" -C "$GIT_ROOT" "$@"
}

ensure_git_repo_readable() {
  if ! git_in_repo ls-files --error-unmatch Cargo.toml >/dev/null 2>&1; then
    echo "ERROR: plan-coverage: git repository is not readable at $GIT_ROOT" >&2
    echo "ERROR: plan-coverage: refusing to continue because coverage would be incomplete" >&2
    exit 2
  fi
}

tracked_rust_files() {
  git_in_repo ls-files -- 'crates' 'apps' |
    grep -E '\.rs$' |
    sed "s|^|$ROOT/|"
}

tracked_rust_line_counts() {
  git_in_repo grep -n -I -e '^' -- 'crates' 'apps' |
    awk -F: -v root="$ROOT" '
      $1 ~ /\.rs$/ { counts[$1] = $2 }
      END {
        for (f in counts) print counts[f], root "/" f
      }
    ' |
    sort -k2,2
}

tracked_plan_ref_entries() {
  git_in_repo grep -n -I -e '^//\! plan_ref:' -e '^//\! *- ' -- 'crates' 'apps' |
    awk -F: -v root="$ROOT" '
      $1 !~ /\.rs$/ { next }
      {
        file = $1
        line_no = $2 + 0
        line = $0
        sub(/^[^:]*:[0-9]+:/, "", line)
        full = root "/" file
      }
      line ~ /^\/\/! plan_ref:/ {
        active[file] = 1
        last[file] = line_no
        print full "\tHEADER\t" line
        if (line ~ /^\/\/![[:space:]]*plan_ref:[[:space:]]*[^[:space:]]/ &&
            line !~ /^\/\/![[:space:]]*plan_ref:[[:space:]]*infra[[:space:]]*$/) {
          print full "\tHEADER_INVALID\t" line
        }
        next
      }
      active[file] && line_no == last[file] + 1 && line ~ /^\/\/![[:space:]]*-[[:space:]]*/ {
        last[file] = line_no
        sub(/^\/\/![[:space:]]*-[[:space:]]*/, "", line)
        split(line, parts, /[[:space:]]/)
        print full "\tREF\t" parts[1]
        next
      }
      {
        active[file] = 0
      }
    '
}

tracked_i18n_cjk_hits() {
  (git_in_repo grep -n -I -P -e '"[^"]*[\x{4e00}-\x{9fff}][^"]*"' -- \
    'apps/web/src/components' 2>/dev/null || true) |
    awk -F: -v root="$ROOT" '$1 ~ /\.rs$/ { print root "/" $0 }'
  git_in_repo ls-files --others --exclude-standard -- 'apps/web/src/components' |
    while IFS= read -r rel; do
      case "$rel" in
        *.rs) printf '%s\0' "$ROOT/$rel" ;;
      esac
    done |
    xargs -0 -r grep -HnP '"[^"]*[\x{4e00}-\x{9fff}][^"]*"' 2>/dev/null || true
}

tracked_i18n_exact_hits() {
  local grep_args=(-n -I -F)
  local literal
  for literal in "${I18N_FORBIDDEN_ENGLISH_LITERALS[@]}"; do
    grep_args+=(-e "$literal")
  done
  (git_in_repo grep "${grep_args[@]}" -- \
    'apps/web/src/components' 'apps/web/src/editor' 2>/dev/null || true) |
    awk -F: -v root="$ROOT" '$1 ~ /\.rs$/ { print root "/" $0 }'
  git_in_repo ls-files --others --exclude-standard -- 'apps/web/src/components' 'apps/web/src/editor' |
    while IFS= read -r rel; do
      case "$rel" in
        *.rs) printf '%s\0' "$ROOT/$rel" ;;
      esac
    done |
    xargs -0 -r grep -HnF "${grep_args[@]:3}" 2>/dev/null || true
}

REWRITE_TMP_FILES=()

cleanup_rewrite_tmp_files() {
  [ "${#REWRITE_TMP_FILES[@]}" -eq 0 ] && return 0
  rm -f "${REWRITE_TMP_FILES[@]}" 2>/dev/null || true
}

plan_ref_rewrite_candidate_files() {
  local grep_output
  local grep_error
  local grep_status
  grep_error="$(mktemp)" || return 2
  REWRITE_TMP_FILES+=("$grep_error")
  set +e
  grep_output="$(git_in_repo grep -l -F -- "$REWRITE_FROM" -- 'crates' 'apps' 2>"$grep_error")"
  grep_status=$?
  set -e
  if [ "$grep_status" -eq 1 ] && [ ! -s "$grep_error" ]; then
    return 0
  fi
  if [ "$grep_status" -ne 0 ]; then
    printf 'ERROR: rewrite-plan-ref candidate scan failed:\n%s\n' "$grep_output" >&2
    cat "$grep_error" >&2
    return "$grep_status"
  fi
  if [ -s "$grep_error" ]; then
    cat "$grep_error" >&2
  fi
  printf '%s\n' "$grep_output" |
    awk -v root="$ROOT" '{ sub(/\r$/, "", $0); if ($0 ~ /\.rs$/) print root "/" $0 }'
}

# resolve_plan_anchor <chapter_ref> -> echoes the chapter markdown file path.
# Supports single-file chapters (`04_repository`) and multi-file chapter paths
# (`03_storage/authority`). The chapter_ref is the part before the `#` anchor.
resolve_plan_anchor() {
  local chapter_ref="$1"
  printf '%s/%s.md' "$PLAN_DIR" "$chapter_ref"
}

declare -A PLAN_CHAPTER_EXISTS_CACHE=()
declare -A PLAN_ANCHOR_EXISTS_CACHE=()

plan_chapter_exists() {
  local chapter="$1"
  local cached="${PLAN_CHAPTER_EXISTS_CACHE[$chapter]+x}"
  if [ -z "$cached" ]; then
    local chapter_file
    chapter_file="$(resolve_plan_anchor "$chapter")"
    if [ -f "$chapter_file" ]; then
      PLAN_CHAPTER_EXISTS_CACHE["$chapter"]=1
    else
      PLAN_CHAPTER_EXISTS_CACHE["$chapter"]=0
    fi
  fi
  [ "${PLAN_CHAPTER_EXISTS_CACHE[$chapter]}" = "1" ]
}

plan_anchor_exists() {
  local chapter="$1"
  local anchor="$2"
  local key="$chapter#$anchor"
  local cached="${PLAN_ANCHOR_EXISTS_CACHE[$key]+x}"
  if [ -z "$cached" ]; then
    local chapter_file
    chapter_file="$(resolve_plan_anchor "$chapter")"
    if [ -f "$chapter_file" ] && grep -Fq "{#$anchor}" "$chapter_file"; then
      PLAN_ANCHOR_EXISTS_CACHE["$key"]=1
    else
      PLAN_ANCHOR_EXISTS_CACHE["$key"]=0
    fi
  fi
  [ "${PLAN_ANCHOR_EXISTS_CACHE[$key]}" = "1" ]
}

# extract_plan_ref_blocks <file> -> echoes each `//!   - <ref>` list line that
# lives inside a `//! plan_ref:` block. It remains the rewrite-mode extractor;
# full-report scans use tracked_plan_ref_entries for the same contiguous-block
# semantics without per-file shelling.
extract_plan_ref_blocks() {
  local f="$1"
  awk '/^\/\/! plan_ref:/{flag=1;next} flag && /^\/\/! *- /{print; next} flag {flag=0}' "$f"
}

# resolve_agents_anchor_ref <ref> -> echoes "chapter_ref<TAB>anchor".
# Only splits the AGENTS registry cell string; it performs no file/anchor
# existence check. Callers that need validation must call resolve_plan_anchor
# on the returned chapter_ref to avoid a second resolution implementation.
# Skip status (planned/no-code-yet | no-rust-plan-ref) is derived separately
# from the registry row marker by Check 7's planned_anchor_map.
resolve_agents_anchor_ref() {
  local ref="$1"
  local chapter="${ref%%#*}"
  local anchor="${ref#*#}"
  printf '%s\t%s\n' "$chapter" "$anchor"
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

# ---------------------------------------------------------------------------
# B0.5 — rewrite plan_ref chapter-path prefixes across tracked Rust modules.
# Only list items inside `//! plan_ref:` blocks are touched; the //! prefix,
# indentation, list marker, ref-token boundary and trailing comments are
# preserved. Default is dry-run; --apply performs an atomic same-dir replace.
# ---------------------------------------------------------------------------
run_rewrite_plan_ref() {
  if [ -z "$REWRITE_FROM" ] || [ -z "$REWRITE_TO" ]; then
    echo "ERROR: --rewrite-plan-ref requires --from <prefix> --to <prefix>" >&2
    return 2
  fi
  local mode_label="dry-run"
  [ "$REWRITE_APPLY" = "1" ] && mode_label="apply"
  echo "== rewrite-plan-ref ($mode_label): '$REWRITE_FROM' -> '$REWRITE_TO' =="
  REWRITE_TMP_FILES=()
  trap 'status=$?; cleanup_rewrite_tmp_files; exit "$status"' EXIT
  trap 'cleanup_rewrite_tmp_files; exit 141' PIPE
  local total_files=0 total_lines=0
  local candidates
  candidates="$(plan_ref_rewrite_candidate_files)" || return 2
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    if ! grep -q '^//! plan_ref:' "$f" 2>/dev/null; then
      continue
    fi
    local tmp
    if [ "$REWRITE_APPLY" = "1" ]; then
      tmp="$(mktemp "${f}.rewrite.XXXXXX")" || return 2
    else
      tmp="$(mktemp)" || return 2
    fi
    REWRITE_TMP_FILES+=("$tmp")
    awk -v from="$REWRITE_FROM" -v to="$REWRITE_TO" '
      /^\/\/! plan_ref:/ { inblock = 1; print; next }
      inblock && /^\/\/![ \t]*-[ \t]*/ {
        match($0, /^\/\/![ \t]*-[ \t]*/)
        pfx = substr($0, 1, RLENGTH)
        rest = substr($0, RLENGTH + 1)
        split(rest, parts, /[ \t]/)
        token = parts[1]
        after = substr(rest, length(token) + 1)
        flen = length(from)
        if (substr(token, 1, flen) == from) {
          token = to substr(token, flen + 1)
        }
        print pfx token after
        next
      }
      inblock { inblock = 0 }
      { print }
    ' "$f" > "$tmp" || return 2
    if cmp -s "$f" "$tmp"; then
      rm -f "$tmp" || return 2
      continue
    fi
    local hits rel
    hits=$(diff "$f" "$tmp" | grep -c '^>' || true)
    rel="${f#$ROOT/}"
    total_files=$((total_files + 1))
    total_lines=$((total_lines + hits))
    echo "  $rel ($hits line(s))"
    if [ "$REWRITE_APPLY" = "1" ]; then
      mv "$tmp" "$f" || return 2
    else
      diff "$f" "$tmp" | grep -E '^[<>]' | sed 's/^/    /' || true
      rm -f "$tmp" || return 2
    fi
  done < <(printf '%s\n' "$candidates" | sort)
  echo "rewrite-plan-ref: $total_files file(s), $total_lines line(s) [$mode_label]"
  return 0
}

# B0 — metadata completeness check (enforcing).
# Scans every tracked docs/plan markdown carrying a `## Metadata` block and
# verifies the block declares both `Version` and `Last Review`. The Metadata
# block spans from `## Metadata` to the next `## ` heading. Self-contained:
# writes diagnostics to stderr, does not touch the global REPORT buffer.
run_check_metadata_completeness() {
  local missing=0 checked=0 f rel block miss
  while IFS= read -r f; do
    rel="${f#"$ROOT/"}"
    if ! grep -q '^## Metadata' "$f"; then
      # A chapter file (docs/plan/NN_*) MUST carry a Metadata block; a deleted
      # block must fail rather than silently pass. Non-chapter docs (AGENTS.md,
      # master index, plugins/) are exempt.
      case "$rel" in
        docs/plan/[0-9][0-9]_*)
          echo "ERROR: metadata-completeness: $rel is a chapter file but has no '## Metadata' block" >&2
          missing=$((missing + 1)) ;;
      esac
      continue
    fi
    checked=$((checked + 1))
    block="$(awk '/^## Metadata/{flag=1;next} flag && /^## /{flag=0} flag' "$f")"
    miss=""
    printf '%s\n' "$block" | grep -q '^- `Version`:' || miss="${miss} Version"
    printf '%s\n' "$block" | grep -q '^- `Last Review`:' || miss="${miss} Last_Review"
    if [ -n "$miss" ]; then
      echo "ERROR: metadata-completeness: $rel missing fields:${miss}" >&2
      missing=$((missing + 1))
    fi
  done < <(git_in_repo ls-files -- 'docs/plan' | grep -E '\.md$' | sed "s|^|$ROOT/|")
  if [ "$missing" -gt 0 ]; then
    echo "check-metadata-completeness: FAIL — $missing/$checked chapter(s) missing Version/Last Review"
    return 1
  fi
  echo "check-metadata-completeness: OK — $checked chapter(s) carry Version + Last Review"
  return 0
}

# B3.2 — perf budget enforcing. 21_perf_budget.md MUST carry a critical-path
# budget table with numeric P50/P99 latency cells and no TBD/TODO placeholder,
# guaranteeing the budget contract never regresses to an empty shell. Runtime
# regression itself is enforced by CI benchmarks (18_release), not here.
run_check_perf_budget() {
  local f="${PERF_BUDGET_FILE:-$PLAN_DIR/21_perf_budget.md}"
  if [ ! -f "$f" ]; then
    echo "check-perf-budget: FAIL — $f not found" >&2
    return 1
  fi
  # Parse only the §2 Critical Path Budget table (## 2. … next ## ). Each data
  # row's P50/P99 (cols 4/5) MUST be `<n>ms`, or both `—` for a feature-off row;
  # reject placeholders (case-insensitive, incl. 待定/FIXME); require ≥6 numeric
  # rows so the contract cannot regress to a near-empty shell. Runtime regression
  # is enforced by CI benchmarks (18_release), not here.
  local out num off bad ph
  out="$(awk '
    /^## 2\./ { intbl=1; next }
    /^## 3\./ { intbl=0 }
    intbl && /^\|/ {
      line=$0
      if (line ~ /Critical Path/) next             # header row
      if (line ~ /^\|[[:space:]:|-]+$/) next        # separator row
      low=tolower(line)
      if (low ~ /tbd|todo|tba|fixme|待定/) { ph++ }
      split(line, c, "|")
      p50=c[4]; p99=c[5]
      gsub(/^[ \t]+|[ \t]+$/, "", p50); gsub(/^[ \t]+|[ \t]+$/, "", p99)
      if (p50 ~ /^[0-9]+ms$/ && p99 ~ /^[0-9]+ms$/) { num++ }
      else if (p50 == "—" && p99 == "—") { off++ }
      else { bad++ }
    }
    END { printf "%d %d %d %d", num+0, off+0, bad+0, ph+0 }
  ' "$f")"
  read -r num off bad ph <<<"$out"
  if [ "${ph:-0}" -gt 0 ]; then
    echo "check-perf-budget: FAIL — placeholder (TBD/TODO/TBA/FIXME/待定) in §2 budget table" >&2
    return 1
  fi
  if [ "${bad:-0}" -gt 0 ]; then
    echo "check-perf-budget: FAIL — ${bad} row(s) with malformed P50/P99 (need <n>ms, or feature-off —)" >&2
    return 1
  fi
  if [ "${num:-0}" -lt 6 ]; then
    echo "check-perf-budget: FAIL — only ${num:-0} numeric P50/P99 row(s) (<6); §2 budget table incomplete" >&2
    return 1
  fi
  echo "check-perf-budget: OK — ${num} numeric P50/P99 budget row(s) (+${off} feature-off) in §2"
  return 0
}

# B3.4 — emit every heading anchor of a markdown file: explicit `{#id}` plus an
# ASCII-only slug of the heading text. This is deliberately a best-effort slug,
# not a full GitHub slugger: non-ASCII (e.g. CJK) is dropped and duplicate
# headings get no `-N` suffix. Per the AGENTS plan-ref convention every linked
# heading carries an explicit `{#kebab-id}`, so a cross-doc link to a CJK or
# duplicate heading without an explicit id is a convention violation — failing
# it here (anchor-not-found) is the intended enforcement, not a false positive.
md_heading_slugs() {
  awk '
    /^#+[[:space:]]/ {
      line = $0
      while (match(line, /\{#[A-Za-z0-9_-]+\}/)) {
        print substr(line, RSTART + 2, RLENGTH - 3)
        line = substr(line, 1, RSTART - 1) substr(line, RSTART + RLENGTH)
      }
      sub(/^#+[[:space:]]+/, "", line)
      s = tolower(line)
      gsub(/[^a-z0-9 _-]/, "", s)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", s)
      gsub(/ /, "-", s)
      if (s != "") print s
    }' "$1"
}

# B3.4 — relative markdown links in the doc trees must resolve: the target .md
# file must exist (relative to the linking file), and any `#anchor` must match
# an explicit `{#id}` or a heading slug. External/absolute/non-.md links skip.
# Link extraction matches only the bare `](target)` form (target has no space
# or `)`); the title `](t "x")`, angle-bracket `](<t>)`, and literal-space
# forms are intentionally not matched (none occur in the doc trees) and would
# be silently skipped rather than mis-validated — so this never yields a false
# CI failure. `%20`-encoded spaces in the target are decoded and supported.
run_check_md_links() {
  local dirs=("$@")
  [ "${#dirs[@]}" -eq 0 ] && dirs=(docs/plan docs/features docs/acceptance-cases)
  local broken=0 checked=0 f fdir raw target anchor tfile
  local d
  for d in "${dirs[@]}"; do
    while IFS= read -r f; do
      fdir="$(dirname "$f")"
      while IFS= read -r raw; do
        target="${raw%%#*}"; anchor=""
        case "$raw" in *#*) anchor="${raw#*#}";; esac
        case "$target" in http://*|https://*|//*|mailto:*|/*) continue;; esac
        target="${target//%20/ }"
        if [ -z "$target" ]; then tfile="$f"; else tfile="$fdir/$target"; fi
        case "$tfile" in *.md) ;; *) continue;; esac
        checked=$((checked + 1))
        if [ ! -f "$tfile" ]; then
          echo "ERROR: md-link: ${f#"$ROOT/"} -> $target (file not found)" >&2
          broken=$((broken + 1)); continue
        fi
        if [ -n "$anchor" ]; then
          md_heading_slugs "$tfile" | grep -Fxq "$anchor" || {
            echo "ERROR: md-link: ${f#"$ROOT/"} -> $target#$anchor (anchor not found)" >&2
            broken=$((broken + 1))
          }
        fi
      done < <(grep -oE '\]\([^) ]+\)' "$f" | sed -E 's/^\]\(//; s/\)$//')
    done < <({ case "$d" in /*) b="$d" ;; *) b="$ROOT/$d" ;; esac; [ -d "$b" ] && find "$b" -type f -name '*.md' 2>/dev/null | sort; })
  done
  if [ "$broken" -gt 0 ]; then
    echo "check-md-links: FAIL — ${broken} broken link(s) of ${checked} checked"
    return 1
  fi
  echo "check-md-links: OK — ${checked} local markdown link(s) resolve"
  return 0
}

# B4.3 — ADRs are a decision-history slice (time attribute), not blueprint
# clauses; no code plan_ref may target them. Fail if any plan_ref entry
# references `adr/` (a 2-digit chapter prefix is required elsewhere, so `adr/`
# in a plan_ref line is unambiguously an ADR reference).
run_check_no_adr_plan_ref() {
  local hits=0 report
  report="$(
    (git_in_repo grep -n -I -E \
      -e '^//\![[:space:]]*plan_ref:' \
      -e '^//\![[:space:]]*-[[:space:]]*' \
      -- 'crates' 'apps' 2>/dev/null || true) |
      awk -F: -v root="$ROOT" '
        $1 !~ /\.rs$/ { next }
        {
          file = $1
          line_no = $2 + 0
          line = $0
          sub(/^[^:]*:[0-9]+:/, "", line)
          full = root "/" file
        }
        line ~ /^\/\/![[:space:]]*plan_ref:/ {
          active[file] = 1
          last[file] = line_no
          if (line ~ /^\/\/![[:space:]]*plan_ref:.*adr\//) {
            print "ERROR: no-adr-plan-ref: " full " plan_ref header references an ADR: " line
          }
          next
        }
        active[file] && line_no == last[file] + 1 && line ~ /^\/\/![[:space:]]*-[[:space:]]*/ {
          last[file] = line_no
          token = line
          sub(/^\/\/![[:space:]]*-[[:space:]]*/, "", token)
          sub(/[[:space:]].*$/, "", token)
          if (token ~ /(^|[^[:alnum:]_])adr\//) {
            print "ERROR: no-adr-plan-ref: " full " plan_ref targets an ADR: " token
          }
          next
        }
        {
          active[file] = 0
        }
      '
  )"
  if [ -n "$report" ]; then
    printf '%s\n' "$report" >&2
    hits="$(printf '%s\n' "$report" | wc -l | tr -d '[:space:]')"
  fi
  if [ "$hits" -gt 0 ]; then
    echo "check-no-adr-plan-ref: FAIL — ${hits} plan_ref(s) reference an ADR (ADRs are not plan_ref targets)"
    return 1
  fi
  echo "check-no-adr-plan-ref: OK — no plan_ref references an ADR"
  return 0
}

# B0.5 — subcommand dispatch. stub/rewrite modes exit before the full report.
ensure_git_repo_readable

case "$MODE" in
  rewrite)
    rc=0
    run_rewrite_plan_ref || rc=$?
    exit "$rc"
    ;;
  metadata-check)
    rc=0
    run_check_metadata_completeness || rc=$?
    exit "$rc"
    ;;
  no-adr-check)
    rc=0
    run_check_no_adr_plan_ref || rc=$?
    exit "$rc"
    ;;
  md-links-check)
    rc=0
    run_check_md_links "${MD_LINKS_DIRS[@]+"${MD_LINKS_DIRS[@]}"}" || rc=$?
    exit "$rc"
    ;;
  perf-budget-check)
    rc=0
    run_check_perf_budget || rc=$?
    exit "$rc"
    ;;
esac

blocking=0
soft_warnings=0
rust_files=()
declare -A rust_file_lines=()

# ---------------------------------------------------------------------------
# Check 1 — Single-file size fuse (> 500 hard, > 250 soft)
# ---------------------------------------------------------------------------
log "== Check 1: single-file size fuse =="
while read -r lines f; do
  [ -n "${f:-}" ] || continue
  rust_files+=("$f")
  rust_file_lines["$f"]="$lines"
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
done < <(tracked_rust_line_counts)
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
declare -A annotated_file_map=()

while IFS=$'\t' read -r f entry_kind entry_value; do
  [ -n "${f:-}" ] || continue
  rel="${f#$ROOT/}"
  if [ -z "${annotated_file_map[$f]+x}" ]; then
    annotated_file_map["$f"]=1
    annotated_refs=$((annotated_refs + 1))
  fi

  # B3.4 guard: the `//! plan_ref:` header tail MUST be empty or exactly `infra`.
  # Any other inline content (e.g. `//! plan_ref: 04_repository#x`) would bypass
  # entry extraction so dangling / reverse-coverage / no-adr all skip it.
  if [ "$entry_kind" = "HEADER_INVALID" ]; then
    err "invalid-plan-ref-header: $rel — '//! plan_ref:' header tail must be empty or 'infra'"
    blocking=$((blocking + 1))
    continue
  fi

  [ "$entry_kind" = "REF" ] || continue

  # Extract `<chapter-path>#<stable-anchor-id>` refs and verify both parts.
  # chapter-path is either a single-file basename (`04_repository`) or a
  # multi-file chapter path (`03_storage/authority`).
  ref="$entry_value"
  [ -z "$ref" ] && continue
  [ "$ref" = "infra" ] && continue

  plan_ref_anchor_re="^${PLAN_REF_CORE}$"
  if ! [[ "$ref" =~ $plan_ref_anchor_re ]]; then
    err "invalid plan_ref in $rel: $ref"
    dangling_refs=$((dangling_refs + 1))
    blocking=$((blocking + 1))
    continue
  fi

  chapter="${ref%%#*}"
  anchor="${ref#*#}"
  if ! plan_chapter_exists "$chapter"; then
    err "dangling plan_ref in $rel: $chapter.md not found in plan"
    dangling_refs=$((dangling_refs + 1))
    blocking=$((blocking + 1))
  elif ! plan_anchor_exists "$chapter" "$anchor"; then
    err "dangling plan_ref in $rel: anchor $ref not found"
    dangling_refs=$((dangling_refs + 1))
    blocking=$((blocking + 1))
  else
    key="$ref"
    plan_coverage_map["$key"]+="$f "
  fi
done < <(tracked_plan_ref_entries)

for f in "${rust_files[@]}"; do
  [ -z "${annotated_file_map[$f]+x}" ] || continue
  rel="${f#$ROOT/}"
  lines="${rust_file_lines[$f]:-0}"
  [ "$lines" -lt 20 ] && continue
  if is_plan_ref_missing_exempt "$rel"; then
    missing_refs_exempt=$((missing_refs_exempt + 1))
    continue
  fi
  missing_refs=$((missing_refs + 1))
  missing_ref_files+=("$rel")
done

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
  done < <(tracked_i18n_cjk_hits)

  # Exact regression guard for English literals already migrated to t::*.
  # A broad English detector is too noisy for class names and protocol strings;
  # this targeted list blocks the UI copy leaks found by the current gap scan.
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
  done < <(tracked_i18n_exact_hits)
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
agents_anchor_planned=0
declare -A agents_anchor_map=()
declare -A dangling_anchor_map=()
declare -A planned_anchor_map=()
# B4.1 — registry rows trailing `planned/no-code-yet` (governance 先登记后落地)
# or `no-rust-plan-ref` (shell/non-Rust contract, e.g. the perf-budget fuse)
# carry no Rust plan_ref by design; the unused check and reverse-coverage skip
# them. Only the row's first column (the anchor) is taken, so a backticked
# defer ref in the description column is never mis-classified as planned.
while IFS= read -r pref; do
  [ -n "$pref" ] || continue
  planned_anchor_map["$pref"]=1
done < <(grep -E "^\| \`${PLAN_REF_CORE}\`.*(planned/no-code-yet|no-rust-plan-ref)" "$PLAN_DIR/AGENTS.md" | sed -E "s@^\| \`(${PLAN_REF_CORE})\`.*@\1@" | sort -u)

while IFS= read -r ref; do
  [ -n "$ref" ] || continue
  agents_anchor_map["$ref"]=1
  IFS=$'\t' read -r chapter anchor < <(resolve_agents_anchor_ref "$ref") || true
  if ! plan_chapter_exists "$chapter"; then
    err "agents-anchor-dangling: $ref chapter not found"
    agents_anchor_dangling=$((agents_anchor_dangling + 1))
    blocking=$((blocking + 1))
    dangling_anchor_map["$ref"]=1
  elif ! plan_anchor_exists "$chapter" "$anchor"; then
    err "agents-anchor-dangling: $ref anchor not found"
    agents_anchor_dangling=$((agents_anchor_dangling + 1))
    blocking=$((blocking + 1))
    dangling_anchor_map["$ref"]=1
  fi
done < <(grep -oE "\`${PLAN_REF_CORE}\`" "$PLAN_DIR/AGENTS.md" | tr -d '`' | sort -u)

for ref in "${!agents_anchor_map[@]}"; do
  [ -n "${plan_coverage_map[$ref]+x}" ] && continue
  [ -n "${dangling_anchor_map[$ref]+x}" ] && continue  # dangling: reported via blocking only
  if [ -n "${planned_anchor_map[$ref]+x}" ]; then
    agents_anchor_planned=$((agents_anchor_planned + 1))
    continue
  fi
  log "agents-anchor-unused(soft): $ref"
  agents_anchor_unused=$((agents_anchor_unused + 1))
  soft_warnings=$((soft_warnings + 1))
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
log "agents anchor no-rust-plan-ref skip (info): $agents_anchor_planned"
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
  printf '%s\n' "${REPORT_LINES[@]}" > "$ROOT/scripts/plan-coverage.txt"
  echo "report written to scripts/plan-coverage.txt"
fi

if [ "$REVERSE_ENFORCE" = "1" ]; then
  if [ "$blocking" -gt 0 ] || [ "$agents_anchor_unused" -gt 0 ]; then
    echo "check-reverse-coverage: FAIL — ${agents_anchor_unused} stable anchor(s) without code plan_ref; ${blocking} blocking" >&2
    exit 1
  fi
  echo "check-reverse-coverage: OK — every stable anchor has a code plan_ref (${agents_anchor_planned} skipped: planned/no-code-yet|no-rust-plan-ref)"
  exit 0
fi

if [ "$blocking" -gt 0 ]; then
  echo "FAILED: $blocking blocking violations" >&2
  exit 1
fi
exit 0
