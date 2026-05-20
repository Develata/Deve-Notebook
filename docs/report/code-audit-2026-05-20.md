<!-- Generated: 2026-05-20 -->

# Deve-Notebook 代码审计报告 — `crates/core` + `apps/*`

- **审计范围**：`crates/core/src`（382 个 `.rs`）、`apps/cli/src`（513）、`apps/web/src`（591）、`apps/desktop/src`（31）、`apps/mobile/src`（21），合计 1 538 个源文件 + 部分根级辅助
- **审计层次**：战略级，结论先行
- **配套报告**：[plan-audit-2026-05-20.md](./plan-audit-2026-05-20.md)、[docs-audit-2026-05-20.md](./docs-audit-2026-05-20.md)
- **辅助证据**：`scripts/plan-coverage.sh` 实测、`grep` 反向覆盖矩阵、Cargo 工程结构、`git ls-files` 抽样

---

## 1. 总体结论

代码层是这一轮审计中**完成度最高**的一层。`scripts/plan-coverage.sh` 报告：

```
== Check 1: single-file size fuse ==
fuse violations: 0, soft warnings: 27
== Check 2: plan_ref annotations ==
modules with plan_ref: 1246
modules without plan_ref (soft): 0
modules without plan_ref (exempt): 340
dangling plan_ref (blocking): 0
== Check 3: i18n facade leak ==
i18n leaks (blocking): 0
== Check 4: acceptance case bindings ==
== Check 5: feature operation path drift ==
== Summary ==
blocking violations: 0
soft warnings: 27
```

也就是说：**没有一个非豁免源文件缺失 `plan_ref` 注解；没有一个 `plan_ref` 指向不存在的 plan anchor；没有一个文件超过 500 行硬熔断；没有一处 i18n 硬编码泄漏。** 这种正向覆盖在同类开源项目中极少见。

不过仍有三类问题需要在「日常工作不出大错」的护栏之外补强：

1. **plan-code 双射的「方向」不对称**——AGENTS.md anchor 表是「代码可以引用什么」的白名单，但代码可以「使用未登记的 anchor」（8 处），AGENTS.md 表也含「无任何代码引用」的陈尸 anchor（4 处）。脚本只查 dangling，不查这两个方向的漂移。
2. **plan §Runtime Skeleton Registry 的 27 个 runtime 名称在代码中只对应 8 个真实模块**，其余 19 个或散落或未启动。Authority Core 的 4 个目标 runtime 已经体现（authority_storage / projection_persistence / projection_repair / repair），但 web/UI/transport/document 这一组（browser_peer / browser_document / pending_overlay / write_confirmation / transport / relay_proxy / document_runtime / render_projection / widget_bridge / outline_projection）**完全没有按名落地**。
3. **Desktop native track 接近熔断线**——27 个 soft warning 里 `apps/desktop/src/` 占 7 个，其中最长 492 行（`process_runtime_test.rs`），与 500 行硬熔断只差 8 行。

总体判断：**代码主线非常干净，但 plan-code 「双射」目前只在「向前」（plan_ref 不指空）这一个方向上完全成立；「向后」（每个 plan 概念都能在代码中找到清晰承载位置）还差关键的几步。**

---

## 2. 强项（保持现状）

| 项 | 评价 / 数字 |
|---|---|
| 文件大小护栏 | 0 fuse violation；27 soft warning，最长 492 行；规模 1 538 源文件 |
| `plan_ref` 覆盖 | 1 246 文件带注解，340 文件 exempt，0 文件非豁免缺失；0 dangling |
| i18n 硬编码 | 0 leak；`apps/web/src/components/` 与 `apps/web/src/editor/` 都走 `t::*` facade |
| 路径规范化 | `deve_core::utils::path::to_forward_slash` 调用 162 处；原始 `replace('\\','/')` 只在 5 处出现，都属于 filename sanitization 或测试 helper |
| 协议版本绑定 | `WS_PROTOCOL_VERSION = 9`、`MIN_SUPPORTED_WS_PROTOCOL_VERSION = 9`、`WS_FRAME_MAGIC = b"DEVEWSF3"` 与 plan §05 §4.2 完全一致 |
| Scope nonce 纪律 | `scope_nonce` 2 534 处，`switch_nonce` 460 处，`client_op_id` 212 处——比 plan §05、§16 要求的「必须携带」更深入分布 |
| Watcher 不写 ledger | 仅 `dispatch_test_support.rs`（测试辅助）出现 `append_generated_op_in_local_repo`；生产 watcher 路径不直写 authority |
| Pending overlay ⊥ pending_fs_ops | 0 处交叉引用，两表完全分离 |
| AUTH guards | `Wildcard CORS origin is forbidden` 在 `setup.rs` 强制；`AUTH_ALLOW_ANONYMOUS_LOCALHOST` 检查 `is_loopback()`；`identity_key_permissions_fail_closed_for_non_file` 测试在位 |
| Native packaging gate | `apps/desktop/Cargo.toml` 与 `apps/mobile/Cargo.toml` 都显式 `tauri = { optional = true }` + `native-packaging = ["dep:tauri", "dep:tauri-build", …]`，默认 feature set 不引入 Tauri |
| AGENTS.md 嵌套 | 89 份 `AGENTS.md` 覆盖到每一级关键子目录（apps/cli/src/server/handlers/source_control/errors/ 等都有），层级指导极其细致 |
| TODO/FIXME 卫生 | 整个 src/ 树仅 1 处「TODO/FIXME/HACK/XXX」匹配，且是 `"Peer-XXX"` 占位符的误命中——0 真实 TODO 债务 |
| 安全/敏感字面量 | 未发现明文密钥；`AUTH_*` 全部读环境变量；密码哈希走 Argon2 |
| 命令面默认 off | `agent_bridge/policy.rs` 默认关闭，需要 enabled + trusted + 绝对 `AGENT_CLI_PATH` 三条件全满足 |

---

## 3. 关键 Finding（按优先级）

### P0 — 影响后续工程的结构性问题

#### F-C-1. 27 个 plan-declared runtime 中只有 8 个在代码里按名存在

**事实**：对 `deve-note plan.md §Runtime Skeleton Registry` 列出的 runtime 名称做按名 grep：

| Runtime | 模块按名存在 | 承载位置 |
|---|:---:|---|
| `authority_storage_runtime` | ✓ | `crates/core/src/ledger/manager/authority_storage_runtime.rs` |
| `projection_persistence_runtime` | ✓ | `crates/core/src/sync/projection_persistence_runtime.rs` |
| `projection_repair_runtime` | ✓ | `crates/core/src/sync/projection_repair_runtime.rs` |
| `repair_runtime` | ✓ | `crates/core/src/ledger/manager/repair_runtime.rs` |
| `repo_catalog_runtime` | ✓ | `crates/core/src/ledger/manager/repo_catalog_runtime.rs`（+ local/shadow 变体） |
| `repo_scope_runtime` | ✓ | `crates/core/src/ledger/manager/repo_scope_runtime.rs`（+ lookup/selector 变体） |
| `source_control_runtime` | ✓ | `crates/core/src/ledger/manager/source_control_runtime.rs`（+ read/write/scoped 变体） |
| `watcher_runtime` | ✗ | 存在功能模块 `crates/core/src/sync/watcher/`，但不按 `watcher_runtime` 命名 |
| `diff_session_runtime` | ✗ | 没有按名模块 |
| `merge_runtime` | ✗ | 没有按名模块（`crates/core/src/ledger/merge/` 存在但未冠 runtime 名） |
| `session_runtime` | ✗ | 没有按名模块 |
| `auth_gateway` | ✗ | 没有按名模块（功能散在 `apps/cli/src/server/auth/`） |
| `browser_auth_runtime` | ✗ | 没有按名模块（`apps/web/src/app/auth_monitor.rs` 等） |
| `transport_runtime` | ✗ | 没有按名模块 |
| `repo_scope_sync_runtime` | ✗ | 部分实现散在 `crates/core/src/sync/repo_scoped*` |
| `browser_peer_runtime` | ✗ | 没有按名模块 |
| `relay_proxy_runtime` | ✗ | 没有按名模块 |
| `browser_document_runtime` | ✗ | 没有按名模块（功能散在 `apps/web/src/editor/sync/`） |
| `pending_overlay_runtime` | ✗ | 名字仅在 4 处注释/测试出现，无独立模块 |
| `write_confirmation_runtime` | ✗ | 没有按名模块 |
| `document_runtime` | ✗ | 没有按名模块 |
| `render_projection_runtime` | ✗ | 没有按名模块 |
| `widget_bridge_runtime` | ✗ | 没有按名模块 |
| `outline_projection_runtime` | ✗ | 没有按名模块 |
| `ui_shell` / `application_control` / `feature_runtime` | ✗ | 三层名仅作为 plan 词汇，未在代码冠名 |

按名落地比例 **8 / 27 ≈ 30%**，且全部集中在 `crates/core` 的 authority/ledger 层。**web/UI/transport/document 这一大块的 runtime 收敛尚未开始按名拆分**。

**风险**：
1. plan §Runtime Skeleton Registry 在 `deve-note plan.md` 是「权威登记」，但代码侧没有强制呼应。如果未来 plan 改名（比如 `pending_overlay_runtime` → `local_edit_runtime`），代码不会感知。
2. 这与 plan-audit 的 F-P0-1（缺少 runtime 收敛状态字段）和 docs-audit 的 F-D-3（tasks/18 §7 个责任带与 plan §27 个 runtime 不映射）合在一起，揭示一条**未完工的主线**：plan 把目标拆得很细，tasks 把它分组到 7 个带，代码只在 authority 层兑现了一小半。

**建议**：
- 立刻在 `deve-note plan.md §Runtime Skeleton Registry` 表里加 `current_module_path` 列，对每个 runtime 写「未启动 / 部分实现于 `path/to.rs` / 已独立模块」。
- 任何 `apps/web/src/hooks/use_core/effects/message_*` 重构合并到独立 runtime crate 时，必须同步更新该列。
- `scripts/plan-coverage.sh` 增加正向校验：遍历 Registry 中 `current_module_path != 未启动` 的条目，检查路径存在；不存在则 warning。

#### F-C-2. `docs/plan/AGENTS.md` 锚点表与代码使用的 `plan_ref` 集合双向漂移

**事实**：
- AGENTS.md 锚点表登记 **51** 个锚点
- 代码中 `//! plan_ref:` 实际使用 **55** 个唯一锚点
- 集合差异：

**已登记但代码未引用的「孤儿」（4 个）**：
```
08_ui_design#native-post-gate-common-contract
08_ui_design_01_web#single-binary-distribution
08_ui_design_03_mobile#mobile-android-shell-package-execution-gate
08_ui_design_03_mobile#mobile-ios-shell-package-execution-gate
```

**代码引用但未登记的「漂移」（8 个）**：
```
04_storage#git-ecosystem-coexistence
07_diff_logic#git-mirror-lifecycle
09_auth#audit
09_auth#cors
09_auth#key-and-file-permissions
09_auth#localhost-dev-policy
14_tech_stack#graph-visualization
14_tech_stack#native-packaging-dependency-gate
```

后者尤其值得关注：`07_diff_logic#git-mirror-lifecycle` 是 plan-audit 列出的「唯一权威」之一，被代码引用但**没有出现在 AGENTS.md 表里**——意味着该表的「这些是可以引用的稳定 anchor」承诺与现实脱钩。

**风险**：
1. `scripts/plan-coverage.sh` 只校验 `plan_ref` → plan anchor 的可达性（左侧 → 右侧），不校验 AGENTS.md 表 ↔ 代码使用集（左右镜像）。
2. 4 个 orphan anchor 的实际语义可能已被代码移到别处（如 `mobile-android-shell-package-execution-gate` 被代码改名引用），但表没跟上。
3. 8 个 drift anchor 表明开发者已经事实上「先在 plan 加 `{#anchor}` 再在代码 plan_ref」，但忘了同步 AGENTS.md 表——这正是 plan-audit §F-P2-4 预测的情况。

**建议**：
- `scripts/plan-coverage.sh` 增加 Check 6：扫描所有 plan 章节 `{#anchor}` 标记构建权威集合 P；扫描所有代码 `plan_ref` 构建集合 C；扫描 AGENTS.md 表构建集合 A。要求 `A ⊆ P` 且 `C ⊆ P`，并把 `A − C`（orphan）与 `C − A`（drift）输出为 warning。
- 短期手工修复：4 个 orphan 删除或保留并标注「reserved for future」；8 个 drift 添加到 AGENTS.md 表。

#### F-C-3. plan §`Primary Code Areas` 字段已出现至少一处失效引用

**事实**：`docs/plan/09_auth.md` 的 Metadata `Primary Code Areas` 引用：
```
apps/web/src/api/auth_probe.rs        ← 存在
apps/web/src/app_auth_monitor.rs      ← 不存在
```

实际位置是 `apps/web/src/app/auth_monitor.rs`（多了一级 `app/`）。这是 plan-audit §F-P0-3 警告的情况首次被代码侧抽样命中。

**风险**：`Primary Code Areas` 在代码层无任何脚本扫描，错误会持续存在。
- 此次只抽样了 4 章（06 / 07 / 09 / 16），就发现 1 处不命中。
- 推断：19 章 × 平均 3-5 个 glob ≈ 70-90 个 path，若错误率 5%，有 4-5 处类似问题。

**建议**（与 plan-audit F-P0-3 一致）：
- `scripts/plan-coverage.sh` 增加 Check 7：扫描所有 plan 章节 Metadata 中 `Primary Code Areas` 的 glob（包含 `*` 与 `?`），调用 `find` / `glob` 校验每个至少匹配 1 个真实文件；不匹配则 warning。
- 立即修复 `09_auth.md`：把 `apps/web/src/app_auth_monitor.rs` 改为 `apps/web/src/app/auth_monitor.rs`。

---

### P1 — 一致性与冗余

#### F-C-4. Desktop native track 接近 500 行硬熔断

**事实**：27 个 soft warning（>250 行）中 `apps/desktop/src/` 占 7 个，且占据前 4 名：

```
492 apps/desktop/src/process_runtime_test.rs
469 apps/desktop/src/process_runtime.rs
464 apps/desktop/src/service_bootstrap_test.rs
412 apps/desktop/src/service_bootstrap.rs
310 apps/desktop/src/tauri_entry.rs
303 apps/desktop/src/service_entrypoint.rs
251 apps/desktop/src/tauri_bootstrap.rs
```

最长的 492 行距离 500 行硬熔断只差 8 行；这层是新近添加的 native packaging gate 代码，仍在迭代中。

**风险**：
1. 下一批 Desktop native session / installer / local service 工作很可能会越线，触发 CI 阻塞。
2. plan-audit §F-P0-2 已经指出 Desktop/Mobile 子章承担了过多 governance 决策——代码层的镜像表现是「单文件越大越接近熔断」。

**建议**：
- 主动拆 `apps/desktop/src/process_runtime.rs`（469 行）与 `apps/desktop/src/service_bootstrap.rs`（412 行），按职责（start / stop / health-probe / session-handoff / state-machine）分子模块。
- `_test.rs` 文件按 `soft-size-audit-2026-04-27.md` 政策允许超过软阈值，但 492 行的 `process_runtime_test.rs` 仍应评估是否能拆成多场景文件。
- 短期：在 `Cargo.toml` workspace 的 lint 配置里把 `apps/desktop/src/process_runtime.rs` 加入 review checklist，避免该文件继续增长。

#### F-C-5. 编辑 buffer 三元名在代码里使用第三套术语，与 plan §16 / §03 都不同

**事实**：
- plan `16_web_thin_client_ledger.md §2.1`：`L_confirmed` / `O_session` / `V_web`
- plan `03_rendering.md §2.1`：`L_confirmed` / `O_pending` / `V_editor`
- 代码 `apps/web/src/hooks/use_core/pending/ops.rs`：`PendingLocalEdits` / `PendingLocalEdit` / `PendingLocalEditInput`
- 代码 `apps/web/src/editor/sync/history_replay.rs`：函数 `replay_pending_overlay`

三处共指同一个对象（浏览器会话未确认编辑集合），但用了至少 4 个名字：`O_session`、`O_pending`、`pending_overlay`、`PendingLocalEdits`。

这印证了 plan-audit §F-P1-2 的预测：plan 跨章不同名，代码再产生第三种叫法。

**建议**：
- 在 `01_terminology.md` 显式定义「权威英文名 = `PendingLocalEdits`，plan 同义词 = `O_session` / `O_pending` / `pending_overlay`」。
- 或反过来：把代码里命名收敛到 `O_session`（更短）+ `PendingLocalEdits` 作为 type alias。
- 让 `plan-coverage.sh` 在扫描 `apps/web/src/hooks/use_core/pending` 时增加 anchor `16_web_thin_client_ledger#pending-overlay-types`（如果不存在则要求加锚点）。

#### F-C-6. 生产代码中存在 72 处 `unwrap()` 与 74 处 `println!/eprintln!`

**事实**：
- `unwrap()` 在非测试 src 中出现 72 处。
- `println! / eprintln!` 在非测试 src 中出现 74 处，其中：
  - CLI 命令（`apps/cli/src/commands/{dump,config,...}.rs`）是合理 stdout UX
  - `crates/core/src/plugin/loader.rs` 与 `crates/core/src/security/keypair.rs` 使用 `eprintln!`，按全局 AGENTS.md 应改用 `tracing::warn!` / `tracing::error!`

**风险**：
1. `unwrap()` 在 Rust 生产代码中通常是「无证 expect」——72 处可能多数是 const 类的安全 unwrap，但缺少分类即缺少审计。
2. `tracing` 是 plan §14_tech_stack 选定的 Logs 技术，但仍有 plugin/security 模块用 `eprintln!` 绕过结构化日志。

**建议**：
- 引入一个一次性扫描批次：用 `clippy::unwrap_used` 配置默认 `warn` 或 `deny`（plan-coverage 也可加这条 Check），人工评估每一处。
- 把 `crates/core/src/plugin/loader.rs` 与 `crates/core/src/security/keypair.rs` 的 `eprintln!` 改为 `tracing::warn!`。

---

### P2 — 局部清理

#### F-C-7. 仓库根仍有少量未登记目录与文件

**事实**：
- `Vault_old/`：在 `.gitignore`，工作树残留，未追踪
- `target_codex/` / `target_codexhvteR7/`：在 `.gitignore`（`/target_codex*`），云端工作目录残留
- `repomix-output.xml`：在 `.gitignore`，未追踪
- `deve-note plan/`（同名目录！与 `deve-note plan.md` 仅一字之差）：内容只有一个 `repomix-output.xml`，工作树残留
- `add_path_comments.ps1`：**已追踪**，39 行，PowerShell 脚本，一次性工具

**结论**：上述 4 项都属于本地或云端 scratch，git 已经忽略；只有 `add_path_comments.ps1` 是真实在版本控制里的、孤立的、未在 `AGENTS.md` 提及的脚本。

**建议**：
- 删除或归档 `add_path_comments.ps1`，或在仓库根 `AGENTS.md` 显式登记其用途。
- 删除工作树本地的 `Vault_old/`、`target_codex/`、`target_codexhvteR7/`、`deve-note plan/` 与 `repomix-output.xml`，避免后续 IDE 索引干扰。

#### F-C-8. `apps/web/src/storage/` facade 完全收口浏览器存储

**事实**：在 `apps/web/src` 中 `window().local_storage() / session_storage()` 直接调用 **0 处**，所有访问都经由 `apps/web/src/storage/prefs.rs`、`storage/identity.rs`、`storage/js_bridge.rs`。`08_ui_design.md §6 Layout Persistence Contract` 与 `13_settings.md §4 Browser UI Preferences` 的约束在代码层是闭合的。

**结论**：无 finding；记录为强项。

#### F-C-9. 大量 AGENTS.md 嵌套（89 份）

**事实**：从仓库根到 `apps/cli/src/server/handlers/source_control/service/AGENTS.md` 七级深处都有 AGENTS.md。极其细致的层级指导。

**风险**：同步维护成本高——每个 AGENTS.md 自身可能漂移。但本审计没有抽样发现单个 AGENTS.md 写错，所以不列为 finding，仅记录。

#### F-C-10. `target/` 目录占用与构建副产物

仓库根可见 `target/`、`target_codex/`、`target_codexhvteR7/` 三类构建目录。前者是标准 Cargo target，后两个是云端工作目录。本地占用应该已经很可观；建议 CI 加 disk-usage 监控，但不在本审计范围内。

---

## 4. 跨 Phase 闭环

下面把三轮审计的关键 finding 串联，给出后续治理批次的推荐顺序：

| 优先级 | Finding（来源） | 单批工作量 | 修复后能解锁 |
|:---:|---|---|---|
| 1 | F-D-1 / F-D-2（report 流水化、next-tasks 自相矛盾） | 半天 | 后续所有 PR 的 review 信噪比 |
| 2 | F-P0-1 / F-D-3 / F-C-1（27 runtime 收敛状态字段） | 1-2 天 | 「现状 vs plan 漂移」可量化、后续重构有锚 |
| 3 | F-C-2（AGENTS.md anchor 表双向校验） | 半天，含脚本改造 | plan_ref 双射机制真正闭合 |
| 4 | F-C-3 + F-P0-3（`Primary Code Areas` 失效引用） | 半天 | plan 章节作为索引可用性 |
| 5 | F-C-4（Desktop native track 接近熔断） | 1 天 | 下一批 Desktop work 不被 fuse 阻塞 |
| 6 | F-P0-2 / F-D-3（平台子章 governance 内容外迁） | 2-3 天 | UI shell 章重新内聚，governance 集中 |
| 7 | F-P1-2 / F-C-5（编辑 buffer 三元名统一） | 0.5 天 | plan ↔ code 词汇对齐 |
| 8 | F-C-6（unwrap/eprintln 审计） | 1 天 | 长期质量基线 |
| 9 | F-C-7（仓库根 stale 清理） | 0.2 天 | 工作树干净 |

预估总工作量：5-7 个工作日，分多批次进入，互不阻塞。

---

## 5. 数字摘要

| 维度 | 数量 |
|---|---:|
| `crates/core/src` `.rs` | 382 |
| `apps/cli/src` `.rs` | 513 |
| `apps/web/src` `.rs` | 591 |
| `apps/desktop/src` `.rs` | 31 |
| `apps/mobile/src` `.rs` | 21 |
| 合计源文件 | 1 538 |
| `plan_ref` 注解模块 | 1 246 |
| 非豁免缺失 `plan_ref` | 0 |
| dangling `plan_ref` | 0 |
| i18n 硬编码泄漏 | 0 |
| 单文件 > 500 行（硬熔断） | 0 |
| 单文件 > 250 行（软警告） | 27 |
| Desktop native 单文件 > 250 行 | 7 |
| 最长单文件 | 492（process_runtime_test.rs） |
| AGENTS.md 嵌套份数 | 89 |
| AGENTS.md 锚点表条目 | 51 |
| 代码中使用的唯一 plan anchor | 55 |
| Orphan anchor（已登记未使用） | 4 |
| Drift anchor（已使用未登记） | 8 |
| Plan-declared runtime 按名落地 | 8 / 27 |
| `to_forward_slash` 调用 | 162 |
| `scope_nonce` 引用 | 2 534 |
| `client_op_id` 引用 | 212 |
| 非测试 `unwrap()` | 72 |
| 非测试 `println!/eprintln!` | 74（多数为合理 CLI UX） |
| 真实 TODO/FIXME/HACK | 0（1 处误命中） |
| `WS_PROTOCOL_VERSION` | 9（与 plan §05 一致） |
| `WS_FRAME_MAGIC` | `DEVEWSF3`（与 plan §05 §4.2 一致） |
| Tauri 依赖 | optional + `native-packaging` feature gated（与 plan §14 §1.4 一致） |
| pending overlay ↔ pending_fs_ops 交叉引用 | 0 |
| Watcher → ledger append（生产路径） | 0 |
| CORS wildcard 默认 | 显式 forbidden + 测试 fail-closed |
| `AUTH_ALLOW_ANONYMOUS_LOCALHOST` | 仅 loopback 时生效 |

---

## 6. 总结

代码层是这次审计中**唯一可以下「健康」结论**的部分：

- 所有 plan-coverage 强制检查通过
- 没有任何熔断违规
- 关键 plan 不变量（pending overlay vs pending_fs_ops、watcher ⊥ ledger 写、协议版本、scope nonce、CORS、native packaging gate）都在代码里得到验证
- TODO 卫生几乎完美

但下一步的瓶颈不在代码本身，而在 **plan ↔ code 的双射方向不完整**：

- 「代码不指空 anchor」已经成立；
- 「代码概念都能在 plan 找到锚点」也大致成立；
- 但「plan 概念都能在代码找到承载位置」**只完成了 ~30%**——这就是 F-P0-1 / F-D-3 / F-C-1 在三层各看到的同一件事。

**推荐下一步行动**：把「plan §Runtime Skeleton Registry 加 status/path 列」作为单批 PR 提出，它一次性把三个 audit phase 提到的最高优先级 finding 全部解锁，且修改面只在 plan 文档与 `scripts/plan-coverage.sh`，不动代码主线。
