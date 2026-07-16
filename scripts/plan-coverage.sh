#!/usr/bin/env bash
# plan-coverage.sh — Plan-Code Bijection Enforcement
#
# Implements Layer 2 (CI Coverage Check) and minimum automated checks
# defined in `docs/plan/AGENTS.md §Plan-Code Bijection Enforcement`.
#
# Exit codes:
#   0 — all checks passed
#   1 — blocking violations found (size fuse, missing/dangling plan_ref, i18n leak)
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
CODE_ROOTS=(apps crates tools)
FUSE_LINES=500
SOFT_LINES=250
# Canonical plan_ref / AGENTS anchor core pattern (single source of truth).
# chapter-path is either a single basename (`04_repository`) or a one-level
# multi-file chapter path (`03_storage/authority`), followed by `#<anchor>`.
PLAN_REF_CORE='[0-9][0-9]_[A-Za-z0-9_]+(/[A-Za-z0-9_]+)?#[A-Za-z0-9_-]+'
ALLOWLIST="$ROOT/scripts/plan-coverage-allowlist.txt"
PLAN_REF_EXEMPTIONS="$ROOT/scripts/plan-ref-exemptions.tsv"
PLAN_ANCHOR_REGISTRY="$PLAN_DIR/AGENTS.md"
PLAN_ANCHOR_REGISTRY_START='<!-- stable-plan-anchor-registry:start -->'
PLAN_ANCHOR_REGISTRY_END='<!-- stable-plan-anchor-registry:end -->'
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
  grep -Fxq "$rel" <(grep -v '^[[:space:]]*#' "$ALLOWLIST" | grep -v '^[[:space:]]*$')
}

is_i18n_allowlisted() {
  local rel_hit="$1"
  [ -f "$I18N_ALLOWLIST" ] || return 1
  grep -Fxq "$rel_hit" <(grep -v '^[[:space:]]*#' "$I18N_ALLOWLIST" | grep -v '^[[:space:]]*$')
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

RUST_SOURCE_RELS=()
RUST_SOURCE_FILES=()
declare -A RUST_SOURCE_REL_MAP=()
TRACKED_RUST_SOURCE_FILES=()
declare -A TRACKED_RUST_SOURCE_REL_MAP=()
RUST_BATCH_SIZE=100

load_rust_source_files() {
  local list_file tracked_list_file rel
  list_file="$(mktemp)" || return 2
  if ! git_in_repo ls-files -z --cached --others --exclude-standard -- "${CODE_ROOTS[@]}" >"$list_file"; then
    rm -f "$list_file"
    echo "ERROR: plan-coverage: failed to enumerate Rust source files" >&2
    return 2
  fi
  while IFS= read -r -d '' rel; do
    case "$rel" in
      *.rs)
        [ -f "$ROOT/$rel" ] || continue
        if [[ "$rel" == *$'\t'* || "$rel" == *$'\n'* ]]; then
          rm -f "$list_file"
          echo "ERROR: plan-coverage: Rust source path contains a tab/newline: $rel" >&2
          return 2
        fi
        RUST_SOURCE_RELS+=("$rel")
        RUST_SOURCE_FILES+=("$ROOT/$rel")
        RUST_SOURCE_REL_MAP["$rel"]=1
        ;;
    esac
  done <"$list_file"
  rm -f "$list_file"

  tracked_list_file="$(mktemp)" || return 2
  if ! git_in_repo ls-files -z --cached -- "${CODE_ROOTS[@]}" >"$tracked_list_file"; then
    rm -f "$tracked_list_file"
    echo "ERROR: plan-coverage: failed to enumerate tracked Rust source files" >&2
    return 2
  fi
  while IFS= read -r -d '' rel; do
    case "$rel" in
      *.rs)
        [ -f "$ROOT/$rel" ] || continue
        TRACKED_RUST_SOURCE_FILES+=("$ROOT/$rel")
        TRACKED_RUST_SOURCE_REL_MAP["$rel"]=1
        ;;
    esac
  done <"$tracked_list_file"
  rm -f "$tracked_list_file"
}

rust_source_scan() {
  local start
  local -a batch=()
  for ((start = 0; start < ${#RUST_SOURCE_FILES[@]}; start += RUST_BATCH_SIZE)); do
    batch=("${RUST_SOURCE_FILES[@]:start:RUST_BATCH_SIZE}")
    awk '
      FILENAME != current {
        if (current != "") print current "\tLINES\t" count
        current = FILENAME
        count = 0
        active = 0
      }
      {
        count++
        line_no = FNR
        line = $0
        sub(/\r$/, "", line)
      }
      line ~ /^\/\/! plan_ref:/ {
        active = 1
        last = line_no
        print FILENAME "\tHEADER\t" line
        if (line ~ /^\/\/![[:space:]]*plan_ref:[[:space:]]*[^[:space:]]/ &&
            line !~ /^\/\/![[:space:]]*plan_ref:[[:space:]]*infra[[:space:]]*$/) {
          print FILENAME "\tHEADER_INVALID\t" line
        }
        next
      }
      active && line_no == last + 1 && line ~ /^\/\/![[:space:]]*-[[:space:]]*/ {
        last = line_no
        sub(/^\/\/![[:space:]]*-[[:space:]]*/, "", line)
        split(line, parts, /[[:space:]]/)
        print FILENAME "\tREF\t" parts[1]
        next
      }
      { active = 0 }
      END { if (current != "") print current "\tLINES\t" count }
    ' "${batch[@]}"
  done
}

RUST_SCAN_READY=0
RUST_SCAN_OUTPUT=""

prepare_rust_source_scan() {
  [ "$RUST_SCAN_READY" -eq 0 ] || return 0
  RUST_SCAN_OUTPUT="$(rust_source_scan)"
  RUST_SCAN_READY=1
}

all_plan_ref_entries() {
  local f entry_kind entry_value
  prepare_rust_source_scan
  while IFS=$'\t' read -r f entry_kind entry_value; do
    [ "$entry_kind" = "LINES" ] && continue
    printf '%s\t%s\t%s\n' "$f" "$entry_kind" "$entry_value"
  done <<<"$RUST_SCAN_OUTPUT"
}

declare -A PLAN_REF_EXEMPTION_KIND=()
declare -A PLAN_REF_EXEMPTION_OWNER=()
declare -A PLAN_REF_EXEMPTION_REASON=()

is_explicit_test_surface() {
  local rel="$1"
  case "$rel" in
    */tests/*|*/benches/*|*/*_test/*|*/*_tests/*|*/test_*/*|*/*_test_support/*) return 0 ;;
    *_test.rs|*_tests.rs|*_test_*.rs|*_test_support.rs|*/test_support.rs|*/tests.rs|*/test_modules.rs) return 0 ;;
    */channel_test/*|*/switcher_prepare_test/*) return 0 ;;
  esac
  return 1
}

load_plan_ref_exemptions() {
  local raw stripped line_no=0 path kind owner reason owner_is_safe
  local -a fields=()
  if [ ! -f "$PLAN_REF_EXEMPTIONS" ]; then
    err "plan-ref-exemptions: missing registry ${PLAN_REF_EXEMPTIONS#$ROOT/}"
    blocking=$((blocking + 1))
    return 0
  fi

  while IFS= read -r raw || [ -n "$raw" ]; do
    line_no=$((line_no + 1))
    raw="${raw%$'\r'}"
    stripped="${raw#"${raw%%[![:space:]]*}"}"
    stripped="${stripped%"${stripped##*[![:space:]]}"}"
    case "$stripped" in
      ''|'#'*) continue ;;
    esac
    fields=()
    IFS=$'\t' read -r -a fields <<<"$raw"
    if [ "${#fields[@]}" -ne 4 ]; then
      err "plan-ref-exemptions:$line_no: expected 4 tab-separated fields"
      blocking=$((blocking + 1))
      continue
    fi
    path="${fields[0]}"
    kind="${fields[1]}"
    owner="${fields[2]}"
    reason="${fields[3]}"
    reason="${reason#"${reason%%[![:space:]]*}"}"
    reason="${reason%"${reason##*[![:space:]]}"}"

    case "$path" in
      apps/*.rs|crates/*.rs|tools/*.rs) ;;
      *)
        err "plan-ref-exemptions:$line_no: path must be an exact apps/crates/tools Rust path: $path"
        blocking=$((blocking + 1))
        continue
        ;;
    esac
    if [[ "$path" == /* || "$path" == *\\* || "$path" == ../* || "$path" == */../* || "$path" == */.. || "$path" == *$'\t'* ]]; then
      err "plan-ref-exemptions:$line_no: unsafe path: $path"
      blocking=$((blocking + 1))
      continue
    fi
    if [ -n "${PLAN_REF_EXEMPTION_KIND[$path]+x}" ]; then
      err "plan-ref-exemptions:$line_no: duplicate path: $path"
      blocking=$((blocking + 1))
      continue
    fi
    if [ -z "${RUST_SOURCE_REL_MAP[$path]+x}" ]; then
      err "plan-ref-exemptions:$line_no: stale or absent Rust source: $path"
      blocking=$((blocking + 1))
      continue
    fi
    case "$kind" in
      test|generated|local-only-infra) ;;
      *)
        err "plan-ref-exemptions:$line_no: invalid kind '$kind' for $path"
        blocking=$((blocking + 1))
        continue
        ;;
    esac
    if [ "$kind" = "test" ] && ! is_explicit_test_surface "$path"; then
      err "plan-ref-exemptions:$line_no: test entry is outside an explicit test/bench surface: $path"
      blocking=$((blocking + 1))
      continue
    fi
    if [ -z "$reason" ] || [ "$reason" = "$kind" ]; then
      err "plan-ref-exemptions:$line_no: non-generic reason required for $path"
      blocking=$((blocking + 1))
      continue
    fi
    if [ "$kind" = "generated" ]; then
      owner_is_safe=1
      case "$owner" in
        ''|'-'|/*|*\\*|../*|*/../*|*/..|*'*'*|*'?'*|*'['*|*$'\t'*|*$'\n'*) owner_is_safe=0 ;;
      esac
      if [ "$owner_is_safe" -ne 1 ] || \
         [ ! -f "$ROOT/$owner" ] || [ -L "$ROOT/$owner" ] || \
         [ "$(git_in_repo ls-files --error-unmatch -- ":(literal)$owner" 2>/dev/null || true)" != "$owner" ]; then
        err "plan-ref-exemptions:$line_no: generated owner must be one exact present tracked producer file for $path: $owner"
        blocking=$((blocking + 1))
        continue
      fi
      if ! head -n 20 "$ROOT/$path" | grep -Eqi '(generated|do not edit)'; then
        err "plan-ref-exemptions:$line_no: generated source lacks provenance marker: $path"
        blocking=$((blocking + 1))
        continue
      fi
    elif [ "$owner" != "-" ]; then
      err "plan-ref-exemptions:$line_no: owner must be '-' for $kind entry $path"
      blocking=$((blocking + 1))
      continue
    fi

    PLAN_REF_EXEMPTION_KIND["$path"]="$kind"
    PLAN_REF_EXEMPTION_OWNER["$path"]="$owner"
    PLAN_REF_EXEMPTION_REASON["$path"]="$reason"
  done <"$PLAN_REF_EXEMPTIONS"
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
  local output rc rel
  set +e
  output="$(git_in_repo grep -l -F -- "$REWRITE_FROM" -- "${CODE_ROOTS[@]}" 2>/dev/null)"
  rc=$?
  set -e
  if [ "$rc" -ne 0 ] && [ "$rc" -ne 1 ]; then
    echo "ERROR: rewrite-plan-ref candidate scan failed" >&2
    return 2
  fi
  [ "$rc" -eq 0 ] || return 0
  while IFS= read -r rel; do
    [ -n "${TRACKED_RUST_SOURCE_REL_MAP[$rel]+x}" ] || continue
    printf '%s\n' "$ROOT/$rel"
  done <<<"$output"
}

declare -A PLAN_CHAPTER_EXISTS_CACHE=()
declare -A PLAN_ANCHOR_INDEX=()
PLAN_ANCHOR_INDEX_LOADED=0

load_plan_anchor_index() {
  [ "$PLAN_ANCHOR_INDEX_LOADED" -eq 0 ] || return 0
  local chapter anchor nullglob_was_set=0
  shopt -q nullglob && nullglob_was_set=1
  shopt -s nullglob
  local -a plan_files=("$PLAN_DIR"/*.md "$PLAN_DIR"/*/*.md)
  [ "$nullglob_was_set" -eq 1 ] || shopt -u nullglob
  while IFS=$'\t' read -r chapter anchor; do
    [ -n "$chapter" ] && [ -n "$anchor" ] || continue
    PLAN_ANCHOR_INDEX["$chapter#$anchor"]=1
  done < <(
    awk -v plan_dir="$PLAN_DIR" '
      {
        rest = $0
        while (match(rest, /\{#[A-Za-z0-9_-]+\}/)) {
          anchor = substr(rest, RSTART + 2, RLENGTH - 3)
          chapter = substr(FILENAME, length(plan_dir) + 2)
          sub(/\.md$/, "", chapter)
          print chapter "\t" anchor
          rest = substr(rest, RSTART + RLENGTH)
        }
      }
    ' "${plan_files[@]}"
  )
  PLAN_ANCHOR_INDEX_LOADED=1
}

plan_chapter_exists() {
  local chapter="$1"
  local cached="${PLAN_CHAPTER_EXISTS_CACHE[$chapter]+x}"
  if [ -z "$cached" ]; then
    local chapter_file="$PLAN_DIR/$chapter.md"
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
  load_plan_anchor_index
  [ -n "${PLAN_ANCHOR_INDEX[$key]+x}" ]
}

# extract_plan_ref_blocks <file> -> echoes each `//!   - <ref>` list line that
# lives inside a `//! plan_ref:` block. It remains the rewrite-mode extractor;
# full-report scans use all_plan_ref_entries for the same contiguous-block
# semantics without per-file shelling.
extract_plan_ref_blocks() {
  local f="$1"
  awk '/^\/\/! plan_ref:/{flag=1;next} flag && /^\/\/! *- /{print; next} flag {flag=0}' "$f"
}

# resolve_agents_anchor_ref <ref> -> echoes "chapter_ref<TAB>anchor".
# Only splits the AGENTS registry cell string; it performs no file/anchor
# existence check. Callers validate the returned parts through the shared
# plan_chapter_exists / plan_anchor_exists index.
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
  local hits=0 report f entry_kind entry_value
  report="$(
    while IFS=$'\t' read -r f entry_kind entry_value; do
      [ -n "${f:-}" ] || continue
      if [ "$entry_kind" = "HEADER" ] && [[ "$entry_value" == *adr/* ]]; then
        printf 'ERROR: no-adr-plan-ref: %s plan_ref header references an ADR: %s\n' "$f" "$entry_value"
      elif [ "$entry_kind" = "REF" ] && [[ "$entry_value" == adr/* || "$entry_value" == */adr/* ]]; then
        printf 'ERROR: no-adr-plan-ref: %s plan_ref targets an ADR: %s\n' "$f" "$entry_value"
      fi
    done < <(all_plan_ref_entries)
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
  full|rewrite|no-adr-check)
    if ! load_rust_source_files; then
      exit 2
    fi
    ;;
esac

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
rust_files=("${RUST_SOURCE_FILES[@]}")
declare -A rust_file_lines=()
for f in "${rust_files[@]}"; do
  rust_file_lines["$f"]=0
done

# ---------------------------------------------------------------------------
# Check 1 — Single-file size fuse (> 500 hard, > 250 soft)
# ---------------------------------------------------------------------------
log "== Check 1: single-file size fuse =="
prepare_rust_source_scan
while IFS=$'\t' read -r f entry_kind entry_value; do
  [ "$entry_kind" = "LINES" ] || continue
  lines="$entry_value"
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
done <<<"$RUST_SCAN_OUTPUT"
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
declare -A header_count_map=()
declare -A invalid_header_map=()
declare -A infra_header_map=()
declare -A real_ref_count_map=()

load_plan_ref_exemptions

while IFS=$'\t' read -r f entry_kind entry_value; do
  [ -n "${f:-}" ] || continue
  rel="${f#$ROOT/}"
  if [ "$entry_kind" = "HEADER" ]; then
    header_count_map["$f"]=$(( ${header_count_map[$f]:-0} + 1 ))
    if [ "$entry_value" = "//! plan_ref: infra" ]; then
      infra_header_map["$f"]=1
    fi
    continue
  fi

  if [ "$entry_kind" = "HEADER_INVALID" ]; then
    err "invalid-plan-ref-header: $rel — '//! plan_ref:' header tail must be empty or 'infra'"
    invalid_header_map["$f"]=1
    blocking=$((blocking + 1))
    continue
  fi

  [ "$entry_kind" = "REF" ] || continue

  # Extract `<chapter-path>#<stable-anchor-id>` refs and verify both parts.
  # chapter-path is either a single-file basename (`04_repository`) or a
  # multi-file chapter path (`03_storage/authority`).
  ref="$entry_value"
  [ -z "$ref" ] && continue
  real_ref_count_map["$f"]=$(( ${real_ref_count_map[$f]:-0} + 1 ))

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
    plan_coverage_map["$key"]+="$rel"$'\n'
  fi
done <<<"$RUST_SCAN_OUTPUT"

for f in "${rust_files[@]}"; do
  rel="${f#$ROOT/}"
  header_count="${header_count_map[$f]:-0}"
  real_ref_count="${real_ref_count_map[$f]:-0}"
  exemption_kind="${PLAN_REF_EXEMPTION_KIND[$rel]:-}"

  if [ "$header_count" -gt 1 ]; then
    err "duplicate-plan-ref-header: $rel — expected exactly one header"
    blocking=$((blocking + 1))
  fi
  if [ -n "${infra_header_map[$f]+x}" ] && [ "$real_ref_count" -gt 0 ]; then
    err "invalid-plan-ref-header: $rel — infra cannot be combined with plan anchors"
    blocking=$((blocking + 1))
  fi
  if [ "$header_count" -gt 0 ] && [ "$real_ref_count" -eq 0 ] && \
     [ -z "${infra_header_map[$f]+x}" ] && [ -z "${invalid_header_map[$f]+x}" ]; then
    err "empty-plan-ref-header: $rel — add at least one stable anchor or use a reasoned exemption"
    blocking=$((blocking + 1))
  fi

  if [ "$real_ref_count" -gt 0 ]; then
    annotated_refs=$((annotated_refs + 1))
    if [ -n "$exemption_kind" ]; then
      err "plan-ref-exemptions: stale exemption on anchored module: $rel"
      blocking=$((blocking + 1))
    fi
    continue
  fi

  if [ -n "${infra_header_map[$f]+x}" ]; then
    if [ "$exemption_kind" = "local-only-infra" ]; then
      missing_refs_exempt=$((missing_refs_exempt + 1))
    else
      err "plan-ref-exemptions: infra header requires exact local-only-infra entry: $rel"
      blocking=$((blocking + 1))
    fi
    continue
  fi

  if [ "$header_count" -gt 0 ]; then
    if [ -n "$exemption_kind" ]; then
      err "plan-ref-exemptions: exemption cannot mask malformed/empty header: $rel"
      blocking=$((blocking + 1))
    fi
    continue
  fi

  if [ "$exemption_kind" = "test" ] || [ "$exemption_kind" = "generated" ]; then
    missing_refs_exempt=$((missing_refs_exempt + 1))
    continue
  fi
  if [ "$exemption_kind" = "local-only-infra" ]; then
    err "plan-ref-exemptions: local-only-infra entry requires exact '//! plan_ref: infra' header: $rel"
    blocking=$((blocking + 1))
    continue
  fi
  missing_refs=$((missing_refs + 1))
  missing_ref_files+=("$rel")
  err "missing-plan-ref: $rel"
  blocking=$((blocking + 1))
done

log "modules with plan_ref: $annotated_refs"
log "modules without plan_ref (blocking): $missing_refs"
log "modules without plan_ref (exempt): $missing_refs_exempt"
log "dangling plan_ref (blocking): $dangling_refs"
if [ "$LIST_MISSING_PLAN_REF" = "1" ]; then
  log "missing plan_ref files (blocking):"
  if [ "${#missing_ref_files[@]}" -eq 0 ]; then
    log "    (none)"
  else
    for rel in "${missing_ref_files[@]}"; do
      log "    $rel"
    done
  fi
fi
if [ "$SUMMARY_MISSING_PLAN_REF" = "1" ]; then
  log "missing plan_ref summary (blocking):"
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
# Check 4 — Acceptance case / flow / journey matrix
# ---------------------------------------------------------------------------
log "== Check 4: acceptance matrix =="
matrix_status=0
matrix_report="$(bash "$ROOT/scripts/check-acceptance-matrix.sh" 2>&1)" || matrix_status=$?
while IFS= read -r line; do
  [ -z "$line" ] && continue
  log "$line"
done <<< "$matrix_report"
if [ "$matrix_status" -ne 0 ]; then
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
agents_anchor_registry_errors=0
declare -A agents_anchor_map=()
declare -A dangling_anchor_map=()
declare -A planned_anchor_map=()

# Only the explicitly delimited three-column registry is authoritative. A ref
# mentioned in prose, examples, or another table MUST NOT silently register an
# anchor. Exact markers and row shape also make duplicate/malformed rows
# machine-detectable.
registry_start_count="$(grep -Fxc "$PLAN_ANCHOR_REGISTRY_START" "$PLAN_ANCHOR_REGISTRY" 2>/dev/null || true)"
registry_end_count="$(grep -Fxc "$PLAN_ANCHOR_REGISTRY_END" "$PLAN_ANCHOR_REGISTRY" 2>/dev/null || true)"
registry_start_count="${registry_start_count:-0}"
registry_end_count="${registry_end_count:-0}"
if [ "$registry_start_count" -ne 1 ] || [ "$registry_end_count" -ne 1 ]; then
  err "agents-anchor-registry: expected exactly one start/end marker in ${PLAN_ANCHOR_REGISTRY#$ROOT/}"
  agents_anchor_registry_errors=$((agents_anchor_registry_errors + 1))
  blocking=$((blocking + 1))
else
  registry_header_count=0
  registry_separator_count=0
  registry_row_count=0
  registry_row_re="^\\| \`(${PLAN_REF_CORE})\` \\| [^|]+ \\| [^|]+ \\|$"
  while IFS= read -r registry_line || [ -n "$registry_line" ]; do
    registry_line="${registry_line%$'\r'}"
    case "$registry_line" in
      '') continue ;;
      '| Anchor | Plan 位置 | 语义 |')
        registry_header_count=$((registry_header_count + 1))
        continue
        ;;
      '|---|---|---|')
        registry_separator_count=$((registry_separator_count + 1))
        continue
        ;;
    esac
    if ! [[ "$registry_line" =~ $registry_row_re ]]; then
      err "agents-anchor-registry: malformed row: $registry_line"
      agents_anchor_registry_errors=$((agents_anchor_registry_errors + 1))
      blocking=$((blocking + 1))
      continue
    fi
    ref="${BASH_REMATCH[1]}"
    if [ -n "${agents_anchor_map[$ref]+x}" ]; then
      err "agents-anchor-registry: duplicate anchor: $ref"
      agents_anchor_registry_errors=$((agents_anchor_registry_errors + 1))
      blocking=$((blocking + 1))
      continue
    fi
    agents_anchor_map["$ref"]=1
    registry_row_count=$((registry_row_count + 1))
    if [[ "$registry_line" == *planned/no-code-yet* || "$registry_line" == *no-rust-plan-ref* ]]; then
      planned_anchor_map["$ref"]=1
    fi
  done < <(
    awk -v start="$PLAN_ANCHOR_REGISTRY_START" -v end="$PLAN_ANCHOR_REGISTRY_END" '
      $0 == start { in_registry = 1; next }
      $0 == end { exit }
      in_registry { print }
    ' "$PLAN_ANCHOR_REGISTRY"
  )
  if [ "$registry_header_count" -ne 1 ] || [ "$registry_separator_count" -ne 1 ] || [ "$registry_row_count" -eq 0 ]; then
    err "agents-anchor-registry: expected one header, one separator, and at least one anchor row"
    agents_anchor_registry_errors=$((agents_anchor_registry_errors + 1))
    blocking=$((blocking + 1))
  fi
fi

for ref in "${!agents_anchor_map[@]}"; do
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
done

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
    err "agents-anchor-missing(blocking): $ref"
    agents_anchor_missing=$((agents_anchor_missing + 1))
    blocking=$((blocking + 1))
  fi
done

log "agents anchor dangling (blocking): $agents_anchor_dangling"
log "agents anchor unused (soft): $agents_anchor_unused"
log "agents anchor no-rust-plan-ref skip (info): $agents_anchor_planned"
log "agents anchor missing from registry (blocking): $agents_anchor_missing"
log "agents anchor registry format errors (blocking): $agents_anchor_registry_errors"
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
  while IFS= read -r rel; do
    [ -n "$rel" ] || continue
    log "    $rel"
  done <<<"${plan_coverage_map[$key]}"
done
log ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
log "== Summary =="
log "blocking violations: $blocking"
log "soft warnings: $soft_warnings"

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
