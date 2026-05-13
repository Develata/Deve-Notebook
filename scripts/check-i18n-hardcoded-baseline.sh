#!/usr/bin/env bash
set -euo pipefail

# I18N-001 guard: Command/Search visible helper text must be routed through
# the i18n facade instead of being embedded in UI/provider code.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPONENT_SCOPE="$ROOT_DIR/apps/web/src/components"

fail() {
  echo "i18n-hardcoded-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg --fixed-strings --quiet "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

reject_component_literal() {
  local pattern="$1"
  local label="$2"
  if rg --fixed-strings \
    --glob '!**/tests.rs' \
    --glob '!**/*_test.rs' \
    --glob '!**/*_tests.rs' \
    --quiet "$pattern" "$COMPONENT_SCOPE/command_palette" "$COMPONENT_SCOPE/search_box"; then
    rg -n --fixed-strings \
      --glob '!**/tests.rs' \
      --glob '!**/*_test.rs' \
      --glob '!**/*_tests.rs' \
      "$pattern" "$COMPONENT_SCOPE/command_palette" "$COMPONENT_SCOPE/search_box" >&2
    fail "$label must use t::* facade"
  fi
}

contains docs/acceptance-cases/09_i18n.md "scripts/check-i18n-hardcoded-baseline.sh"
contains apps/web/src/i18n/command_palette.rs "pub fn keyboard_navigate_hint"
contains apps/web/src/i18n/search.rs "pub fn command_detail"
contains apps/web/src/i18n/search.rs "pub fn file_op_detail"

reject_component_literal "to navigate" "keyboard navigation copy"
reject_component_literal "to select" "keyboard selection copy"
reject_component_literal "to close" "keyboard close copy"
reject_component_literal 'Some("Command".to_string())' "command search detail"
reject_component_literal 'Some("Current Branch".to_string())' "branch current detail"
reject_component_literal 'Some("Remote Branch".to_string())' "branch remote detail"
reject_component_literal 'format!("Create/Open' "create/open title"
reject_component_literal 'Some("New File".to_string())' "new-file detail"
reject_component_literal 'Some("FileOp".to_string())' "file-op detail"
reject_component_literal 'Some("Group".to_string())' "group detail"
reject_component_literal 'Some("Error".to_string())' "error detail"
reject_component_literal '"Paths with spaces must be quoted".to_string()' "file-op quoted-path error"
reject_component_literal '"Unclosed quote".to_string()' "file-op quote error"
reject_component_literal '"Usage: >rm <path>".to_string()' "remove usage copy"
reject_component_literal 'format!("Move:' "move file-op title"
reject_component_literal 'format!("Copy:' "copy file-op title"
reject_component_literal 'format!("Remove:' "remove file-op title"

echo "i18n-hardcoded-baseline-check: ok"
