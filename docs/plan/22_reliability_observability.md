# 22_reliability_observability.md - Reliability & Observability (可靠性与可观测性)

## Metadata

- `Layer`: `Governance Contracts (non-layer ownership-axis slice)`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-05`
- `Authority Owns`: `SLO/SLI catalog / telemetry schema / metrics taxonomy / tracing span boundary / observation-to-health mapping / alerting tier 映射 / DR playbook index`
- `Authority Defers To`: `04_repository#repo-health-and-repair (degraded 状态全集与状态迁移), 13_i18n#i18n-error-code-catalog (错误码), 17_tech_stack#performance-profiles-and-feature-matrix (profile), 18_release#runtime-observability (运维观测 endpoint), 21_perf_budget (latency/RSS budget), 06_backup (DR/恢复步骤)`
- `Counterpart Feature`: `docs/features/operation-coverage.md (release / observability flows)`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md (REL-013)`
- `Primary Code Areas`: `crates/core/src/` 各 runtime 的 tracing / log / metric 实现位置；`apps/cli/src/server/` observability endpoint

## 1. Scope & Authority {#reliability-observability-scope}

本章是**可观测性契约唯一权威**：定义遥测/指标/追踪的结构标准与告警映射。

- **Owns**：SLO/SLI catalog（§2）、telemetry schema（§3）、metrics taxonomy（§4）、tracing span boundary（§5）、observation-to-health mapping（§6）、alerting tier 映射（§7）、DR playbook index（§8）。
- **Defers To**：health 状态全集与状态迁移规则归 `04_repository#repo-health-and-repair`（本章只做观测→health 的**映射**，§6）；错误码定义归 `13_i18n#i18n-error-code-catalog`（§7 只映射 tier）；latency/RSS 目标归 `21_perf_budget`；profile 归 `17_tech_stack#performance-profiles-and-feature-matrix`；运维观测 endpoint 归 `18_release#runtime-observability`；DR/恢复步骤归 `06_backup`（§8 DR Playbook Index）。
- **边界**：本章 **MUST NOT** 定义 health 状态、错误码、budget 数值或新增调用层；只承载观测/映射/边界声明。

## 2. SLO / SLI Catalog {#slo-sli-catalog}

SLO 目标与其 SLI 度量。Error Budget 耗尽即触发治理动作（降级发布节奏 / 优先修复）。

| SLO | SLI | Target | Error Budget |
|---|---|---|---|
| Edit-ack 可用性 | `ack` 成功率（非 `SYNC_EDIT_REJECTED` 协议错误占比） | ≥ 99.5% / 7d | 0.5% / 7d |
| Edit-ack 延迟 | P99 edit→ack（对照 `21_perf_budget`） | 达标率 ≥ 99% | 1% / 7d |
| Sync 成功率 | sync-transfer apply 成功 / 总尝试 | ≥ 99% / 7d | 1% / 7d |
| Repair 收敛 | repair 后 repo 回到 `Healthy` 占比 | ≥ 95% / 30d | 5% / 30d |
| Cold mount 可用性 | cold mount 成功且 ≤ P99（§ `21_perf_budget`） | ≥ 99% / 7d | 1% / 7d |

SLI 的 latency 阈值唯一引用 `21_perf_budget` §2；本章不复制数值。

## 3. Telemetry Schema {#telemetry-schema}

结构化日志/事件字段标准。每条可观测事件 **MUST** 携带下列规范字段；自然语言 message 仅用于人工排查，不得作为分支判断依据。

| Field | Type | 必填 | 语义 |
|---|---|---|---|
| `ts` | RFC3339 | MUST | 事件时间戳 |
| `level` | enum(`error`/`warn`/`info`/`debug`) | MUST | 级别 |
| `runtime` | string | MUST | 产生事件的 runtime（authority/projection/watcher/repair/sync/auth…） |
| `flow_id` | string | SHOULD | 对应 `20_operations_catalog` 的 operation-flow（如 `flow.doc.edit-confirmed-op`） |
| `repo_scope` | string | SHOULD | 当前 repo scope（脱敏后） |
| `scope_nonce` | int | SHOULD | 当前 `scope_nonce`（写路径事件 MUST） |
| `repo_id` | string | SHOULD | repo 机器身份；repo 相关 degraded / repair / rename 事件 MUST |
| `repo_name` | string | SHOULD | 当前 `RepoNameBinding.repo_name`；仅用于人工识别，不得作为机器身份 |
| `error_code` | string | 条件 | 失败事件 MUST；取值唯一引用 `13_i18n#i18n-error-code-catalog`，本章不定义 |
| `span_id` / `trace_id` | string | SHOULD | 关联 §4 tracing span |

规则：字段名稳定、snake_case；新增字段 SHOULD 复用既有语义而非另造同义字段。

## 4. Metrics Taxonomy {#metrics-taxonomy}

指标命名与类型规则。命名前缀 `deve_<runtime>_<subject>_<unit>`。

| 类型 | 用途 | 命名示例 |
|---|---|---|
| counter | 单调累计（次数/字节） | `deve_sync_transfer_total`、`deve_ledger_append_failed_total` |
| gauge | 瞬时值（在用资源） | `deve_runtime_rss_bytes`、`deve_pending_overlay_entries` |
| histogram | 分布（延迟/大小） | `deve_edit_ack_latency_ms`、`deve_open_doc_latency_ms` |

规则：
- latency histogram 的 budget 对照唯一引用 `21_perf_budget`；本章不定义阈值。
- 维度标签 SHOULD 含 `runtime` 与（写路径）`repo_scope`；高基数标签（如 doc_id）**MUST NOT** 作为标签。

## 5. Tracing Span Boundary {#tracing-span-boundary}

追踪跨度边界，沿 `00_engineering_constitution` 四层调用链。

- 每个 **Flow Coordination**（第三层）入口开一个 **root span**，span 名 = 对应 `20_operations_catalog` 的 `flow_id`。
- Instruction Interface 与 Execution Domain 内部为 child span；不得脱离 root 单独成树。
- span 属性 SHOULD 含 `flow_id` / `repo_scope` / `scope_nonce`；失败 span MUST 记 `error_code`（引用 `13_i18n`）。
- 跨 Web↔CLI 的写确认链（edit→ack）SHOULD 透传 `trace_id`，使一次用户编辑在前后端可关联。

## 6. Observation-to-Health Mapping {#observation-to-health-mapping}

把可观测信号映射到 `04_repository#repo-health-and-repair` §2.4 **已定义**的 health 状态。**状态全集、状态迁移（含 `Degraded* → Quarantined`、`→ Repairing → Healthy`）与准入/禁止规则唯一归该章**；本节只声明「什么观测对应哪个已定义状态」，不新增状态、不定义迁移。

| 观测信号 | 映射到（04 §2.4 已定义状态） |
|---|---|
| structure projection 缺 parent / 断链 / 脏 path cache | `DegradedProjection` |
| Structure Facts authority 引用缺失 / cycle / identity mismatch | `DegradedProjection`（是否升级 `Quarantined` 由 04 repair/quarantine gate 决定） |
| Projection Locator 缺失 / 冲突 | `DegradedLocator` |
| catalog / name drift / duplicate metadata / blank selector | `DegradedCatalog` |
| durable projection fault pending（writeback / workspace realign / rebuild interrupted） | `DegradedProjection` 或 `DegradedLocator`，按 fault kind 映射 |
| 全部校验通过、authority 与 projection 一致 | `Healthy` |

迁移条件（如何从 `Degraded*` 进入 `Repairing` 或 `Quarantined`）不在本章定义，唯一见 `04_repository#repo-health-and-repair` §4.3。

### 6.1 DurableProjectionFault Boundary

`DurableProjectionFault` 是 host-local recovery journal，用于记录“authority 已提交，但 projection/workspace 物理副作用尚未完成或完成状态未知”的可恢复故障。它的目标是让重启后的 repair runtime 能精确知道要重试什么，而不是从路径名、repo name 或 URL 猜测身份。

它 **不是** ledger authority，不能新增、撤销或改写业务事实；所有重放动作都必须先重新验证 `RepoId`、当前 `RepoNameBinding` 与 `.notegit` identity marker。

最小字段：

```text
DurableProjectionFault = {
  repo_id,
  repo_name_at_fault,
  name_epoch,
  fault_kind,
  target_path,
  source_path?,
  ledger_seq_or_head,
  first_seen_at,
  last_error,
  retry_count,
  status,
}
```

要求：

- `ProjectionWritebackFailed`、`WorkspaceRealignFailed`、`ProjectionRebuildInterrupted` 这类 ledger-committed 后的物理故障 **SHOULD** 写入 durable fault journal。
- 已实现 durable fault journal 的运行时，进程启动时必须先加载 durable fault journal，再执行 scan/materialize；两者不一致时以 `RepoId` admission 与 ledger authority 为准，保持 fail-closed。
- repair 成功后必须把对应 fault 标记为 resolved 或删除；不得仅清内存 degraded gate。
- 如果当前实现暂未持久化该 journal，启动 scan/materialize 必须能重新发现 drift，并且不得把未知完成状态暴露为 `Healthy`。

## 7. Alerting Tier {#alerting-tier}

错误码 → 告警等级映射。错误码与 HTTP 状态定义唯一归 `13_i18n#i18n-error-code-catalog`；本节只映射 tier，不重定义码或状态。

| Tier | 触发类别 | 示例码（13_i18n） | 动作 |
|---|---|---|---|
| `T1` 紧急 | 5xx；或显式列入的数据完整性/解密失败码 | `STORAGE_PERSIST_FAILED`、`STORAGE_DB_LOCKED`、`SYNC_DECRYPT_FAILED`（数据完整性，显式纳入） | 立即 page + 阻断发布 |
| `T2` 警告 | 409 冲突 / 作用域失效 | `SC_REPO_NOT_SELECTED`、`SC_STALE_SCOPE`、`SC_CONFLICT_TARGET_MISSING`、`SYNC_REPO_UNBOUND`、`SYNC_VERSION_MISMATCH` | 工单 + 观察 Error Budget |
| `T3` 提示 | 4xx 客户端可纠正（含 not-found） | `AUTH_INVALID_PASSWORD`、`DOC_NOT_FOUND`、`SC_DOC_NOT_FOUND`、`SC_COMMIT_NOT_FOUND` | 仅记录，不告警 |

某具体码的 HTTP 状态以 `13_i18n#i18n-error-code-catalog` 为准；本表按状态类归 tier，`SYNC_DECRYPT_FAILED` 为显式例外。**health 信号**（非错误码，来源 `04_repository#repo-health-and-repair`）单独映射：`Quarantined` → `T1`，其余 `Degraded*` → `T2`。

## 8. DR Playbook Index {#dr-playbook-index}

灾难恢复手册索引。备份/恢复/导出的权威合同归 `06_backup`：

- 备份展开与 locator：`06_backup#backup-locator-contract`。
- 恢复候选与还原：`06_backup#backup-restore-candidate-contract`。
- repo 级 degraded/quarantine 后的恢复路径：`04_repository#repo-health-and-repair`。

本章不复制恢复步骤，只索引权威章节。

## 9. Related Configuration (本章相关配置)

- 运维观测 endpoint（健康/就绪探针、debug 开关）：归 `18_release#runtime-observability`。
- 日志级别 / 采样率配置：定义归 `15_settings`；本章只引用其对 telemetry 输出的影响。
