#!/usr/bin/env bash
set -euo pipefail

# I18N-005 guard: visible frontend time formatting must go through the
# locale-aware formatting utility rather than component-local HH:MM assembly.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHAT_ITEM="$ROOT_DIR/apps/web/src/components/chat/message_item.rs"
TIME_UTIL="$ROOT_DIR/apps/web/src/utils/time.rs"

fail() {
  echo "i18n-formatting-baseline-check: $*" >&2
  exit 1
}

rg --fixed-strings --quiet 'format_time_of_day' "$CHAT_ITEM" \
  || fail "chat timestamp must use format_time_of_day"

rg --fixed-strings --quiet 'to_locale_time_string' "$TIME_UTIL" \
  || fail "time utility must use Intl locale time formatting"

if rg --fixed-strings --quiet 'format!("{:02}:{:02}"' "$ROOT_DIR/apps/web/src"; then
  fail "manual HH:MM formatting is not allowed in frontend source"
fi

echo "i18n-formatting-baseline-check: ok"
