## Markdown 渲染

```markdown
- case_id: RENDER-BLOCK-001
  goal: 块级解析优先级生效。
  preconditions:
    - 打开编辑器并创建文档 render_block.md
  steps:
    - ui_type: |
        ```
        $not_math$
        ```
        $$a^2$$
        <div>html</div>
    - ui_wait_render: true
  assertions:
    - ui_assert: code_block_contains_literal "${not_math}"  # 代码块内不渲染公式
    - ui_assert: math_block_rendered "a^2"
    - ui_assert: html_block_filtered "<div>"

- case_id: RENDER-INLINE-001
  goal: 行内解析与转义优先级。
  preconditions:
    - 打开 render_inline.md
  steps:
    - ui_type: "`code $x$` \\$$ \\|"
    - ui_wait_render: true
  assertions:
    - ui_assert: inline_code_contains_literal "code $x$"
    - ui_assert: text_contains_literal "$"
    - ui_assert: text_contains_literal "|"

- case_id: RENDER-HEADING-001
  goal: 空与非空 ATX 标题行保持层级行高。
  preconditions:
    - 打开 render_heading.md
  steps:
    - run: scripts/check-rendering-baseline.sh
    - run: cargo test -p deve_web empty_atx_headings -- --nocapture
    - run: cargo test -p deve_web markdown_heading_modes -- --nocapture
    - run: node apps/web/js/extensions/block_styling.test.cjs
    - ui_type: |
        #
        ##
        ###
        # title
        # s
        ## s
        ### s
        ## title
        ### title
        #申话
    - ui_move_cursor_into: "#申话"
    - ui_wait_render: true
  assertions:
    - ui_assert: editor_heading_line_classes ["cm-heading-line-1", "cm-heading-line-2", "cm-heading-line-3"]
    - ui_assert: nonempty_editor_heading_line_classes ["cm-heading-line-1", "cm-heading-line-2", "cm-heading-line-3"]
    - ui_assert: markdown_body_empty_heading_heights_distinct true
    - ui_assert: nonempty_heading_keeps_layered_height true
    - ui_assert: nonempty_heading_font_scales_match_levels true
    - ui_assert: nonempty_heading_not_double_scaled true
    - ui_assert: heading_with_inline_math_keeps_line_class true
    - ui_assert: heading_opener_inside_math_frontmatter_not_styled true
    - ui_assert: atx_heading_lines_have_heading_class true
    - ui_assert: atx_heading_text_line_height_gt_plain true
    - ui_assert: atx_empty_heading_line_height_gt_plain true
    - ui_assert: atx_nonempty_short_heading_line_height_gt_plain true
    - ui_assert: atx_active_cjk_candidate_line_height_gt_plain true
    - ui_assert: source_mode_heading_line_height_gt_plain true
    - ui_assert: hybrid_active_heading_line_height_gt_plain true
    - ui_assert: hybrid_inactive_heading_mark_hidden_and_line_height_gt_plain true
    - ui_assert: preview_mode_heading_line_height_gt_plain true
    - ui_assert: preview_mode_markdown_source_hidden true
    - ui_assert: preview_mode_editing_disabled true

- case_id: RENDER-CURSOR-001
  goal: 光标揭示规则。
  preconditions:
    - 文档包含 $a^2$、**b**、~~c~~、Frontmatter。
  steps:
    - ui_move_cursor_into: "$a^2$"
    - ui_move_cursor_into: "**b**"
    - ui_move_cursor_into: "---"
  assertions:
    - ui_assert: source_visible_for_current_token true

- case_id: RENDER-LINK-001
  goal: 链接需 Ctrl/Cmd 激活。
  preconditions:
    - 文档包含 [link](https://example.com) 与 [bad](javascript:alert(1))
  steps:
    - ui_click: "link"
    - ui_keydown: "Ctrl"
    - ui_click: "link"
    - ui_click: "bad"
    - ui_keyup: "Ctrl"
  assertions:
    - ui_assert: navigation_not_triggered_first_click true
    - ui_assert: navigation_triggered_second_click true
    - ui_assert: unsafe_scheme_not_opened true

- case_id: RENDER-LINK-002
  goal: 外链安全属性强制。
  preconditions:
    - 文档包含外链
  steps:
    - ui_query_dom: "a[href^='http']"
  assertions:
    - ui_dom_attr_eq: ["target", "_blank"]
    - ui_dom_attr_eq: ["rel", "noopener noreferrer"]

- case_id: RENDER-LARGE-001
  goal: 大文档 snapshot-first 与渐进回放不阻塞首屏。
  preconditions:
    - 文档大小 >= 1MB
  steps:
    - run: scripts/check-large-doc-baseline.sh
    - run: cargo test -p deve_web large_doc_search_gate -- --nocapture
    - ui_open_doc: "large.md"
    - ui_time_to_first_paint: true
  assertions:
    - metric_lt_ms: ["first_paint", 2000]
    - ui_assert: snapshot_first true
    - ui_assert: progressive_replay_enabled true
    - ui_assert: search_disabled_until_prefetch_complete true

- case_id: RENDER-LARGE-002
  goal: 大文档 delta batch 不可应用时回退 full snapshot。
  preconditions:
    - 文档 snapshot 已到达
    - delta batch replay 失败
  steps:
    - run: scripts/check-large-doc-baseline.sh
    - run: cargo test -p deve_web snapshot_apply_failure -- --nocapture
  assertions:
    - cli_assert: remote_batch_apply_returns_failure true
    - cli_assert: failed_batch_does_not_advance_version_or_history true
    - ui_assert: full_snapshot_fallback_requested true

- case_id: RENDER-MATH-001
  goal: 公式渲染与折叠。
  preconditions:
    - 打开 render_math.md
  steps:
    - ui_type: "$$a^2$$"
    - ui_keypress: "Ctrl+Enter"
  assertions:
    - ui_assert: math_block_rendered "a^2"
    - ui_assert: source_collapsed true

- case_id: RENDER-MERMAID-001
  goal: Mermaid 静态渲染与尺寸。
  preconditions:
    - 文档包含 ```mermaid``` 代码块
  steps:
    - ui_wait_render: true
    - ui_query_dom: "svg.mermaid"
  assertions:
    - ui_assert: network_requests_count 0
    - ui_assert: svg_width_percent 100

- case_id: RENDER-RICH-001
  goal: 任务列表回写源码。
  preconditions:
    - 文档包含 "- [ ] task"
  steps:
    - ui_click: "task_checkbox"
  assertions:
    - ui_assert: checkbox_checked true
    - ui_assert: source_contains "- [x] task"

- case_id: RENDER-RICH-002
  goal: Frontmatter 样式与揭示。
  preconditions:
    - 文档含 Frontmatter
  steps:
    - ui_move_cursor_outside: "frontmatter"
    - ui_move_cursor_inside: "frontmatter"
  assertions:
    - ui_assert: frontmatter_delimiter_hidden true
    - ui_assert: frontmatter_delimiter_visible true

- case_id: RENDER-CODE-001
  goal: 代码块工具栏与空状态。
  preconditions:
    - 文档含代码块
  steps:
    - ui_hover: "code_block"
    - ui_click: "ellipsis"
  assertions:
    - ui_assert: toolbar_has_buttons ["Copy", "Ellipsis"]
    - ui_assert: menu_empty_state_text localized_editor_copy "noActionsAvailable"
    - ui_assert: code_toolbar_action_markers ["copy", "ellipsis"]
    - ui_assert: code_menu_empty_state_marker_visible true
    - ui_assert: code_menu_empty_state_uses_i18n_key "noActionsAvailable"

- case_id: RENDER-BRIDGE-001
  goal: Editor / widget browser globals 必须经 bridge registry 暴露。
  preconditions:
    - Web bridge registry 已在 editor adapter 之前加载。
  steps:
    - run: node apps/web/js/web_bridge_registry.test.cjs
  assertions:
    - cli_assert: editor_adapter_globals_registered_through_bridge true
    - cli_assert: index_editor_wrappers_registered_through_bridge true
    - cli_assert: index_html_bridge_logic_externalized true
    - cli_assert: index_editor_bootstrap_state_registered_through_bridge true
    - cli_assert: index_editor_bootstrap_state_avoids_direct_window_fields true
    - cli_assert: index_editor_wrappers_avoid_direct_window_calls true
    - cli_assert: bridge_registry_get_call_facade_bound true
    - cli_assert: index_mobile_editor_stubs_do_not_read_debug_view true
    - cli_assert: index_boot_helpers_registered_through_bridge true
    - cli_assert: index_boot_helpers_avoid_direct_window_calls true
    - cli_assert: index_error_handler_does_not_assign_window_onerror true
    - cli_assert: init_code_actions_registered_through_bridge true
    - cli_assert: init_i18n_registered_through_bridge true
    - cli_assert: init_script_order_keeps_registry_before_init true
    - cli_assert: code_menu_does_not_assign_action_registry true
    - cli_assert: bridge_registry_missing_fails_closed true
    - cli_assert: gutter_diff_extension_does_not_bypass_bridge true
    - cli_assert: chat_math_globals_registered_through_bridge true
    - cli_assert: chat_math_missing_registry_fails_closed true
    - cli_assert: native_backend_config_global_registered_through_bridge true

- case_id: RENDER-WHITELIST-001
  goal: 语法白名单与限制。
  preconditions:
    - 文档包含 `==highlight==`、`<div>`、`H~2~O`、`^sup^` 与 `:smile:`
  steps:
    - ui_wait_render: true
  assertions:
    - ui_assert: highlight_not_rendered true
    - ui_assert: html_div_filtered true
    - ui_assert: extended_inline_syntax_plain_text ["H~2~O", "^sup^", ":smile:"]

- case_id: RENDER-NEST-001
  goal: 深度嵌套渲染稳定。
  preconditions:
    - 文档含 List -> Quote -> List -> Code/Math 嵌套
  steps:
    - ui_wait_render: true
  assertions:
    - ui_assert: nesting_indentation_consistent true
    - ui_assert: background_layers_correct true

- case_id: RENDER-OUTLINE-001
  goal: Outline 解析规则。
  preconditions:
    - 标题含 **bold**/*italic*/~~strike~~/`code`/$a^2$ 与 `==highlight==`
  steps:
    - ui_open_outline: true
  assertions:
    - ui_assert: outline_contains_math true
    - ui_assert: outline_treats_highlight_as_text true
    - ui_assert: outline_atx_empty_tab_and_closing_headings_supported true
    - ui_assert: outline_heading_items_are_buttons true
```
