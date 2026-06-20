# Handoff — codex/native-peer-modes (2026-06-20)

本文档交接两件事:**(A)** ledger/redb 存储版本化恢复 + 验证,**(B)** 16 个 pre-existing 测试失败的全部修复。所有结论均经本轮真实跑测验证,非自述。

工作树状态:**clean**。分支:`codex/native-peer-modes`。

---

## A. 存储版本化(#A/#B/#C)— 已恢复并验证

> 背景:codex 提交的 `6e480b13 Version ledger and redb storage formats` 一度在 detached-HEAD 操作中被挤出分支(实为 HEAD 游离,分支 ref 未丢)。已用**非破坏 cherry-pick** 恢复,无 `reset --hard`、无历史改写,codex 原始提交完整保留。

| 项 | 落地 | 验证 |
|---|---|---|
| **#A** ledger entry 显式信封 `DEVELDG1` + `LEDGER_ENTRY_FORMAT_VERSION=1` + bincode;删除全部 `LegacyLedgerEntryV*` 形状探测;缺 magic / 版本不符 fail-closed | `crates/core/src/models/ledger_decode.rs` | roundtrip + 旧无版本数据 fail-closed + 未知版本 fail-closed 三测试通过 |
| **#C** redb 顶层 schema gate `REPO_METADATA[1]=REDB_SCHEMA_VERSION=1`,local + shadow 两条开库路径都在**开库时**校验、fail-closed | `ledger/schema.rs` / `manager/repo_info.rs` / `shadow/management.rs` | `schema_version_test.rs` 3 测试(written / missing fail-closed / mismatch fail-closed) |
| **#B** 维持 lockstep:`MIN_SUPPORTED_WS_PROTOCOL_VERSION == WS_PROTOCOL_VERSION`,plan 写明"只下调常量不算兼容实现" | 文档 `07_network.md` | — |

文档对齐链完整:`03_storage/authority §4.1.1`(ledger 格式契约)+ `§4.3.1`(redb schema gate)+ `07_network`(lockstep MUST)+ `18_release`(pre-1.0 无版本数据可 fail-closed 重置;stable 基线含两个 v1)。

**发布策略(需记住)**:pre-1.0 旧开发期产生的无版本 ledger entry / 无 schema gate `.redb` **可要求 reset/repair/migration**,不进入 stable 兼容承诺。

---

## B. 16 个 pre-existing 测试失败 — 全部修复

这 16 个失败**早于**存储工作,根因是本分支两个已有硬化提交,与 #A/#B/#C 无关。

### 我的三个提交

| 提交 | 性质 | 修复数 | 改动文件 |
|---|---|---|---|
| `50f18059` | 纯测试 | 14 | `tests/edit/edit_state_test_support.rs`、`tests/rebuild_projection_test.rs`、`tests/startup_scan_projection_skip_test.rs` |
| `fa586571` | 纯测试 | 1 | `tests/sync/sync_scope_cleanup_test.rs` |
| `6069541c` | **产品** | 1 | `crates/core/src/source_control/staging/target.rs` |

### 根因与处理

1. **`72a3f069` bind projection workspaces to repo identity**(14 个)
   给 projection rebuild/materialize **及 document-edit 热路径**加了 `.notegit/identity.toml` 身份 gate(+ 工作区根 `canonicalize`)。失败测试用裸 `RepoManager::init` + `set_projection_base` 后直接写工作区内容 / 驱动 edit,等于构造"有外部内容但无 marker"的非法工作区,触发 gate。
   **修复**:setup 阶段补 `ensure_local_repo_workspace_identity(...)`,对齐生产 init 顺序。**无产品改动。**
   *注:失败信息里的 `\\?\C:\...` 只是 Windows 扩展长路径显示,gate 逻辑平台无关(Linux 上同样会挂)。*

2. **`cf7ad475` harden writer registration session gate**(1 个)
   writer 注册第一关拒绝非 browser 会话。失败的 cleanup 测试用了非 browser 会话,卡在该关、到不了它要断言的 remote-unbound binding 清理。
   **修复**:测试改用 browser 会话。非 browser 会话**不被** writer 注册改动 binding 是**有意行为**(可能是合法 peer 会话),cf7ad475 自带单测已固化该契约。**无产品改动。**

3. **真实契约冲突 — unstage 目标解析**(1 个,经 codex review 裁决)
   两个测试相互矛盾:
   - 单元 `staging::target::tests::staged_doc_target_prefers_live_successor_over_exact_deleted_doc_path` → rename pair 取 **live successor**
   - 集成 `source_control_side_index_test::target_resolution_keeps_exact_deleted_path` → 取 **exact deleted**

   **根因**:stage 与 unstage 的目标解析本该**分治**,却共用同一 resolver。`take_staged_for_target`(唯一调用方 = unstage,`source_control_write_runtime.rs:144`)沿用共享 resolver,对 rename pair(`old=Deleted` / `new=Added renamed_from old`)偏好 live successor,导致显式 unstage 删除侧 `old` 误吃 `new`。叠加用户级 unstage 会展开 rename pair,先处理 old 会先拿走 new → **非原子半迁移风险**。
   **修复(scoped,per codex 裁决)**:stage/read 路径与 `select_entry_for_doc` **保持** live-successor;**只**让 `take_staged_for_target` 对**带 doc_id 的目标**精确路径优先;path-only 目标保留歧义 fail-closed。**两个原本矛盾的测试现在同时通过,均无需改动。**

### 验证(本轮真实跑过)

- `cargo test -p deve_core --lib` → **587 passed / 0 failed**
- `cargo test -p deve_cli --lib source_control` → **169 passed / 0 failed**
- 11 个 source_control/discard 集成测试文件 → 全绿
- 16 个原始失败面(rebuild_projection 5、source_control_side_index 5、startup_scan 1、cli edit+sync 21)→ 全绿

---

## C. 给 codex 的待办 / 注意事项

1. **rustfmt 版本噪声(建议统一)**:`crates/core/tests/common/mod.rs`、`crates/core/src/source_control/staging/target/tests.rs` 反复出现 import 重排 / `assert!` 格式的 cosmetic diff,是本地 rustfmt 版本与提交时不一致所致。已每次剔除,但建议固定仓库 rustfmt 版本(`rust-toolchain.toml` 或 CI 对齐)消除噪声。

2. **release-native.yml 与 baseline-CLI 化的关系**:已确认**无需改动**。`deve_baseline` 只镜像 storage-repo / network 两类仓库文件检查;`18_release.md` 明确 Rust mirror 与 shell script 并存,打包脚本(`build-web-dist-ci.sh`、`check-*-package-build.sh` 等)仍在,workflow 调用不受影响。

3. **协作约定(避免再次工作树碰撞)**:本轮一度因并发 git 操作 + detached-HEAD 出现 6e480b13 "疑似丢失"。建议并发期间:谁在主树改动时,另一方在**独立 git worktree** 里只读诊断(本轮即如此,零碰撞);git reset/checkout 类操作前先确认对方未在改主树。
