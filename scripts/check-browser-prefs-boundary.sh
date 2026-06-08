#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "browser-prefs-boundary-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

direct_hits="$(rg -n 'window\(\)\.local_storage|\.local_storage\(|globalThis\.localStorage|sessionStorage' "$ROOT_DIR/apps/web/src" -g '*.rs' || true)"
unexpected_hits="$(printf '%s\n' "$direct_hits" \
  | rg -v 'apps/web/src/storage/prefs\.rs|apps/web/src/storage/js_bridge\.rs' || true)"

if [[ -n "$unexpected_hits" ]]; then
  printf '%s\n' "$unexpected_hits" >&2
  fail "direct browser storage access must go through apps/web/src/storage/prefs.rs unless it is capability probing"
fi

check_contains apps/web/src/hooks/use_layout/storage.rs "read_i32_pref"
check_contains apps/web/src/hooks/use_outline.rs "read_bool_pref"
check_contains apps/web/src/shortcuts/config.rs "read_pref"
check_contains apps/web/src/components/settings_prefs.rs "read_pref"
check_contains apps/web/src/components/settings_prefs.rs "write_pref"
check_contains apps/web/src/components/settings_prefs.rs "read_bool_pref"
check_contains apps/web/src/components/settings_prefs.rs "write_bool_pref"
check_contains apps/web/src/components/settings_prefs.rs "deve.ui.ai_chat_visible"
check_contains apps/web/src/i18n/mod.rs "write_pref"
check_contains apps/web/src/storage/prefs.rs "typed_prefs_roundtrip_through_fallback_layer"
check_contains apps/web/src/hooks/use_core/scope_prefs.rs "repo_name: String"
check_contains apps/web/src/hooks/use_core/scope_prefs.rs "fn serialize_scope_pref(repo_name: String) -> Option<String>"
check_absent apps/web/src/hooks/use_core/scope_prefs.rs "expect(\"scope pref should serialize\")"
check_absent apps/web/src/hooks/use_core/scope_prefs.rs "repo_id:"
check_absent apps/web/src/hooks/use_core/scope_prefs.rs "active_branch:"

echo "browser-prefs-boundary-check: ok"
