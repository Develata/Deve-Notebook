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
    - 文档包含 [link](https://example.com)
  steps:
    - ui_click: "link"
    - ui_keydown: "Ctrl"
    - ui_click: "link"
    - ui_keyup: "Ctrl"
  assertions:
    - ui_assert: navigation_not_triggered_first_click true
    - ui_assert: navigation_triggered_second_click true

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
  goal: 大文档首屏优先渲染。
  preconditions:
    - 文档大小 >= 1MB
  steps:
    - ui_open_doc: "large.md"
    - ui_time_to_first_paint: true
  assertions:
    - metric_lt_ms: ["first_paint", 2000]
    - ui_assert: virtual_render_enabled true

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
    - ui_assert: menu_empty_state_text "No actions available"

- case_id: RENDER-WHITELIST-001
  goal: 语法白名单与限制。
  preconditions:
    - 文档包含 `==highlight==` 与 `<div>`
  steps:
    - ui_wait_render: true
  assertions:
    - ui_assert: highlight_not_rendered true
    - ui_assert: html_div_filtered true

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
```
