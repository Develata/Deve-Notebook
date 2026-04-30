#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "rendering-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

# Plan/features must keep the current renderer distinct from future hybrid work.
contains docs/plan/03_rendering.md "Current Implementation Split"
contains docs/plan/03_rendering.md "Future / Planned 能力"
contains docs/plan/03_rendering.md "轻量 Markdown-to-HTML 渲染器"
contains docs/plan/03_rendering.md "它不是主编辑器 hybrid engine"
contains docs/plan/03_rendering.md "独立 Preview Projection / Live Preview / Milkdown"
contains docs/plan/03_rendering.md "真正跨超大文档的完整 virtual rendering"
contains docs/plan/03_rendering.md "rendering settings 的完整 GUI 持久化"
contains docs/features/03_rendering.md "下列能力属于 Future / Planned"
contains docs/features/03_rendering.md "它不是主编辑器 hybrid engine"
contains docs/features/03_rendering.md "不等价于完整 virtual rendering"

# Lightweight renderer contract: narrow Markdown subset, secure links, and BUILD apply affordance only.
contains apps/web/src/utils/markdown.rs "Options::ENABLE_TABLES"
contains apps/web/src/utils/markdown.rs "Options::ENABLE_STRIKETHROUGH"
contains apps/web/src/utils/markdown.rs "Options::ENABLE_TASKLISTS"
contains apps/web/src/utils/markdown.rs "code_block_omits_apply_button_without_label"
contains apps/web/src/utils/markdown.rs "code_block_includes_apply_button_with_label"
contains apps/web/src/utils/markdown.rs "html_filter_allows_br_only"
contains apps/web/src/utils/markdown.rs "link_rendering_adds_blank_target_and_rel"
contains apps/web/src/utils/markdown.rs "link_rendering_rejects_script_scheme"
contains apps/web/src/utils/markdown.rs "unsupported_highlight_syntax_stays_plain_text"
contains apps/web/src/components/chat/message_item.rs "apply_label_is_build_only_for_assistant_messages"
contains apps/web/src/components/chat/message_list.rs "apply_click_is_consumed_only_in_build_mode"
contains apps/web/src/components/chat/actions_apply.rs "apply_edit_message_carries_current_scope_nonce"
contains scripts/check-ai-baseline.sh "apply_edit_message_carries_current_scope_nonce"

# Current release must not claim full renderer or settings persistence as implemented.
absent docs/features/03_rendering.md "完整 preview mode 已实现"
absent docs/features/03_rendering.md "完整 virtual rendering 已实现"
absent docs/features/03_rendering.md "rendering settings 的完整 GUI 持久化已实现"

echo "rendering-baseline-check: ok"
