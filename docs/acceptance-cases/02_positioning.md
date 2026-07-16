## 项目定位与核心边界

```markdown
- case_id: POS-001
  goal: `deve init` 初始化 Ledger、local repo 与 Projection Locator。
  preconditions:
    - env: DEVE_DATA_DIR 指向空目录
  steps:
    - run: deve init --path ${DEVE_DATA_DIR} --repo main --projection-base ${DEVE_DATA_DIR}/notes
  assertions:
    - fs_exists: "${DEVE_DATA_DIR}/notes/main"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/.host/projection-locators.toml"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/local"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/remotes"

- case_id: POS-002
  goal: `deve watch` 可处理外部编辑器原子写。
  preconditions:
    - deve watch 已启动并监听 repo Projection Workspace
    - 文件存在: ${DEVE_DATA_DIR}/notes/main/test.md
  steps:
    - run: powershell -Command "'v1' | Set-Content -Path ${DEVE_DATA_DIR}/notes/main/test.md"
    - run: powershell -Command "Start-Sleep -Milliseconds 500"
    - run: cargo test -p deve_core --test watcher_platform_fs watcher_atomic_replace_records_single_final_candidate -- --nocapture
  assertions:
    - log_contains: "watch"  # Watcher 有事件日志
    - pending_fs_ops_contains: "test.md"  # 仅进入工作区待确认集合
    - ledger_op_not_appended: "test.md"
    - api_assert: atomic_replace_has_one_final_pending_candidate true

- case_id: POS-003
  goal: 双向闭环无死循环。
  preconditions:
    - deve watch 正在运行
  steps:
    - run: powershell -Command "'loop-test' | Set-Content -Path ${DEVE_DATA_DIR}/notes/main/loop.md"
    - run: powershell -Command "Start-Sleep -Milliseconds 1000"
    - run: cargo test -p deve_core --test watcher_writeback_loop -- --nocapture
  assertions:
    - log_not_contains: "repeat-trigger"  # 不出现重复循环标记
    - pending_fs_ops_count_increases_by: 1
    - api_assert: projection_writeback_absent_after_watcher_liveness_sentinel true

- case_id: POS-004
  goal: 重命名不丢 DocId。
  preconditions:
    - 文件存在: ${DEVE_DATA_DIR}/notes/main/rename_a.md
  steps:
    - run: deve dump --path ${DEVE_DATA_DIR}/notes/main/rename_a.md
    - run: powershell -Command "Rename-Item ${DEVE_DATA_DIR}/notes/main/rename_a.md rename_b.md"
    - run: deve dump --path ${DEVE_DATA_DIR}/notes/main/rename_b.md
    - run: cargo test -p deve_core watcher_pairs_rename_and_preserves_doc_identity -- --nocapture
  assertions:
    - stdout_contains: "DocId:"
    - api_assert: rename_preserves_doc_identity true

- case_id: POS-005
  goal: `.deveignore` 对 watcher 与 startup scan 均生效。
  preconditions:
    - 已初始化 local repo: main
  steps:
    - run: powershell -Command "'ignored/*.md' | Set-Content -Path ${DEVE_DATA_DIR}/notes/main/.deveignore"
    - run: powershell -Command "New-Item -ItemType Directory -Force ${DEVE_DATA_DIR}/notes/main/ignored"
    - run: powershell -Command "'x' | Set-Content -Path ${DEVE_DATA_DIR}/notes/main/ignored/scratch.md"
    - run: start-background deve watch
    - run: powershell -Command "'y' | Set-Content -Path ${DEVE_DATA_DIR}/notes/main/ignored/later.md"
    - run: powershell -Command "Start-Sleep -Milliseconds 500"
  assertions:
    - pending_fs_ops_not_contains: "ignored/scratch.md"
    - pending_fs_ops_not_contains: "ignored/later.md"
    - ledger_op_not_appended: "ignored/scratch.md"
    - ledger_op_not_appended: "ignored/later.md"
    - tree_not_contains: "ignored/scratch.md"

- case_id: POS-006
  goal: 核心禁止项不默认启用。
  preconditions:
    - 应用使用默认配置启动
  steps:
    - run: rg -n "Tantivy|AI|Code Execution" "docs/plan/02_positioning.md"
  assertions:
    - stdout_contains: "Core MUST NOT"
```
