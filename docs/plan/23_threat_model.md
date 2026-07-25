# 23_threat_model.md - Threat Model (威胁模型)

## Metadata

- `Layer`: `Governance Contracts (non-layer ownership-axis slice)`
- `Status`: `Current MUST`
- `Version`: `0.1.0`
- `Last Review`: `2026-07-18`
- `Authority Owns`: `STRIDE catalog / key lifecycle (高层流程) / algorithm deprecation / supply chain policy / CVD policy`
- `Authority Defers To`: `07_network#trust-boundary (trust boundary), 07_network#full-peer-mesh-v1 (P2P mesh / FullPeer admission), 08_auth (auth runtime contract), 06_backup#projection-backup-secret-ref-contract (Remote Projection credential refs), 06_backup#remote-import-runtime-boundary (Remote Import session/runtime), 03_storage/authority#sealed-ledger-change-batch (apply authority), 11_ui_design#native-adapter-gate-registry (native shell gate), 13_i18n#i18n-error-code-catalog (错误码/限流码), 17_tech_stack#native-packaging-dependency-gate (供应链依赖门禁), 18_release (artifact 签名), 19_plugins (plugin capability gate), 22_reliability_observability#alerting-tier (告警等级)`
- `Counterpart Feature`: `docs/features/operation-coverage.md (auth / trusted-agent security flows)`
- `Counterpart Acceptance`: `docs/acceptance-cases/00_index.md (AUTH-* / PLUG-001)`
- `Primary Code Areas`: `crates/core/src/security/`；SECURITY.md；docs/adr/ 中安全相关 ADR（B4.3 后）

## 1. Scope & Authority {#threat-model-scope}

本章是**威胁建模契约唯一权威**：登记 STRIDE 目录、密钥生命周期高层流程、算法退役预案、供应链与漏洞披露策略。

- **Owns**：STRIDE catalog（§3）、key lifecycle 高层流程（§4）、algorithm deprecation（§5）、supply chain policy（§6）、CVD policy（§7）。
- **Defers To**：信任边界归 `07_network#trust-boundary`（§2 引用）；P2P mesh、FullPeer `/ws` admission、source attribution 与 shadow-only apply 归 `07_network#full-peer-mesh-v1`；auth runtime 合同（session/token/gate）归 `08_auth`；Remote Projection credential refs 归 `06_backup#projection-backup-secret-ref-contract`；ledger append 校验归 `03_storage/authority`（§3）；native shell / local service gate 归 `11_ui_design#native-adapter-gate-registry` 与 `17_tech_stack#native-packaging-dependency-gate`；错误码/限流码归 `13_i18n#i18n-error-code-catalog`（§3/§4）；供应链依赖门禁归 `17_tech_stack#native-packaging-dependency-gate`、artifact 签名归 `18_release`（§6）；plugin capability gate 归 `19_plugins`（§3）；告警等级归 `22_reliability_observability#alerting-tier`（§7）。本章只承载威胁/边界/策略声明，不重定义这些合同。
- **边界**：本章 **MUST NOT** 重写 auth/network/backup 的运行时合同，也不新增 §6 四层调用链之外的调用层。

## 2. Trust Boundaries {#trust-boundaries}

本章不定义信任边界规则，只引用既有定义作为 STRIDE 分析前提；规范性约束以各 owner 为准：

- relay 转发与来源归属、间接同步写入路径由签名来源决定、relay blind storage：见 `07_network#trust-boundary` 与 `07_network` §10。
- FullPeer mesh、server-to-server `/ws` admission、P2P token 环境变量、shadow-only apply 与显式 merge 边界：见 `07_network#full-peer-mesh-v1` 与 `07_network#full-peer-ws-admission`。
- writer gate、Writer Identity、WebLightPeer（浏览器 repo-scoped transient writer identity）：见 `01_terminology`。
- native `LocalBackend` / `RemoteBrowser` 双模式、Desktop child-process local service、Mobile embedded loopback service 与 shell no-direct-authority：见 `11_ui_design#native-adapter-gate-registry`、`11_ui_design#native-post-gate-common-contract` 与 `17_tech_stack#native-packaging-dependency-gate`。
- Remote Import 信任链：见 `06_backup#remote-import-runtime-boundary` 与 `03_storage/authority#sealed-ledger-change-batch`。
- `PROJECTION_FAULTS` 只保存 repo-local host diagnostics/recovery evidence，不进入
  sync、wire、export 或 Ledger payload；路径与 raw error detail 不得暴露给 Web/remote
  client，Remote Import receipt CAS 仍归 Remote Import authority。

以上 MUST/SHOULD 约束不在本章复述或扩展。

### 2.1 Remote Import Trust Boundary {#remote-import-trust-boundary}

```text
untrusted provider
  -> transport adapter
  -> project-owned bounded sink
  -> immutable digest-bound manifest/blob
  -> remote_import_runtime
  -> sealed source-specific writer
```

provider metadata、listing order、locator 与 remote path 都不是 authority。只有通过 path/budget/digest admission 的 immutable capture 能形成 session；只有 exact session/revision/head/writer validation 后的 sealed batch 能提交 Ledger。

## 3. STRIDE Catalog {#stride-catalog}

| 类别 | 主要威胁面 | 缓解（权威归属） |
|---|---|---|
| Spoofing | 伪造 peer / 伪造 session / 伪造 FullPeer transport | peer Ed25519 签名（`crates/core/src/security/`）；session 鉴权归 `08_auth`；FullPeer bearer token 只作为 `/ws` transport admission，不能替代 `SyncHello` peer signature 与 repo scope proof（`07_network#full-peer-ws-admission`） |
| Tampering | 篡改 ledger / 同步数据 | ledger append-only 与 append validation 归 `03_storage/authority`；同步数据签名来源 / relay attribution 归 `07_network#trust-boundary` 与 §10.5 |
| Repudiation | 否认写入 | ledger 因果链 `(PeerId, LedgerSeq)` 提供审计定位（非完整 per-entry 不可抵赖）；定义归 `01_terminology` 与 `03_storage/authority` |
| Information Disclosure | relay 窥探 / Projection Backup provider credential 泄露 / P2P 与 native bootstrap secret 泄露 | relay blind storage；Projection Backup 只传输 Markdown Projection Workspace files，不传输 ledger history；credential refs 与 provider metadata 边界归 `06_backup#projection-backup-secret-ref-contract`；P2P token material 不得进入 config、日志、URL、browser storage 或 native bootstrap payload（`07_network#static-peer-config`） |
| Denial of Service | 登录爆破 / 连接洪泛 | 速率限制（`AUTH_RATE_LIMITED`，归 `13_i18n`/`08_auth`）；malicious peer 隔离（`07_network` §10.3） |
| Elevation of Privilege | 越权写 / 越权插件能力 / native shell 越权成为 authority | writer gate（`01_terminology`）；plugin capability gate（`19_plugins`）；native shell 即使在 `LocalBackend` 模式也不得直接写 ledger/source-control/search，所有写入仍经本地 server/core writer gate（`11_ui_design#native-post-gate-common-contract`） |

### 3.1 Remote Import STRIDE Contract {#remote-import-stride-contract}

- **Spoofing**：RepoId、session、revision、writer identity、branch 与 CAS 必须 exact-bind；locator/profile 继续遵守 ADR 0008。
- **Tampering**：capture publish 与 Apply transaction 都重验 manifest/blob digest；mtime、ETag、listing order 不参与 authority。
- **Repudiation**：Applied receipt 绑定 session、revision、writer、head 与 exact digests；Ledger actor 固定为 Remote Import Apply。
- **Information Disclosure**：wire/UI 不暴露 locator、provider/host path、blob path、digest、credential 或 raw failure detail。
- **Denial of Service**：单 repo 单 active session、2048 文件、4 MiB 单文件、64 MiB 总量、路径预算、100/200 分页与 terminal 64 条 retention；`cleanup_pending` 永不自动裁剪。
- **Elevation of Privilege**：`PreparedLedgerChangeBatch` 为 crate-private，只能由 source-specific constructor 构造；Prepare 无 Ledger/workspace 写权限，Apply 必须 writer gate + Healthy + Mounted；`LocalCliProxyAuthority` 不复用 browser grant。

### 3.2 Remote Import DoS Boundary {#remote-import-dos-boundary}

数值硬上限唯一归 `06_backup#remote-import-resource-contract`。实现必须先做有界 path/listing admission再 streaming payload；超限在 session Ready/Apply 前 fail-closed，不得先无界分配再报错。

## 4. Key Lifecycle (高层流程) {#key-lifecycle}

仅登记高层生命周期；具体合同 Defers To 各章。

| 密钥 | 用途 | 生命周期要点 | 权威 |
|---|---|---|---|
| Auth token (JWT, HS256) | 会话鉴权 | 签发 / 过期 / 撤销（token_version）；运行时合同归 `08_auth`，错误码归 `13_i18n` | `08_auth` |
| P2P admission token | server-to-server `/ws` transport admission | 只通过环境变量间接引用；token material 不写入 config、日志、bootstrap payload、browser storage 或 URL；失效与轮换策略由部署环境负责 | `07_network#full-peer-ws-admission` |
| Password hash (Argon2) | 口令存储 | 加盐哈希、参数升级 | `08_auth` |
| Peer keypair (Ed25519) | peer 身份与签名 | 现状：生成 / 持久化恢复 / 签名 / 验签（无轮转撤销协议）；轮转/撤销为本章高层策略，协议化 defer `07_network` | `crates/core/src/security/keypair.rs` |
| Remote Projection transport credential ref | push/source-acquisition provider I/O credential reference | secret-free endpoint/bucket/prefix/credential-ref binding 归 `06_backup#projection-backup-secret-ref-contract`；credential value 只能由 runtime resolver 在 provider I/O 时解析 | `06_backup#projection-backup-secret-ref-contract` |

轮转与撤销的具体协议归各权威章节，本章不重定义。Projection Backup 首版不引入 ledger backup encryption key；若未来重新引入 ledger-history backup / pack encryption，必须按 `06_backup#projection-backup-deferred-ledger-backup` 另开 ADR 与 runtime proposal。

## 5. Algorithm Deprecation {#algorithm-deprecation}

加密原语退役**策略**由本章拥有；具体算法标识由各 owner 以其原生格式表达（本章不新造统一 `algo_id` 字段）。退役 **MUST** 经迁移窗口（新旧并存）后，才在某个 minor version 移除旧算法。

| 原语 | 当前算法（标识格式 / owner） | 退役策略 |
|---|---|---|
| JWT 签名 | HS256（JOSE `alg`；`08_auth`） | 迁移到非对称（如 EdDSA）须提供新旧兼容窗口（迁移机制由 `08_auth` 定义）；确认无旧 token 后移除 HS256 |
| 口令哈希 | Argon2（PHC 字符串 algorithm/params；`08_auth`） | 参数/算法升级须提供兼容窗口（re-hash 机制由 `08_auth` 定义），不强制即时全量迁移 |
| peer 签名 | Ed25519（`crates/core/src/security`） | 引入新签名算法须先在 `07_network` protocol schema 增加算法标识与协商；旧 peer 在窗口内仍可验签 |

任一算法移除都 **MUST** 先按 `00_engineering_constitution` §8 提交骨架级变更分析。新增 per-signature 算法标识属 `07_network` protocol schema 变更，不在本章定义。

## 6. Supply Chain {#supply-chain}

- **SBOM**：发布产物 **MUST** 生成依赖清单（`cargo` 依赖树 + 锁定版本）。
- **Reproducible Build**：发布构建 **SHOULD** 可复现；锁文件 `Cargo.lock` 纳入版本管理。
- **Dependency Gate**：原生打包可选依赖门禁归 `17_tech_stack#native-packaging-dependency-gate`；新增依赖 SHOULD 经审查（许可证 / 维护状态 / 体积）。
- **Provider Adapter**：Remote Projection transport 必须保持轻量、可审计；引入重型 provider SDK或新的 credential authority属于架构停止条件，不能以 B0/B2 普通依赖升级处理。
- **Signing**：发布 artifact 的签名与校验归 `18_release`。

## 7. Coordinated Vulnerability Disclosure {#coordinated-vulnerability-disclosure}

- **入口**：`SECURITY.md` 声明私密报告渠道；不在公开 issue 讨论未披露漏洞。
- **GitHub PVR**：公开仓库的 Private Vulnerability Reporting 启用状态是 GitHub
  repository governance fact，不进入产品 runtime、Ledger、CLI 或 Web。first-tag
  evidence 必须由只读 `github.pvr-enabled` producer 对精确
  `Develata/Deve-Notebook` GET endpoint 取得，并且只有官方响应
  `{"enabled":true}` 才能生成 current-HEAD receipt；producer 不拥有 PUT、DELETE
  或其它 repository setting mutation 能力。
- **Embargo**：修复就绪前对报告内容保密；高危漏洞优先 T1 处理（见 `22_reliability_observability` Alerting Tier）。
- **SLA**：确认 / 初步评估 / 修复发布的目标时限在 `SECURITY.md` 声明。
- **披露**：修复发布后公开致谢与 CVE/advisory（如适用）。

## 8. Related Configuration (本章相关配置)

- 鉴权 / TLS / 速率限制配置：归 `08_auth` 与 `15_settings`。
- P2P static peer 配置只保存 endpoint、repo/peer identity 与 token env 名称；token material 只在运行环境中提供：归 `07_network#static-peer-config`。
- Native `LocalBackend` 只打开本地受控 service；`RemoteBrowser` 只加载远端 HTTPS origin。两者都不授予 shell 直接 authority：归 `11_ui_design#native-adapter-gate-registry` 与 `17_tech_stack#native-packaging-dependency-gate`。
- Remote Projection transport credential refs：归 `06_backup#projection-backup-secret-ref-contract`；Remote Import 不创建新的 credential authority。
- 本章自身无独立配置项。
