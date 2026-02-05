## 项目定位与核心边界

```markdown
- case_id: POS-001
  goal: `deve init` 初始化 Vault 与 Ledger 目录。
  preconditions:
    - env: DEVE_DATA_DIR 指向空目录
  steps:
    - run: deve init
  assertions:
    - fs_exists: "${DEVE_DATA_DIR}/vault"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/local"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/remotes"

- case_id: POS-002
  goal: `deve watch` 可处理外部编辑器原子写。
  preconditions:
    - deve watch 已启动并监听 vault
    - 文件存在: ${DEVE_DATA_DIR}/vault/test.md
  steps:
    - run: powershell -Command "'v1' | Set-Content -Path ${DEVE_DATA_DIR}/vault/test.md"
    - run: powershell -Command "Start-Sleep -Milliseconds 500"
  assertions:
    - log_contains: "watch"  # Watcher 有事件日志
    - ledger_op_appended: "test.md"  # 产生 Op（可通过 dump 验证）

- case_id: POS-003
  goal: 双向闭环无死循环。
  preconditions:
    - deve watch 正在运行
  steps:
    - run: powershell -Command "'loop-test' | Set-Content -Path ${DEVE_DATA_DIR}/vault/loop.md"
    - run: powershell -Command "Start-Sleep -Milliseconds 1000"
  assertions:
    - log_not_contains: "repeat-trigger"  # 不出现重复循环标记
    - ledger_op_count_increases_by: 1

- case_id: POS-004
  goal: 重命名不丢 DocId。
  preconditions:
    - 文件存在: ${DEVE_DATA_DIR}/vault/rename_a.md
  steps:
    - run: deve dump --path ${DEVE_DATA_DIR}/vault/rename_a.md --field doc_id
    - run: powershell -Command "Rename-Item ${DEVE_DATA_DIR}/vault/rename_a.md rename_b.md"
    - run: deve dump --path ${DEVE_DATA_DIR}/vault/rename_b.md --field doc_id
  assertions:
    - stdout_eq: "doc_id_before == doc_id_after"

- case_id: POS-005
  goal: `.deveignore` 生效。
  preconditions:
    - 文件存在: ${DEVE_DATA_DIR}/vault/.deveignore
  steps:
    - run: powershell -Command "'*.tmp' | Set-Content -Path ${DEVE_DATA_DIR}/vault/.deveignore"
    - run: powershell -Command "'x' | Set-Content -Path ${DEVE_DATA_DIR}/vault/ignored.tmp"
    - run: powershell -Command "Start-Sleep -Milliseconds 500"
  assertions:
    - ledger_op_not_appended: "ignored.tmp"
    - tree_not_contains: "ignored.tmp"

- case_id: POS-006
  goal: 核心禁止项不默认启用。
  preconditions:
    - 应用使用默认配置启动
  steps:
    - run: rg -n "Tantivy|AI|Code Execution" "deve-note plan/02_positioning.md"
  assertions:
    - stdout_contains: "Core MUST NOT"
```
