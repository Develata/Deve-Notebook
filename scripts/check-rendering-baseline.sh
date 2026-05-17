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
contains docs/plan/03_rendering.md "Rendering Capability Boundary"
contains docs/plan/03_rendering.md "Extended target"
contains docs/plan/03_rendering.md "辅助 Markdown-to-HTML 渲染器"
contains docs/plan/03_rendering.md "不得被视为主编辑器 hybrid engine"
contains docs/plan/03_rendering.md "独立 Preview Projection / Live Preview / Milkdown"
contains docs/plan/03_rendering.md "真正跨超大文档的完整 virtual rendering"
contains docs/plan/03_rendering.md "rendering settings 的完整 GUI 持久化"
contains docs/features/03_rendering.md "下列能力不得作为本功能篇的已完成验收目标"
contains docs/features/03_rendering.md "不承担主编辑器职责"
contains docs/features/03_rendering.md "不宣称完整 virtual rendering"

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

# RENDER-LINK-002: rendered external links keep target/rel safety attributes.
contains docs/acceptance-cases/03_rendering.md "case_id: RENDER-LINK-002"
contains docs/acceptance-cases/03_rendering.md 'ui_dom_attr_eq: ["target", "_blank"]'
contains docs/acceptance-cases/03_rendering.md 'ui_dom_attr_eq: ["rel", "noopener noreferrer"]'
contains apps/web/src/utils/markdown.rs 'target="_blank" and rel="noopener noreferrer"'
contains docs/report/rendering-interaction-spot-smoke-2026-05-13.md 'window.open("https://example.com", "_blank", "noopener,noreferrer")'

# RENDER-WHITELIST-001: unsupported highlight stays plain and arbitrary div HTML is filtered.
contains docs/acceptance-cases/03_rendering.md "case_id: RENDER-WHITELIST-001"
contains docs/acceptance-cases/03_rendering.md "highlight_not_rendered true"
contains docs/acceptance-cases/03_rendering.md "html_div_filtered true"
contains apps/web/src/utils/markdown.rs "html_filter_allows_br_only"
contains apps/web/src/utils/markdown.rs "unsupported_highlight_syntax_stays_plain_text"
contains docs/report/rendering-interaction-spot-smoke-2026-05-13.md "==plain=="
contains apps/web/src/components/chat/message_item.rs "chat_apply_label_is_build_only_for_assistant_messages"
contains apps/web/src/components/chat/message_list.rs "chat_apply_click_is_consumed_only_in_build_mode"
contains apps/web/src/components/chat/actions/apply.rs "chat_apply_edit_message_carries_current_scope_nonce"
contains scripts/check-ai-baseline.sh "chat_apply_edit_message_carries_current_scope_nonce"
contains apps/web/src/components/outline_render/scan.rs "pub(super) fn next_char_at"
contains apps/web/src/components/outline_render/parse_test.rs "outline_scan_helpers_fail_soft_on_non_char_boundary_start"
absent apps/web/src/components/outline_render/parse.rs "chars().next().unwrap()"
absent apps/web/src/components/outline_render/scan.rs "chars().next().unwrap()"
contains docs/features/operations/rendering_link_activation_gate.md "apps/web/src/hooks/use_ctrl_key.rs"
contains apps/web/src/hooks/use_ctrl_key.rs "fn body() -> Option<web_sys::HtmlElement>"
contains apps/web/src/hooks/use_ctrl_key.rs "fn browser_window() -> Option<web_sys::Window>"
contains apps/web/src/hooks/use_ctrl_key.rs "ctrl_key_dom_helpers_fail_soft_without_browser_window"
absent apps/web/src/hooks/use_ctrl_key.rs "expect(\"window\")"
absent apps/web/src/hooks/use_ctrl_key.rs "expect(\"document\")"
absent apps/web/src/hooks/use_ctrl_key.rs "body().unwrap()"

# CodeMirror toolbar keeps only the extension point by default; future actions must register real handlers.
# RENDER-CODE-001: code block toolbar exposes copy + menu, with empty state when no actions are registered.
contains docs/acceptance-cases/03_rendering.md "case_id: RENDER-CODE-001"
contains docs/acceptance-cases/03_rendering.md 'toolbar_has_buttons ["Copy", "Ellipsis"]'
contains docs/acceptance-cases/03_rendering.md 'menu_empty_state_text "No actions available"'
contains apps/web/js/extensions/code_toolbar.js "ICON_COPY"
contains apps/web/js/extensions/code_toolbar.js "ICON_ELLIPSIS"
contains apps/web/js/extensions/code_menu.js 'editorCopy("noActionsAvailable")'
contains apps/web/js/init.js "window.deve_code_actions = window.deve_code_actions || [];"
absent apps/web/js/init.js "Run Code"
absent apps/web/js/init.js "Send to AI"
absent apps/web/js/init.js "TODO: Connect to backend"

# RENDER-NEST-001: nested list/quote/math rendering keeps depth decorations and smoke evidence.
contains docs/acceptance-cases/03_rendering.md "case_id: RENDER-NEST-001"
contains docs/acceptance-cases/03_rendering.md "nesting_indentation_consistent true"
contains docs/acceptance-cases/03_rendering.md "background_layers_correct true"
contains apps/web/js/extensions/blockquote_border.js "cm-blockquote-depth-\${effectiveDepth}"
contains apps/web/js/extensions/math.js "cm-nested-math-depth-\${effectiveDepth}"
contains apps/web/style/_blockquote.css '[class*="cm-blockquote-depth-"]'
contains apps/web/style/_math.css '[class*="cm-nested-math-depth-"]'
contains docs/report/rendering-interaction-spot-smoke-2026-05-13.md "Nested rendering"
contains docs/report/rendering-interaction-spot-smoke-2026-05-13.md "cm-blockquote-depth-1"

# Current release must not claim full renderer or settings persistence as implemented.
absent docs/features/03_rendering.md "完整 preview mode 已实现"
absent docs/features/03_rendering.md "完整 virtual rendering 已实现"
absent docs/features/03_rendering.md "rendering settings 的完整 GUI 持久化已实现"

echo "rendering-baseline-check: ok"
