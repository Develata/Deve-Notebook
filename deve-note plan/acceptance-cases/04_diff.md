## Diff 与合并

```markdown
- case_id: DIFF-001
  goal: UTF-16 索引一致。
  preconditions:
    - 文档包含 emoji: "A😀B"
  steps:
    - ui_place_cursor_after: "A😀"
    - ui_type: "X"
    - run: deve dump --doc current --field last_op
  assertions:
    - stdout_contains: "utf16_index"

- case_id: DIFF-002
  goal: 3-Way Merge 使用 LCA。
  preconditions:
    - Local 与 Remote 均基于同一 Base 修改
  steps:
    - run: deve merge --peer <peer_id>
  assertions:
    - log_contains: "LCA"

- case_id: DIFF-003
  goal: 冲突检测按 Hunk 触发。
  preconditions:
    - Local 与 Remote 修改同一段落
  steps:
    - run: deve merge --peer <peer_id>
  assertions:
    - ui_assert: conflict_view_open true

- case_id: DIFF-004
  goal: 冲突 UI 三种策略。
  preconditions:
    - 已进入冲突界面
  steps:
    - ui_click: "Accept Current"
    - ui_click: "Accept Incoming"
    - ui_click: "Accept Both"
  assertions:
    - ui_assert: result_matches_strategy true

- case_id: DIFF-005
  goal: 合并中断可续。
  preconditions:
    - 合并进行中
  steps:
    - run: taskkill /F /IM deve_cli.exe
    - run: deve merge --peer <peer_id>
  assertions:
    - log_contains: "resume"
    - ledger_ops_not_lost true

- case_id: DIFF-006
  goal: Watcher 防抖与 Hash 校验。
  preconditions:
    - `deve watch` 运行中
  steps:
    - run: powershell -Command "1..5 | % { 'x' | Add-Content ${DEVE_DATA_DIR}/vault/debounce.md }"
  assertions:
    - ledger_op_count_increases_by: 1

- case_id: DIFF-007
  goal: Diff 颜色语义。
  preconditions:
    - 文档包含新增/修改/删除
  steps:
    - ui_open_diff: true
  assertions:
    - ui_assert: gutter_color_added "var(--color-added)"
    - ui_assert: gutter_color_modified "var(--color-modified)"
    - ui_assert: gutter_color_deleted "var(--color-deleted)"

- case_id: DIFF-008
  goal: 长文档打开策略。
  preconditions:
    - 文档含快照与大量 Ops
  steps:
    - ui_open_doc: "large.md"
  assertions:
    - ui_assert: snapshot_first true
    - ui_assert: search_disabled_until_prefetch_complete true
```
