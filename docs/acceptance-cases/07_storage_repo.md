## 多仓库与存储

```markdown
- case_id: STORE-001
  goal: Trinity Isolation 结构存在。
  preconditions:
    - 已执行 `deve init`
  steps:
    - run: dir "${DEVE_DATA_DIR}"
  assertions:
    - fs_exists: "${DEVE_DATA_DIR}/vault"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/local"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/remotes"

- case_id: STORE-002
  goal: Repo 命名冲突自动重命名。
  preconditions:
    - 两个 Repo 同名不同 URL
  steps:
    - run: deve repo create --name wiki --url https://a.example
    - run: deve repo create --name wiki --url https://b.example
  assertions:
    - fs_exists: "${DEVE_DATA_DIR}/ledger/local/wiki.redb"
    - fs_exists: "${DEVE_DATA_DIR}/ledger/local/wiki-1.redb"

- case_id: STORE-003
  goal: Redb 索引表存在。
  preconditions:
    - 至少一个 .redb 文件
  steps:
    - run: deve db inspect --tables ${DEVE_DATA_DIR}/ledger/local/wiki.redb
  assertions:
    - stdout_contains_all:
        - "NODEID_TO_META"
        - "PATH_TO_NODEID"
        - "LEDGER_OPS"

- case_id: STORE-004
  goal: Snapshot 双表与修剪。
  preconditions:
    - snapshot_depth=3
  steps:
    - run: deve doc edit --id <doc> --repeat 5
    - run: deve db inspect --tables ${DEVE_DATA_DIR}/ledger/local/wiki.redb
  assertions:
    - stdout_contains: "SNAPSHOT_INDEX"
    - stdout_contains: "SNAPSHOT_DATA"
    - snapshot_count_le: 3

- case_id: STORE-005
  goal: Ledger-First 与原子持久化。
  preconditions:
    - 存在一个可编辑文档 <doc>
  steps:
    - run: deve doc edit --id <doc> --set "x"
    - run: deve dump --doc <doc> --field seq
  assertions:
    - ledger_seq_increases true
    - vault_content_eq: "x"

- case_id: STORE-006
  goal: Clean File Policy。
  preconditions:
    - 文档含 Frontmatter
  steps:
    - run: type ${DEVE_DATA_DIR}/vault/frontmatter.md
  assertions:
    - file_not_contains: "uuid"
    - file_contains: "---"

- case_id: STORE-007
  goal: Watcher 事件映射与目录 scan 过滤。
  preconditions:
    - watch 运行中并监听 local repo: main
    - 已跟踪文件存在: ${DEVE_DATA_DIR}/vault/main/notes/live.md
    - 已跟踪文件存在: ${DEVE_DATA_DIR}/vault/main/notes/delete.md
    - ${DEVE_DATA_DIR}/vault/.deveignore 包含: ignored/*.md
  steps:
    - run: powershell -Command "'new' | Set-Content ${DEVE_DATA_DIR}/vault/main/notes/new.md"
    - run: powershell -Command "'dirty' | Set-Content ${DEVE_DATA_DIR}/vault/main/notes/live.md"
    - run: powershell -Command "Remove-Item ${DEVE_DATA_DIR}/vault/main/notes/delete.md"
    - run: powershell -Command "New-Item -ItemType Directory -Force ${DEVE_DATA_DIR}/vault/main/ignored"
    - run: powershell -Command "'ignored' | Set-Content ${DEVE_DATA_DIR}/vault/main/ignored/scratch.md"
  assertions:
    - pending_fs_ops_contains: "notes/new.md:Added"
    - pending_fs_ops_contains: "notes/live.md:Modified"
    - pending_fs_ops_contains: "notes/delete.md:Deleted"
    - pending_fs_ops_not_contains: "ignored/scratch.md"
    - ledger_op_not_appended: "notes/new.md"

- case_id: STORE-008
  goal: 数据恢复策略。
  preconditions:
    - 备份 ledger
  steps:
    - run: powershell -Command "Remove-Item -Recurse ${DEVE_DATA_DIR}/vault"
    - run: deve recover --from-ledger
  assertions:
    - fs_exists: "${DEVE_DATA_DIR}/vault"
    - vault_rebuilt true

- case_id: STORE-009
  goal: UUID-First Retrieval。
  preconditions:
    - API 接口支持 Name/Path
  steps:
    - run: deve api call --path-by-name "file.md"
  assertions:
    - log_contains: "resolve_to_uuid"

- case_id: STORE-010
  goal: 路径规范化。
  preconditions:
    - Windows 路径输入
  steps:
    - run: deve path normalize "folder\\sub\\a.md"
  assertions:
    - stdout_eq: "folder/sub/a.md"

- case_id: STORE-011
  goal: Browser storage authority boundary。
  preconditions:
    - WebLightPeer 运行在浏览器环境
  steps:
    - run: cargo test -p deve_web storage_capabilities -- --nocapture
    - run: cargo test -p deve_web typed_prefs_roundtrip -- --nocapture
    - run: cargo test -p deve_web shortcut_config_roundtrips -- --nocapture
    - run: cargo test -p deve_web locale_preference_uses_ui_prefs -- --nocapture
    - run: scripts/check-browser-prefs-boundary.sh
    - run: cargo test -p deve_web output_write_classification -- --nocapture
  assertions:
    - WebCrypto_Ed25519_key_extractable_false: true
    - IndexedDB_missing_enters_DegradedSyncMode: true
    - ui_prefs_use_fallback_layer_only: true
    - degraded_mode_blocks_RegisterWriter_and_SyncPush: true
    - degraded_mode_allows_read_and_snapshot_pull: true
```
