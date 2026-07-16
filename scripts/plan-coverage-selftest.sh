#!/usr/bin/env bash
# plan-coverage-selftest.sh — plan/code governance scanner unit tests.
#
# Covers the anchor-contract checks plus isolated positive/negative fixtures:
#   (a) the canonical plan_ref pattern accepts both basename and chapter-path
#       anchor forms (and rejects malformed refs);
#   (b) `--rewrite-plan-ref` in dry-run mode does not modify any tracked file;
#   (c+) metadata/perf/ADR/link gates and strict plan_ref exemptions.
#
# The pattern under test is extracted from plan-coverage.sh (single source of
# truth) so this test never drifts from the implementation.
#
# Exit codes: 0 — all assertions passed; 1 — at least one assertion failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
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
rewrite_tmp="$(mktemp -d)"
rewrite_root="$rewrite_tmp/repo"
mkdir -p "$rewrite_root/scripts" "$rewrite_root/apps"
cp "$COVERAGE" "$rewrite_root/scripts/plan-coverage.sh"
chmod +x "$rewrite_root/scripts/plan-coverage.sh"
printf '[workspace]\nresolver = "2"\n' >"$rewrite_root/Cargo.toml"
printf '//! plan_ref:\n//!   - 04_storage#old\n//!\npub const TRACKED: bool = true;\n' >"$rewrite_root/apps/tracked.rs"
git -C "$rewrite_root" init -q
git -C "$rewrite_root" config user.name plan-coverage-selftest
git -C "$rewrite_root" config user.email plan-coverage-selftest@example.invalid
git -C "$rewrite_root" add .
git -C "$rewrite_root" commit -qm fixture
printf '//! plan_ref:\n//!   - 04_storage#old\n//!\npub const UNTRACKED: bool = true;\n' >"$rewrite_root/apps/untracked.rs"

tracked_before="$(sha256sum "$rewrite_root/apps/tracked.rs" | awk '{print $1}')"
untracked_before="$(sha256sum "$rewrite_root/apps/untracked.rs" | awk '{print $1}')"
"$rewrite_root/scripts/plan-coverage.sh" --rewrite-plan-ref --from 04_storage# --to 04_storage/authority# >/dev/null 2>&1
tracked_after="$(sha256sum "$rewrite_root/apps/tracked.rs" | awk '{print $1}')"
untracked_after="$(sha256sum "$rewrite_root/apps/untracked.rs" | awk '{print $1}')"
if [ "$tracked_before" = "$tracked_after" ] && [ "$untracked_before" = "$untracked_after" ]; then
  echo "ok        (dry-run preserved tracked and untracked bytes)"; pass=$((pass + 1))
else
  echo "FAIL: --rewrite-plan-ref dry-run modified fixture bytes"
  fail=$((fail + 1))
fi
residue="$(find "$rewrite_root/apps" -type f -name '*.rewrite.*' -print -quit 2>/dev/null || true)"
if [ -z "$residue" ]; then
  echo "ok        (dry-run left no rewrite temp files)"; pass=$((pass + 1))
else
  echo "FAIL: --rewrite-plan-ref dry-run left temp file: $residue"
  fail=$((fail + 1))
fi

"$rewrite_root/scripts/plan-coverage.sh" --rewrite-plan-ref --from 04_storage# --to 04_storage/authority# --apply >/dev/null 2>&1
if grep -qF '04_storage/authority#old' "$rewrite_root/apps/tracked.rs" && \
   [ "$(sha256sum "$rewrite_root/apps/untracked.rs" | awk '{print $1}')" = "$untracked_before" ]; then
  echo "ok        (apply rewrote tracked source only)"; pass=$((pass + 1))
else
  echo "FAIL: --rewrite-plan-ref --apply touched untracked source or missed tracked source"
  fail=$((fail + 1))
fi

git_fail_tmp="$(mktemp -d)"
cat >"$git_fail_tmp/git-fail" <<'SH'
#!/usr/bin/env bash
for arg in "$@"; do
  if [ "$arg" = "ls-files" ]; then
    for nested in "$@"; do
      if [ "$nested" = "--error-unmatch" ]; then
        echo "Cargo.toml"
        exit 0
      fi
    done
    echo "fake git source enumeration failure" >&2
    exit 2
  fi
done
git "$@"
SH
chmod +x "$git_fail_tmp/git-fail"
if GIT_BIN="$git_fail_tmp/git-fail" "$rewrite_root/scripts/plan-coverage.sh" --rewrite-plan-ref --from 04_storage# --to 04_storage/authority# >/dev/null 2>&1; then
  echo "FAIL: --rewrite-plan-ref candidate scan failure returned success"
  fail=$((fail + 1))
else
  echo "ok       (rejected): rewrite candidate scan failure"
  pass=$((pass + 1))
fi
rm -rf "$git_fail_tmp"
rm -rf "$rewrite_tmp"

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

# ---------------------------------------------------------------------------
# (g) strict plan_ref coverage + explicit exemption fixtures
# ---------------------------------------------------------------------------
echo "== (g) strict plan_ref and exemption fixtures =="
fixture_tmp="$(mktemp -d)"
trap 'rm -rf "$perf_tmp" "$mdl_tmp" "$fixture_tmp"' EXIT
fixture_base="$fixture_tmp/base"
mkdir -p \
  "$fixture_base/scripts" \
  "$fixture_base/docs/plan" \
  "$fixture_base/docs/registry" \
  "$fixture_base/apps" \
  "$fixture_base/crates/tests" \
  "$fixture_base/tools"
cp "$COVERAGE" "$fixture_base/scripts/plan-coverage.sh"
chmod +x "$fixture_base/scripts/plan-coverage.sh"
printf '[workspace]\nresolver = "2"\n' >"$fixture_base/Cargo.toml"
printf '# Fixture Plan\n\n## Contract {#contract}\n' >"$fixture_base/docs/plan/02_positioning.md"
cat >"$fixture_base/docs/plan/AGENTS.md" <<'MD'
# Fixture Plan Rules

<!-- stable-plan-anchor-registry:start -->
| Anchor | Plan 位置 | 语义 |
|---|---|---|
| `02_positioning#contract` | `## Contract` | Fixture contract |
<!-- stable-plan-anchor-registry:end -->
MD
cat >"$fixture_base/docs/registry/runtime-skeleton-registry.md" <<'MD'
## Runtime Registry

| Runtime | Status | Current Module Paths | Tracking | Boundary |
|---|---|---|---|---|
| `fixture_runtime` | `未启动` | 未启动 | fixture | fixture |

## Notes
MD
cat >"$fixture_base/scripts/check-acceptance-matrix.sh" <<'SH'
#!/usr/bin/env bash
echo "fixture-acceptance-matrix: OK"
SH
cat >"$fixture_base/scripts/check-feature-operation-paths.sh" <<'SH'
#!/usr/bin/env bash
echo "fixture-feature-operation-paths: OK"
SH
chmod +x "$fixture_base/scripts/check-acceptance-matrix.sh" "$fixture_base/scripts/check-feature-operation-paths.sh"
for source in \
  "$fixture_base/apps/app.rs" \
  "$fixture_base/crates/lib.rs" \
  "$fixture_base/tools/tool.rs" \
  "$fixture_base/tools/path with space.rs"
do
  printf '//! plan_ref:\n//!   - 02_positioning#contract\n//!\npub const OK: bool = true;\n' >"$source"
done
printf 'pub const TEST_ONLY: bool = true;\n' >"$fixture_base/crates/tests/helper.rs"
printf '// @generated by scripts/generate-fixture.sh; DO NOT EDIT.\npub const GENERATED: bool = true;\n' >"$fixture_base/tools/generated.rs"
printf '//! plan_ref: infra\npub const LOCAL_ONLY: bool = true;\n' >"$fixture_base/tools/local.rs"
printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture_base/scripts/generate-fixture.sh"
chmod +x "$fixture_base/scripts/generate-fixture.sh"
printf '%s\r\n' \
  '# exact fixture exemptions (CRLF is intentional)' \
  $'crates/tests/helper.rs\ttest\t-\tTest-only fixture module; production behavior belongs to the exercised module.' \
  $'tools/generated.rs\tgenerated\tscripts/generate-fixture.sh\tGenerated fixture with an explicit tracked producer and provenance marker.' \
  $'tools/local.rs\tlocal-only-infra\t-\tRepository-local fixture plumbing; it owns no product authority or runtime state.' \
  >"$fixture_base/scripts/plan-ref-exemptions.tsv"
git -C "$fixture_base" init -q
git -C "$fixture_base" config user.name plan-coverage-selftest
git -C "$fixture_base" config user.email plan-coverage-selftest@example.invalid
git -C "$fixture_base" add .
git -C "$fixture_base" commit -qm fixture

run_plan_ref_fixture() {
  GIT_BIN=git "$1/scripts/plan-coverage.sh" --list-missing-plan-ref --check-reverse-coverage 2>&1
}

assert_fixture_pass() { # <fixture-root> <label>
  local out
  if out="$(run_plan_ref_fixture "$1")" && \
     printf '%s\n' "$out" | grep -q '^blocking violations: 0$' && \
     printf '%s\n' "$out" | grep -q '^== Check 8: primary code areas path drift ==$'; then
    echo "ok        (accepted): $2"; pass=$((pass + 1))
  else
    echo "FAIL (want accept): $2"
    printf '%s\n' "$out"
    fail=$((fail + 1))
  fi
}

assert_fixture_diagnostics() { # <fixture-root> <expected-text>...
  local root="$1" out expected
  shift
  if out="$(run_plan_ref_fixture "$root")"; then
    echo "FAIL (want reject): combined negative fixture"
    fail=$((fail + $#))
    return
  fi
  for expected in "$@"; do
    if printf '%s\n' "$out" | grep -Fq -- "$expected"; then
      echo "ok       (rejected): $expected"; pass=$((pass + 1))
    else
      echo "FAIL (missing diagnostic): $expected"
      fail=$((fail + 1))
    fi
  done
}

assert_fixture_pass "$fixture_base" "apps/crates/tools + CRLF exact exemptions + spaced path"

case_root="$fixture_tmp/combined-negative"
git clone -q "$fixture_base" "$case_root"
printf 'pub const UNTRACKED_SHORT: bool = true;' >"$case_root/tools/one.rs"
: >"$case_root/crates/empty.rs"
printf 'pub const HAND_WRITTEN: bool = true;\n' >"$case_root/apps/hand_generated.rs"
printf '//! plan_ref: 02_positioning#contract\npub const INLINE: bool = true;\n' >"$case_root/tools/inline.rs"
{
  printf '//! plan_ref:\n//!   - 02_positioning#contract\n//!\n'
  for i in $(seq 1 497); do printf '// line %s\n' "$i"; done
  printf '// final line without newline'
} >"$case_root/tools/too_large.rs"
printf '//! plan_ref: infra\npub const INFRA: bool = true;\n' >"$case_root/tools/unregistered_infra.rs"
printf 'pub const LOCAL: bool = true;\n' >"$case_root/tools/header_missing.rs"
printf '%s\n' $'tools/header_missing.rs\tlocal-only-infra\t-\tRepository-local fixture helper with no product authority.' >>"$case_root/scripts/plan-ref-exemptions.tsv"
printf '// @generated; DO NOT EDIT.\npub const BAD: bool = true;\n' >"$case_root/tools/bad_generated.rs"
printf '%s\n' $'tools/bad_generated.rs\tgenerated\tscripts/missing-producer.sh\tGenerated fixture whose producer is intentionally absent.' >>"$case_root/scripts/plan-ref-exemptions.tsv"
printf '// @generated; DO NOT EDIT.\npub const BAD_DIR: bool = true;\n' >"$case_root/tools/bad_generated_dir.rs"
printf '%s\n' $'tools/bad_generated_dir.rs\tgenerated\tscripts\tGenerated fixture whose owner is intentionally a directory.' >>"$case_root/scripts/plan-ref-exemptions.tsv"
printf '// @generated; DO NOT EDIT.\npub const BAD_GLOB: bool = true;\n' >"$case_root/tools/bad_generated_glob.rs"
printf '%s\n' $'tools/bad_generated_glob.rs\tgenerated\tscripts/*.sh\tGenerated fixture whose owner is intentionally a glob.' >>"$case_root/scripts/plan-ref-exemptions.tsv"
printf '#!/usr/bin/env sh\nexit 0\n' >"$case_root/scripts/deleted-producer.sh"
git -C "$case_root" add scripts/deleted-producer.sh
rm "$case_root/scripts/deleted-producer.sh"
printf '// @generated; DO NOT EDIT.\npub const BAD_DELETED: bool = true;\n' >"$case_root/tools/bad_generated_deleted.rs"
printf '%s\n' $'tools/bad_generated_deleted.rs\tgenerated\tscripts/deleted-producer.sh\tGenerated fixture whose tracked producer is intentionally absent from the worktree.' >>"$case_root/scripts/plan-ref-exemptions.tsv"
printf '%s\n' $'crates/tests/helper.rs\ttest\t-\tDuplicate fixture entry is intentionally invalid.' >>"$case_root/scripts/plan-ref-exemptions.tsv"
printf 'pub const RUNTIME: bool = true;\n' >"$case_root/apps/runtime.rs"
printf '%s\n' $'apps/runtime.rs\ttest\t-\tThis fixture intentionally misclassifies production code as test-only.' >>"$case_root/scripts/plan-ref-exemptions.tsv"
assert_fixture_diagnostics "$case_root" \
  "missing-plan-ref: tools/one.rs" \
  "missing-plan-ref: crates/empty.rs" \
  "missing-plan-ref: apps/hand_generated.rs" \
  "invalid-plan-ref-header: tools/inline.rs" \
  "FUSE(501): tools/too_large.rs" \
  "infra header requires exact local-only-infra entry: tools/unregistered_infra.rs" \
  "local-only-infra entry requires exact '//! plan_ref: infra' header: tools/header_missing.rs" \
  "generated owner must be one exact present tracked producer file for tools/bad_generated.rs: scripts/missing-producer.sh" \
  "generated owner must be one exact present tracked producer file for tools/bad_generated_dir.rs: scripts" \
  "generated owner must be one exact present tracked producer file for tools/bad_generated_glob.rs: scripts/*.sh" \
  "generated owner must be one exact present tracked producer file for tools/bad_generated_deleted.rs: scripts/deleted-producer.sh" \
  "duplicate path: crates/tests/helper.rs" \
  "test entry is outside an explicit test/bench surface: apps/runtime.rs"

assert_fixture_reject_text() { # <fixture-root> <expected-text> <label>
  local root="$1" expected="$2" label="$3" out
  if out="$(run_plan_ref_fixture "$root")"; then
    echo "FAIL (want reject): $label"
    fail=$((fail + 1))
  elif printf '%s\n' "$out" | grep -Fq -- "$expected"; then
    echo "ok       (rejected): $label"; pass=$((pass + 1))
  else
    echo "FAIL (missing diagnostic): $label"
    printf '%s\n' "$out"
    fail=$((fail + 1))
  fi
}

registry_missing="$fixture_tmp/registry-missing"
git clone -q "$fixture_base" "$registry_missing"
printf '\n## Other {#other}\n' >>"$registry_missing/docs/plan/02_positioning.md"
cat >"$registry_missing/docs/plan/AGENTS.md" <<'MD'
# Fixture Plan Rules

The prose mention `02_positioning#contract` is intentionally not a registry entry.

<!-- stable-plan-anchor-registry:start -->
| Anchor | Plan 位置 | 语义 |
|---|---|---|
| `02_positioning#other` | `## Other` | Fixture-only future anchor; planned/no-code-yet |
<!-- stable-plan-anchor-registry:end -->
MD
assert_fixture_reject_text "$registry_missing" \
  "agents-anchor-missing(blocking): 02_positioning#contract" \
  "anchor mentioned only in prose is unregistered"

registry_duplicate="$fixture_tmp/registry-duplicate"
git clone -q "$fixture_base" "$registry_duplicate"
sed -i '/<!-- stable-plan-anchor-registry:end -->/i | `02_positioning#contract` | `## Contract` | Duplicate fixture row |' \
  "$registry_duplicate/docs/plan/AGENTS.md"
assert_fixture_reject_text "$registry_duplicate" \
  "agents-anchor-registry: duplicate anchor: 02_positioning#contract" \
  "duplicate stable registry row"

echo "----"
echo "selftest: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
