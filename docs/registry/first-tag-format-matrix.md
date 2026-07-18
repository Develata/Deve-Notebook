<!-- Generated: 2026-07-07 | Updated: 2026-07-18 -->

# First-Tag Format Matrix

This registry is the approved first-tag target index. It does not introduce new
engineering rules; authoritative behavior remains in `docs/plan/`.

## Matrix

| Surface | Authority | Approved first-tag target | Current implementation state | Code owner | Failure / reset / repair posture |
|---|---|---|---|---|---|
| Ledger entry payload | `docs/plan/03_storage/authority.md#ledger-entry-format-contract` | `LEDGER_ENTRY_FORMAT_VERSION = 3`; magic `DEVELDG3`; project-owned postcard payload | 已对齐：代码为 v3 / `DEVELDG3`。 | `crates/core/src/models/ledger_decode.rs` | Missing magic, missing version, old codec payload, or unsupported version fail closed. Explicit offline `--allow-legacy-v2` export may read `DEVELDG2` without admitting it to normal runtime. |
| Redb repo schema gate | `docs/plan/03_storage/authority.md#redb-schema-version-contract`; `docs/plan/03_storage/authority.md#remote-import-workflow-tables`; `docs/plan/03_storage/authority.md#projection-fault-recovery-table` | `REDB_SCHEMA_VERSION = 4`，local-authority profile 必须包含 Remote Import session/runtime tables 与 repo-local `PROJECTION_FAULTS`；v3 或缺 required table 的开发 DB 直接 fail closed，不自动迁移、不保留 adapter | 已对齐：v4、UUID-first filename、Remote Import workflow tables、required Projection Fault table、shadow exclusion、缺表 fail-closed 与 B4 Pending rematerialization orchestration 均已落地。 | `crates/core/src/ledger/schema.rs`; `crates/core/src/remote_import/`; `crates/core/src/projection_fault/` | Missing/mismatched top-level schema version 或缺任一 local required table 都在 normal repo/query entry 前 fail closed。Remote shadow 不创建 workflow/fault tables。旧开发数据只能由旧 HEAD 导出后重建；`--allow-legacy-v2` 仅是 Ledger 离线救援，不是 Redb v3/v4 adapter。 |
| WebSocket binary protocol | `docs/plan/07_network.md#server-ws-runtime`; `docs/plan/07_network.md#remote-import-wire-contract` | `WS_PROTOCOL_VERSION = 3;`; `MIN_SUPPORTED_WS_PROTOCOL_VERSION = 3;`; magic `DEVEWSF4`; nested typed Remote Import messages；删除无版本 JSON/legacy fallback | 已对齐：代码为 F4/v3 lockstep；主 `/ws` 无 legacy/unversioned JSON fallback，显式 debug JSON 也必须携带 v3 envelope。 | `crates/core/src/protocol/frame.rs`; `crates/core/src/protocol/remote_import.rs`; `apps/cli/src/server/ws/route/remote_import.rs` | Missing/wrong binary magic or unsupported protocol version fails closed with structured protocol error。F4/v1/v2 无 adapter；保留显式带版本 debug JSON。 |
| Projection Locator | `docs/plan/03_storage/projection.md#projection-locator-contract` | Host-local locator file `ledger/.host/projection-locators.toml`; derived workspace root `<projection_base>/<safe_repo_name>--<repo_id>/`; no ledger or sync payload version | 已对齐。 | `crates/core/src/ledger/manager/projection_locator.rs` | Locator missing, non-canonical, conflicting, nested, or identity-marker mismatch enters `DegradedLocator` and must use explicit locator repair/reset/rebuild flow. |
| Remote Projection transport / Remote Import | `docs/plan/06_backup.md#remote-projection-transport-contract`; `docs/plan/06_backup.md#remote-import-session-contract` | `projection-remote <provider> push` + immutable source acquisition → Remote Import session/review/sealed Ledger Apply；host-local profile binding，不暴露 secret/path/digest | 部分对齐：B4 backend/CLI/product cutover、Mounted gate、post-commit writeback、Local CLI proxy 与旧 pull 删除已完成；B5 typed client 与 B6 fresh release receipts 仍阻塞 tag。 | `crates/core/src/remote_projection/`; `crates/core/src/remote_import/`; `crates/core/src/ledger/manager/prepared_change_batch/`; `apps/cli/src/remote_projection_transport/`; `apps/cli/src/remote_import_runtime.rs`; `apps/cli/src/commands/projection_remote.rs`; `apps/cli/src/commands/remote_import/`; `apps/cli/src/server/handlers/remote_import/` | Locator/profile/provider/path/budget failure must precede authority effects。Prepare 只封存 immutable blobs；任意 blocker 禁用 whole-session Apply；authority transaction 持久化 Pending receipt，post-commit writeback 后单调收敛为 Written/Degraded，不回滚或重复写 Ledger。 |

## Release Gate Binding

`cargo run -p deve_baseline -- release` pins this registry, the plan constants,
and the code constants above. A format change must update `docs/plan/` first,
then this registry and the matching baseline spec before code can claim first-tag
readiness.

Implementation status after B4: Ledger payload v3、Redb schema v4、Projection Locator、
F4/v3 lockstep、Remote Import backend/CLI/product wire、Mounted admission 与 post-commit
writeback 已对齐；旧 pull/continuation/External Changes bridge 已删除。The matrix continues
to block tag readiness until B5 delivers the independent typed client and B6 replaces
the remaining gaps with fresh producer evidence.
