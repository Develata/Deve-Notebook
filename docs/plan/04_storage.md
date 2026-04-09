# 04_storage.md - 存储工程蓝图

本章只定义 authority storage、projection persistence、repair 与恢复合同，不描述用户提示文案。功能语义见 [../features/04_storage.md](../features/04_storage.md)，自动化验收见 [../acceptance-cases/07_storage_repo.md](../acceptance-cases/07_storage_repo.md)。

## 1. 目标

- 保持 `ledger-first`：只有 ledger append 能改变已确认业务真相。
- Vault 是 workspace projection，不是权威存储。
- 外部文件变化必须先进入 side tables，再经显式确认进入 ledger。

## 2. 权威实体

- `Ledger`
  - repo 的唯一权威事实源。
- `Snapshot`
  - 锚定 ledger head 的恢复优化结构，不是第二真相。
- `Projection`
  - tree/path/workspace/materialized views。
- `pending_fs_ops`
  - 工作区偏差候选。
- `staging`
  - 用户已确认、待提交的候选集合。
- `RepoHealth`
  - 存储与 projection 的健康状态。

## 3. 三层存储

- `Vault`
  - repo-scoped workspace projection 的物理载体。
- `Local Branch`
  - 本地可写 authority store。
- `Remote Branches`
  - 远端只读镜像 authority store。

## 4. 写入路径

### 4.1 Direct Write

- 内置编辑器 / 受控命令写入必须直接走 ledger append。
- 不允许先改 Vault 再补账本。

### 4.2 External Edit Ingestion

- watcher 只负责发现工作区偏差。
- watcher 生成的 create/modify/delete/rename 候选只能进入 `pending_fs_ops`。

### 4.3 Stage -> Commit

- `pending_fs_ops -> staging -> ledger append -> projection rebuild`
- commit 时必须把文本变化转为 `Content Facts`，结构变化转为 `Structure Facts`。

## 5. Projection 合同

- projection 只能从 ledger/snapshot 导出。
- tree/path/metadata/path_cache 都属于 projection storage 或 cache。
- projection 失败不得反向改写 ledger。

## 6. 恢复与修复

- workspace 偏离 `projection` 时，必须能被 `pending_fs_ops / staging` 解释。
- runtime tables、projection、workspace writeback 损坏时，必须支持从 ledger 重建。
- 历史坏 repo 必须进入 `Repairing / Degraded / Quarantined` 之一，不能假装健康。

## 7. 失败合同

- ledger append 成功但 workspace writeback 失败：
  - authority 仍视为已提交
  - projection/writeback 必须进入可恢复故障态
- projection rebuild 失败：
  - 可以降级
  - 不得阻断 authority 已存在的事实
- side table 无法解释 workspace 偏差：
  - 视为状态漂移故障

## 8. 禁止事项

- 禁止 metadata/path table 直写替代 structure facts。
- 禁止 watcher 自动把外部变化直接写入 ledger。
- 禁止 Vault 内容被当成已确认权威状态。
- 禁止 projection cache 反向污染 authority。

## 9. 代码边界

- `crates/core/src/ledger/`
  - authority append/query/runtime tables。
- `crates/core/src/sync/`
  - projection/materialize/rebuild。
- `apps/cli/src/server/handlers/document/`
  - 受控写入入口。
- `apps/web/src/hooks/use_core/`
  - 只消费确认态与 pending overlay，不持有第二真相。
