# P2P / 中继底层不变量审计（深层补审）

- `Date`: `2026-05-30`
- `Scope`: 上一轮语义对齐审计（`plan-code-semantic-alignment-audit-2026-05-30.md`）止于"catalog flow 层"；本报告专审 USER 反复强调的 **P2P/中继核心不变量**——比 catalog flow 更底层。
- `Authority`: `07_network` §7.2 Envelope、§10.2 Trust Boundary、§10.3 Malicious Peers、§10.4 Remote Shadow Apply Atomicity、§10.5 Indirect Sync and Attribution (`#relay-proxy-attribution-contract`)。
- `结论`: **逐条 ALIGNED；零偏差。** 且为带专用 core 模块 + 加密来源证明 + 对抗式测试的**典范级**实现。

## 0. 验证方法

Claude 直接用 codegraph + 读源码/测试取证（未委派）。**诚实边界**：本审读取实现与测试作为证据，**未在本会话运行测试**（视已提交测试为通过）；覆盖 §7.2/§10.2-10.5 核心，未逐条覆盖 §8 重连 / §10.1 snapshot fallback 细节 / §11 forbidden patterns 全集。

## 1. 关键代码坐标

- 协议消息（`crates/core/src/protocol/server.rs:27-28`）：`SyncPush { source_peer_id, repo_id, header: SyncPushHeader, scope_nonce, branch, encrypted_payload }`、`SyncPushSnapshot { source_peer_id, …, source_proof: Option<SyncSourceProof>, payload }`——**显式 source_peer_id + 加密 body**。
- 形式化路由契约：`crates/core/src/protocol/relay_proxy.rs:62 plan_relay_proxy_route`（`plan_ref: relay-proxy-attribution-contract`）。
- 来源证明：`crates/core/src/protocol/sync_push_header.rs` `sign_source` / `validate_source_proof` / `SyncSourceProof::verify`。
- 入站 handler：`apps/cli/src/server/handlers/sync/transfer.rs:125 handle_push`（`plan_ref: 07_network#relay-proxy-attribution-contract`）。
- shadow 落盘：`crates/core/src/ledger/shadow_transfer.rs:88 apply_remote_payloads_internal` → `run_on_shadow_repo(peer_id, repo_id, |db| begin_write → append_op_to_txn + structure_projection::apply_in_txn → commit)`。
- 对抗测试：`apps/cli/src/server/tests/sync/sync_transfer_push_test.rs`（10 个测试）。

## 2. 不变量逐条裁定

| 不变量 | 内容 | 判定 | 证据 |
|---|---|---|---|
| **R1** | 中继不得伪造来源；归属由签名定 | ✅ | `relay_proxy.rs:82-85` indirect 必带 proof；`validate_source_proof`→`SyncSourceProof::verify`；测试 `rejects_relay_forged_source_proof`(relay_key 冒签→`SyncInvalidPayload`,shadow=0) |
| **R2** | shadow 写入目标=payload source peer，非 transport | ✅ | `transfer.rs:179 SyncResponse{peer_id: source_peer}` → `receive_remote_ops` 落 `(source_peer,repo_id)`；测试 `uses_message_source_peer_for_shadow_write`(source shadow=1、relay shadow=0) |
| **R3** | `header.peer_id` 须=declared source；`LedgerEntry.peer_id`(author) 不替代 source | ✅ | `relay_proxy.rs:75-77 SourceAttributionMismatch`；测试 `rejects_route_header_source_mismatch` |
| **R4** | authenticated transport peer 仅作会话/scope 校验，不替代 source | ✅ | `transfer.rs:137 transport_peer=require_bound_peer`（仅 scope）；`151 plan_relay_proxy_route(transport,source,target)` 三者分离 |
| **R5** | 一 push 一 source peer | ✅ | 协议 `SyncPush{source_peer_id: PeerId}` 单值；`handle_request:94-108` 每 response 单 push |
| **R7** | 入站 source 必须是 SyncHello diff 请求过的 peer；出站 source 必须是声明可发的 peer | ✅ | 入站 `transfer.rs:140 allows_sync_source`、出站 `:37 allows_sync_export_source`；测试 `rejects_unrequested_authenticated_source` / `rejects_unrequested_relay_source`→`SyncPeerUnauthenticated` |
| **R8** | 无信任/无 proof/无 repo key → 丢弃 | ✅ | indirect 缺 proof→`MissingSourceAttributionProof`；decrypt 需 repo_key；测试 `rejects_indirect_source_without_proof` |
| **E1/E2** | relay 靠 plaintext header 路由、不解密 body；header=repo_id/peer_id/vector/kind | ✅ | `encrypted_payload: Vec<EncryptedOp>` 全程密文；`plan_relay_proxy_route` 仅用 header 字段；destination 才 `decrypt_remote_ops`(需 repo_key) |
| **E3** | relay 不得改 header 来源归属字段 | ✅ | 改 header.peer_id 即触 `SourceAttributionMismatch`；改签名即 proof verify 失败 |
| **T3** | 远端恶意数据只污染对应 remote mirror，不入 local ledger | ✅ | 测试 `does_not_pollute_transport_or_local_ledger`：恶意 source shadow=1、relay shadow=0、**local ledger max_seq 不变** |
| **T4** | merge 到 local 必须显式用户动作 | ✅ | 远端 op 仅入 shadow；local 吸收走显式 `merge_peer`（域 2 §2.C 已验三方合并写 local） |
| **A1** | 先解密再写 storage | ✅ | `decrypt_remote_ops`/`decrypt_pending_payloads` 在 `apply_remote_payloads` 之前 |
| **A2** | ledger append + tree projection 同一 shadow 写事务，中途失败不留前序 op | ✅ | `shadow_transfer.rs:95-117 run_on_shadow_repo` 单 `write_txn`：`append_remote_entries_txn` 内 `append_op_to_txn` + `structure_projection::apply_in_txn` 同事务 |
| **A3** | snapshot reset+replay 同一写事务，replay 失败旧 shadow 保留 | ✅ | `:109-111` `ShadowPayload::Snapshot`→`reset_shadow_repo_txn`+append 同 `write_txn`（失败不 commit→旧内容保留） |
| **A4** | manual 确认单一 peer+repo 目标，混合 fail-closed | ✅ | `sync/engine/manual.rs:91-96 bail!("Manual merge requires one peer/repo target")` |
| **A5** | shadow apply 失败不回滚 local 已确认 write | ✅ | shadow 写在独立 `remotes/<peer>/<repo>.redb` 事务；测试 T3 证 local 不受影响 |
| **维度** | shadow 严格 `PeerId × RepoId`；repo↔repo / branch↔branch 不混 | ✅ | `shadow_dbs: HashMap<PeerId, HashMap<RepoId, Arc<Database>>>`；`plan_relay_proxy_route:65 repo route 校验`；`branch` 为独立 `Option<PeerId>` 字段，与 repo_id 正交 |
| **额外** | 信封 seq 完整性 | ✅ | 测试 `rejects_envelope_seq_mismatch`；`decrypt_remote_ops` 校验 `entry.seq==enc_op.seq` |

## 3. 结论

**18/18 不变量 ALIGNED，零偏差。** P2P/中继层不是"勉强实现"，而是：
1. **专用 core 形式化模块** `plan_relay_proxy_route`（分离 transport/source/target，indirect 必带 proof）。
2. **加密来源证明** `SyncSourceProof`（sign_source/verify），使中继无法伪造或篡改归属。
3. **shadow 严格按 `PeerId×RepoId` 隔离**，恶意/远端数据绝不自动入 local ledger。
4. **单事务原子 shadow apply**（ledger+projection），失败不污染 local。
5. **10 个对抗式测试**逐条钉死：relay 冒签、未请求 source、间接无证明、header/kind 不符、跨 shadow 污染、本地不变。

USER 强调的三点全部成立：**① P→P→P 中继归属由 A 签名定（非 C 通道）；② shadow 严格 repo×peer 维度；③ repo↔repo / branch↔branch 不混。**

> 未覆盖（如需另审）：§8 重连 backoff 矩阵、§10.1 snapshot fallback 的 vector-gap 阈值细节、§11 forbidden patterns 全集、relay **转发**侧（本端作为 C 中继 A→B 的出站转发路径，本审主要验了**接收/归属**侧）。
