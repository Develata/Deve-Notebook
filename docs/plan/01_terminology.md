# 01_terminology.md - 术语与定义篇 (Terminology & Definitions)

## Metadata

- `Layer`: `Foundation`
- `Status`: `Current MUST`
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
* 任何影响一致性与安全性的事实 MUST 位于 Ledger，或 MUST 可由 Ledger 唯一推导；Vault/Markdown 仅承载可读投影。
* 需要明确边界时，使用“**非目标**”直接排除。

## 2. Core Definitions (核心术语定义)

*   **Ledger (账本)**：系统唯一真值源（Source of Truth）；只追加、不可就地修改的账本事实序列 $L = [Fact_1, Fact_2, ..., Fact_n]$。
    *   任何状态变更 $S_{t+1} = Apply(S_t, Fact_{t+1})$ 必须且只能由 Ledger 确定性推导。
    *   **Fact Partition**：账本事实至少可分为 `Content Facts` 与 `Structure Facts`；前者描述文本变化，后者描述节点/路径结构变化。
*   **Snapshot (快照)**：Ledger 在特定账本事实序列位置的状态压缩 $S_t$。
    *   $Snapshot(t) \equiv Fold(Fact_1...Fact_t)$，用于启动、校正与加速 fold；Snapshot 可重建，不是独立真值源。
*   **Projection (投影)**：从 Ledger 派生的、面向用户的可读/可编辑形式（如 Markdown 文件）。
    *   $P = Project(S_{ledger})$。投影不承载权威状态；对投影的外部修改必须先转为差异，再经 Reconciliation 生成 Ledger Facts。
*   **Vault (投影仓)**：宿主文件系统上的一个具体目录路径 `$ROOT/data/vault`。
    *   是 Projection 的物理容器。
    *   **External Edit**：发生在 Vault 内但未经 Deve authority 写路径产生的修改；不得直接成为权威状态。
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
*   **Reconstruction (重建/反推)**：从 Vault 外部突变提取 $\Delta_{fs}$ 的过程。
    *   Reconstruction 只产生候选差异；它本身不得写 authority。
*   **Reconciliation (和解/协调)**：将外部突变合并回权威 Ledger 的过程。
    *   $Merge(L_{current}, \Delta_{fs}) \to L_{next}$。
*   **Peer (节点)**：P2P 网络拓扑图 $G=(V, E)$ 中的顶点 $v \in V$。
    *   所有 Peer 在协议层完全对等，拥有全量或子集 Ledger 副本。
*   **Relay (中继)**：具有 $Attr_{always\_on}$ 的 Peer，只做加密数据 blind storage 与流量转发，不解密业务数据。
*   **LedgerSeq (账本序列数)**：Peer 维度的单调递增计数器。
    *   $Seq(P, i) \in \mathbb{N}$（实现为 `u64`），表示 Peer $P$ 产生的第 $i$ 条账本事实。
    *   `(PeerId, LedgerSeq)` 用于因果定位；repo 落盘全序由 `GlobalSeq` / `LEDGER_OPS` 主键决定。
*   **Vector Clock (向量时钟)**：因果历史的数学表达。
    *   $VC = \{ (PeerID_1, Seq_1), (PeerID_2, Seq_2), ... \}$，用于 diff 与并发冲突检测。

## 3. Data Structure Terms (数据结构术语)

* **Three Stores (三库隔离)**：
    * **Store A (Vault)**：用户工作区 $W_{user}$。
        *   $W_{user} \approx Project(L_{local})$。允许包含未通过 Reconciliation 进入 Ledger 的脏数据（Dirty State）。
    * **Store B (Local Branch)**：本地权威分支 $B_{local}$。
        *   对应 `ledger/local/`，包含多个 `.redb` Repo 文件。
        *   $Write(B_{local})$ 仅允许通过 Command/System 写入。
    *   **Store C (Remote Branches)**：远端影子分支集合 $\Sigma_{remote} = \{ B_{peer_1}, B_{peer_2}, ... \}$。
        *   物理路径：`ledger/remotes/<PeerName>/`，按 PeerUUID 检索。
        *   $\forall B \in \Sigma_{remote}, ReadOnly(B)$（Editor View），但 Gossip Protocol 可写入同步数据。
    *   **Branch (分支)**：以节点为单位的数据集合 $B_{peer}$。
        *   1 Branch $\leftrightarrow$ 1 OS Folder（如 `ledger/local` 或 `ledger/remotes/ipad`）。
        *   代表一个 Writer Identity 作用域；它不是 git-style feature branch。
        *   Local Branch 与 Remote Branch 数据结构同构；写权限由 branch role 决定。
    *   **Repo (仓库)**：逻辑聚合体 $U_{logical}$。
        *   由 Characteristic Parameter（默认 URL）唯一标识；一个 Logical Repo 可对应多个 Branch 下的 Repo Instances。
        *   Repo 表示逻辑集合，Branch 表示 writer identity 作用域；二者 **MUST NOT** 混用。
    *   **Repo Instance (仓库实例)**：物理存储单元 $U_{physical}$。
        *   每个实例拥有独立 `InstanceUUID`（存于 file header）。
        *   物理文件名 **MUST** 采用 `repo_name.redb`；路径为 `ledger/<branch_path>/<repo_name>.redb`。
        *   同一 Branch 下 `repo_name` **MUST** 唯一；`InstanceUUID` 用于内部检索与去重。

## 4. UI Terminology (界面术语)

*   **Workbench**: 交互界面容器集合 $C_{ui} = \{ \text{SideBar}, \text{Editor}, \text{Panel}, \text{ActivityBar} \}$。
*   **View Container**: $V \in C_{ui}$，特定视图组件（Views）的承载者。
*   **Command Palette**: 全局函数调用入口 $Invoke(CommandId, Args)$。
    *   所有系统能力必须可通过此入口访问，实现 $UI \perp Functionality$（界面与功能解耦）。

## 5. Related Configuration (本章相关配置)

*   无特定配置项，但涉及全局架构定义。
