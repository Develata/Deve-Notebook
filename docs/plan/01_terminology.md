# 01_terminology.md - 术语与定义篇 (Terminology & Definitions)

## Metadata

- `Layer`: `Foundation`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-30`
- `Counterpart Feature`: `docs/features/01_terminology.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/01_terminology.md`
- `Primary Code Areas`: `crates/core/src/models/`, `docs/plan/01_terminology.md` (self-referential glossary)

本章定义关键术语与规范性用语。

## 1. Normative Language (规范性用语)

* **MUST / 必须**：不可违反；违反即视为设计不成立或实现错误。
* **SHOULD / 应**：强烈建议；除非有明确理由与替代方案，否则不应偏离。
* **MAY / 可选**：可按阶段或插件化实现，不进入核心必选路径。

**表达约定（追求精确简练）**：

* 每条要求 SHOULD 可验证（能写测试/能观测/能复现），避免“更好/更强/更优雅”这类不可判定表述。
* 任何影响一致性与安全性的事实 MUST 位于 Ledger，或 MUST 可由 Ledger 唯一推导；Projection Workspace / Markdown 仅承载可读投影。
* 需要明确边界时，使用“**非目标**”直接排除。

## 2. Core Definitions (核心术语定义)

*   **Ledger (账本)**：系统唯一真值源（Source of Truth）；只追加、不可就地修改的账本事实序列 $L = [Fact_1, Fact_2, ..., Fact_n]$。
    *   任何状态变更 $S_{t+1} = Apply(S_t, Fact_{t+1})$ 必须且只能由 Ledger 确定性推导。
    *   **Fact Partition**：账本事实至少可分为 `Content Facts` 与 `Structure Facts`；前者描述文本变化，后者描述节点/路径结构变化。
*   **Snapshot (快照)**：Ledger 在特定账本事实序列位置的状态压缩 $S_t$。
    *   $Snapshot(t) \equiv Fold(Fact_1...Fact_t)$，用于启动、校正与加速 fold；Snapshot 可重建，不是独立真值源。
*   **Projection (投影)**：从 Ledger 派生的、面向用户的可读/可编辑形式（如 Markdown 文件）。
    *   $P = Project(S_{ledger})$。投影不承载权威状态；对投影的外部修改必须先转为差异，再经 Reconciliation 生成 Ledger Facts。
*   **Deve-authorized Write Path (Deve 授权写路径)**：由 Deve runtime 明确发起、绑定 repo scope / writer gate，并通过 ledger append、projection writeback、source-control import 或 repair 命令产生可审计副作用的写路径。
    *   裸文件写入、外部编辑器保存、外部 `git checkout/reset/pull/rebase` 造成的 Projection Workspace 变化不属于该路径。
*   **Writer Identity (写入身份)**：能够产生 ledger facts 或 pending authority intent 的受控身份。
    *   Branch 表达持久 writer identity 作用域；browser tab / native shell 只能获得 repo-scoped transient writer identity。
*   **Writer Gate (写入闸门)**：把 auth session、repo scope、branch role、`scope_nonce` 与 writer registration 合并后的写入许可。
    *   未通过 writer gate 的请求 **MUST NOT** append ledger、写入 staging、写入 pending/import 或确认 pending overlay。
    *   Writer Gate 只授予 Local Branch 写入；Remote Branch 不得因 merge、Source Control、editor 或 UI action 获得写入语义。
*   **RepoId (仓库身份)**：repo 的不可变机器身份，UUID-first。
    *   所有 repo-scoped 业务算子在执行前都必须先解析并绑定 `RepoId`。
    *   `RepoName`、URL、路径名与 selector 都只能作为输入别名、显示属性或恢复线索，不得替代 `RepoId`。
*   **RepoNameBinding (仓库名绑定)**：`RepoId` 到当前可变显示名的 ledger-derived 绑定。
    *   最小字段为 `repo_id / repo_name / name_epoch / changed_at_seq`。
    *   repo rename 只能更新 `RepoNameBinding`，不得改变 `RepoId`。
    *   `repo_name -> RepoId` 只能作为 catalog index；若解析不唯一或与 ledger header 不一致，必须 fail-closed。
*   **Pending Overlay (待确认叠层)**：Web thin-client session 内的未确认本地编辑集合 $O_{session}$。
    *   Pending overlay 是 session runtime state，不是 `pending_fs_ops` side table 条目。
    *   Pending overlay 只能由 `Ack` / `Reject` / stale-scope recovery 清理，不得由 watcher 或 scan 清理。
*   **pending_fs_ops (文件系统待处理队列)**：外部文件系统变化或显式 import 进入 Source Control 前的 repo runtime side table。
    *   `PendingFsEntry` 是 `pending_fs_ops` 中的一条 repo-scoped 文件系统/import pending 记录。
    *   `pending_fs_ops` 不承载 Web pending overlay，不是 ledger authority。
*   **Projection Workspace / Vault (投影工作区 / 投影仓)**：宿主文件系统上绑定到单个本地 repo instance 的计算目录，形式为 `<projection_base>/<safe_repo_name>--<repo_id>/`。
    *   `projection_base` 是用户通过 Projection Locator 指定的父目录；它本身不是 repo workspace。
    *   Projection Workspace 是该 repo 的 Markdown Projection 物理容器，不是全局共享仓库。
    *   系统 **MUST NOT** 要求存在总 `vault` 根目录；旧模型中的 `vault` 在新模型下只是某个 locator base，因此最终目录自然是 `vault/<safe_repo_name>--<repo_id>/`。
    *   Markdown 文件可以直接位于 repo workspace 根目录（如 `a.md`），也可以位于子目录（如 `notes/a.md`）；系统不得要求固定 `notes/` 子目录。
    *   **External Edit**：发生在 Projection Workspace 内但未经 Deve-authorized Write Path 产生的修改；不得直接成为权威状态。
*   **Projection Locator (投影定位记录)**：host-local runtime state，描述 `RepoId -> Projection Base path` 的绑定。
    *   Projection Locator 只存在于当前宿主环境；不得写入 repo ledger facts，不得通过 P2P 同步，不得作为 logical repo identity。
    *   本地可写 repo 在 mounted write path 前 **MUST** 具备可 canonicalize 的 Projection Locator；唯一性与冲突检查作用于计算得到的 Projection Workspace root。
*   **Tree State (树状态)**:
    *   内存文件树缓存 $T_{mem}$，由 `TreeManager` 管理。
    *   用于目录树 UI、减少 IO 扫描并生成 `TreeDelta`。
    *   它是 Structure Facts projection 的内存视图，不是独立真值源。
*   **NodeId**: 树（Tree）中任意节点（文件或目录）的不变标识。
    *   定义为 128-bit UUID v4。
    *   在时空上唯一标识一个节点实体；目录与文件统一为 `Node`，以 `NodeId` 为主键。
    *   **Layer**: 结构层标识（Structure Layer），用于表达父子关系、重命名、移动。
*   **DocId**: 图（Graph）中的不变节点标识，**仅对 `kind = File` 的节点有效**。
    *   定义为 128-bit UUID v4（与 `NodeId` 同类型）。
    *   在时空上唯一标识一个逻辑文档，$DocId \perp FilePath$（DocId 正交于文件路径）。
    *   **Layer**: 内容层标识（Content Layer），用于表达文档内容演化（patch / snapshot）。
*   **NodeId vs DocId Relationship (关系形式化)**:
    *   对 `kind = File` 的节点：`doc_id == node_id`（同一 UUID 在结构层与内容层复用），即文件节点的结构身份与内容身份统一。
    *   对 `kind = Dir` 的节点：仅有 `node_id`，`doc_id = None`。
    *   **Rule**: Structure Facts 的主键 MUST 是 `NodeId`；Content Facts 的主键 MUST 是 `DocId`。业务层 MUST NOT 交换使用两者。
*   **Path Mapping (路径映射)**：由 Structure Facts fold 出的一个派生函数 $M: DocId \leftrightarrow FilePath$。
    *   DocId 与 FilePath 分离；移动文件只改变路径投影，不改变文档身份。
    *   重命名/移动通过 Structure Facts 表达，不依赖 metadata 表副作用直写。
*   **Capability (能力清单)**：插件/脚本的可执行函数集合 $C \subseteq \{HostFunctions\}$。
    *   Host 在运行时强制校验 $Call(f) \iff f \in C_{plugin}$。
*   **Host Functions (宿主函数)**：系统暴露的受控 API 全集 $H$。
    *   所有因果性（Causality）操作必须经由 $h \in H$ 完成。
*   **Asset (资产)**：由 DocId 标识的二进制字节序列。
    *   运行时引用形式：`asset://<uuid>`。
    *   物理存储形式：Content Addressable Storage (CAS) 或由 Ledger 管理的 Blob。
*   **Reconstruction (重建/反推)**：从 Projection Workspace 内的 External Edit 提取 $\Delta_{fs}$ 的过程。
    *   Reconstruction 只产生候选差异；它本身不得写 Ledger authority。
*   **Reconciliation (和解/协调)**：将外部突变合并回权威 Ledger 的过程。
    *   $Merge(L_{current}, \Delta_{fs}) \to L_{next}$。
*   **Peer (节点)**：P2P 网络拓扑图 $G=(V, E)$ 中的顶点 $v \in V$。
    *   所有 Peer 在协议层完全对等，拥有全量或子集 Ledger 副本。
*   **WebLightPeer (Web 轻节点)**：浏览器端 thin-client peer。
    *   WebLightPeer 只持有 session、snapshot、pending overlay 与 repo-scoped protocol state，不持有本地 ledger authority。
*   **Relay (中继)**：具有 $Attr_{always\_on}$ 的 Peer，只做加密数据 blind storage 与流量转发，不解密业务数据。
*   **LedgerSeq (账本序列数)**：Peer 维度的单调递增计数器。
    *   $Seq(P, i) \in \mathbb{N}$（实现为 `u64`），表示 Peer $P$ 产生的第 $i$ 条账本事实。
    *   `(PeerId, LedgerSeq)` 用于因果定位；repo 落盘全序由 `GlobalSeq` / `LEDGER_OPS` 主键决定。
*   **Vector Clock (向量时钟)**：因果历史的数学表达。
    *   $VC = \{ (PeerID_1, Seq_1), (PeerID_2, Seq_2), ... \}$，用于 diff 与并发冲突检测。
*   **scope_nonce (作用域版本)**：当前已确认 repo / branch scope 的连接内单调版本。
    *   所有 repo-scoped server message、write intent 与 UI writable state **MUST** 绑定当前 `scope_nonce`。
*   **switch_nonce (切换版本)**：客户端发起 repo / branch switch 时声明的候选 scope 版本。
    *   `switch_nonce` **MUST** 严格大于当前 `scope_nonce`；stale switch 必须 fail-closed。
*   **Repo Health States (仓库健康状态)**：`Healthy` / `Degraded` / `Repairing` / `Quarantined` 的 glossary 级名称。
    *   `Healthy`：通过完整性校验，authority 与 projection 一致，可正常读写。
    *   `Degraded`：检测到非致命异常，部分能力受限，但 authority 未损坏。
    *   `Repairing`：正在执行 repair 流程，写入受控。
    *   `Quarantined`：检测到完整性风险，repo 被隔离，禁止常规写入直至修复。
    *   **Authority Defers To**：`04_repository#repo-health-and-repair`。状态全集、状态迁移规则与准入/禁止条件唯一归该章；本条仅提供名称，不得在此扩展或偏离。

## 2.bis Reliability Vocabulary (可靠性术语)

> 以下术语支撑 Governance Contract 章节 `22_reliability_observability`；其权威定义归该章，本节仅登记 glossary 名称。

*   **SLO (Service Level Objective，服务级目标)**：对某项服务质量指标设定的目标阈值（如 p99 latency ≤ X）。
*   **SLI (Service Level Indicator，服务级指标)**：度量 SLO 达成情况的可观测量化指标。
*   **Error Budget (错误预算)**：SLO 允许范围内可消耗的失败/降级额度；耗尽即触发治理动作。
*   **Telemetry Schema (遥测模式)**：结构化日志/事件字段的标准定义。
*   **Metrics Taxonomy (指标分类法)**：counter / gauge / histogram 等指标的命名与维度规则。
*   **Tracing Span (追踪跨度)**：一次操作在异步/分布式链路中的可观测时间区间。
*   **Alerting Tier (告警等级)**：错误码族 / health 信号到告警严重度的分级映射；映射归 `22_reliability_observability`，错误码定义归 `13_i18n`、health 状态归 `04_repository#repo-health-and-repair`。
*   **DR Playbook (灾难恢复手册)**：灾难恢复操作索引；权威恢复步骤归 `06_backup` 与 `04_repository#repo-health-and-repair`。

## 2.ter Operations Vocabulary (运维与操作术语)

> 以下术语支撑 Governance Contract 章节 `20_operations_catalog`（B3.1 规划中）。

*   **OpId (操作标识)**：user operation 层的稳定标识，原子形式 `op.<domain>.<flow>.<verb>`（如 `op.editor.save_pending`）。
    *   与 `14_commands#cli-commands` / `14_commands#command-palette-shortcuts` 的 `CommandId` **正交**：`OpId` 是 user operation 层标识，`CommandId` 是 instruction interface 层标识；二者 **MUST NOT** 混用。
*   **Flow ID (操作流标识)**：operation-flow 的稳定标识 `flow.<domain>.<flow>`，是 `20_operations_catalog` 目录的键；一个 Flow ID 聚合该 flow 内的全部原子 `OpId`，原子 OpId 由 `docs/features/operations/*.md` 投影文件枚举。`Flow ID` 与 `OpId` 不得混用。
*   **Failure Family (错误码族)**：错误码族别名，引用 `13_i18n#i18n-error-code-catalog`；本条不定义具体错误码。
*   **Extension Point (扩展点)**：暴露给 `19_plugins` / host function 的受控扩展位置。
*   **Replacement Point (替换点)**：允许通过 feature flag 替换实现的位置。
*   **Owning Boundary (归属边界)**：某 op 所属的 runtime boundary（沿各章 §Runtime Boundary）。
*   **Gate (闸门)**：进入某 op 必须满足的前置条件。

## 2.quater Threat Vocabulary (威胁建模术语)

> 以下术语支撑 Governance Contract 章节 `23_threat_model`（B3.4 规划中），首次展开登记。

*   **STRIDE**：Spoofing / Tampering / Repudiation / Information Disclosure / Denial of Service / Elevation of Privilege 的威胁分类法。
*   **CVD (Coordinated Vulnerability Disclosure，协调披露)**：漏洞协调披露流程（embargo / SLA / SECURITY.md）。
*   **SBOM (Software Bill of Materials，软件物料清单)**：构建产物的依赖与组件清单，用于供应链审计。

## 2.quinquies Governance Vocabulary (治理术语)

*   **Governance Contract (治理合同)**：与 A-E 模块层正交的合同切片，沿 Ownership Axis 表达跨层治理；不是 A-E 之外的第六层。
*   **Authority Owns (权威拥有)**：Metadata 字段，声明本章唯一拥有、其他章节不得重定义的对象。
*   **Authority Defers To (权威让渡)**：Metadata 字段，声明本章引用但不拥有的对象所在章节。
*   **Decision History Slice (决策历史切片)**：`docs/adr/` 所在的切片，与 plan/governance、features/walkthrough、acceptance-cases/automation 切片并列。

## 3. Data Structure Terms (数据结构术语)

* **Three Stores (三库隔离)**：
    * **Store A (Projection Workspaces)**：一组 repo-scoped 用户工作区 $\{W_{repo}\}$。
        *   $W_{repo} \approx Project(L_{repo})$。允许包含未通过 Reconciliation 进入 Ledger 的脏数据（Dirty State）。
        *   每个本地可写 repo **MUST** 通过 Projection Locator 绑定到一个 projection base，并派生出独立的 `<projection_base>/<safe_repo_name>--<repo_id>/` 物理目录。
    * **Store B (Local Branch)**：本地权威分支 $B_{local}$。
        *   对应 `ledger/local/`，包含多个 `.redb` Repo 文件。
        *   $Write(B_{local})$ 仅允许通过 Command/System 写入。
    *   **Store C (Remote Branches)**：远端影子分支集合 $\Sigma_{remote} = \{ B_{peer_1}, B_{peer_2}, ... \}$。
        *   物理路径：`ledger/remotes/<PeerName>/`，按 PeerUUID 检索。
        *   $\forall B \in \Sigma_{remote}, ReadOnly(B)$ 对所有用户操作、Editor、Source Control、Merge 与 plugin writer 均为硬性约束。
        *   Remote Branch 只能由经认证的同步 ingest 维护本机 force-mirror / shadow 内容；该维护路径不是 Local Writer Gate，不授予 UI 或 Source Control 写权限，也不得被 merge 结果写回。
    *   **Branch (分支)**：以节点为单位的数据集合 $B_{peer}$。
        *   1 Branch $\leftrightarrow$ 1 OS Folder（如 `ledger/local` 或 `ledger/remotes/ipad`）。
        *   代表一个 Writer Identity 作用域；它不是 git-style feature branch。
        *   Local Branch 与 Remote Branch 数据结构同构；写权限由 branch role 决定。
    *   **Repo (仓库)**：逻辑聚合体 $U_{logical}$。
        *   由 `RepoId` 唯一标识；Characteristic Parameter（默认 URL）只作为协作发现、备份 locator 或恢复线索。
        *   Repo 表示逻辑集合，Branch 表示 writer identity 作用域；二者 **MUST NOT** 混用。
    *   **Repo Instance (仓库实例)**：物理存储单元 $U_{physical}$。
        *   每个实例拥有独立 `RepoId`（存于 file header / genesis metadata）。
        *   物理文件名 **MUST** 采用 `<repo_id>.redb`；路径为 `ledger/<branch_path>/<repo_id>.redb`。
        *   同一 Branch 下 `RepoName` 可以重复；selector by name 不唯一时必须要求显式 `RepoId`。

## 4. UI Terminology (界面术语)

*   **Workbench**: 交互界面容器集合 $C_{ui} = \{ \text{SideBar}, \text{Editor}, \text{Panel}, \text{ActivityBar} \}$。
*   **View Container**: $V \in C_{ui}$，特定视图组件（Views）的承载者。
*   **Command Palette**: 全局函数调用入口 $Invoke(CommandId, Args)$。
    *   所有系统能力必须可通过此入口访问，实现 $UI \perp Functionality$（界面与功能解耦）。

## 5. Related Configuration (本章相关配置)

*   无特定配置项，但涉及全局架构定义。
