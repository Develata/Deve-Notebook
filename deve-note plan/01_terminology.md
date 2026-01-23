# 01_terminology.md - 术语与定义篇 (Terminology & Definitions)

为避免“想法正确但实现含糊”，本白皮书对关键术语给出**定义**，并使用以下规范性用语。

## 规范性用语 (Normative Language)

* **MUST / 必须**：不可违反；违反即视为设计不成立或实现错误。
* **SHOULD / 应**：强烈建议；除非有明确理由与替代方案，否则不应偏离。
* **MAY / 可选**：可按阶段或插件化实现，不进入核心必选路径。

**表达约定（追求精确简练）**：

* 每条要求 SHOULD 可验证（能写测试/能观测/能复现），避免“更好/更强/更优雅”这类不可判定表述。
* 任何影响一致性与安全性的事实 MUST 位于 Ledger，或 MUST 可由 Ledger 唯一推导；Vault/Markdown 仅承载可读投影。
* 需要明确边界时，使用“**非目标**”直接排除。

## 核心术语定义 (Core Definitions)

*   **Ledger (账本)**：系统的唯一真值源（Source of Truth）。
    *   **Description (描述)**：这是一个只增不减的操作记录列表。系统不直接存储最终状态，而是存储“发生了什么”。
    *   定义为一个不可变的操作日志序列 $L = [Op_1, Op_2, ..., Op_n]$。
    *   任何状态变更 $S_{t+1} = Apply(S_t, Op_{t+1})$ 必须且只能由 Ledger 确定性推导。
*   **Snapshot (快照)**：Ledger 在特定 Op 序列位置的状态压缩 $S_t$。
    *   **Description (描述)**：为了避免每次都从头计算，将某个时间点的计算结果保存下来，作为下次计算的起点。
    *   $Snapshot(t) \equiv Fold(Op_1...Op_t)$，用于启动与数据校正。
*   **Projection (投影)**：从 Ledger 派生的、面向用户的可读/可编辑形式（如 Markdown 文件）。
    *   **Description (描述)**：这是呈现给用户看的当前状态。用户对它的修改不会直接生效，而是必须转化成新的 Op 追加到 Ledger 中。
    *   $P = Project(S_{ledger})$。投影本质上是 Ledger 的一个视图（View），不承载权威状态。
*   **Vault (投影仓)**：宿主文件系统上的一个具体目录路径 `$ROOT/data/vault`。
    *   是 Projection 的物理容器。
    *   **External Edit**: 发生在 Vault 内但未经由 Deve-Note 产生的修改视为“外部突变”，需经 Reconstruction 流程回到 Ledger。
*   **Tree State (树状态)**:
    *   **Definition**: 内存中维护的文件树结构缓存 $T_{mem}$ (Managed by `TreeManager`)。
    *   **Role**: 用于快速生成目录树 UI，减少 IO 扫描，并生成 `TreeDelta` 推送前端。
*   **DocId**: 图（Graph）中的不变节点标识。
    *   定义为 128-bit UUID v4。
    *   在时空上唯一标识一个逻辑文档，$DocId \perp FilePath$（DocId 正交于文件路径）。
*   **Path Mapping (路径映射)**： Ledger 中维护的一个双射函数 $M: DocId \leftrightarrow FilePath$。
    *   **Description (描述)**：文档的唯一标识（DocId）和它在文件系统中的位置（Path）是分离的。移动文件只是修改了这个映射关系，文档本身 ID 不变。
    *   文件重命名/移动操作仅修改函数 $M$，不改变 $DocId$。
*   **Capability (能力清单)**：插件/脚本的可执行函数集合 $C \subseteq \{HostFunctions\}$。
    *   Host 在运行时强制校验 $Call(f) \iff f \in C_{plugin}$。
*   **Host Functions (宿主函数)**：系统暴露的受控 API 全集 $H$。
    *   所有因果性（Causality）操作必须经由 $h \in H$ 完成。
*   **Asset (资产)**：由 DocId 标识的二进制字节序列。
    *   运行时引用形式：`asset://<uuid>`。
    *   物理存储形式：Content Addressable Storage (CAS) 或由 Ledger 管理的 Blob。
*   **Reconciliation (和解/协调)**：将外部突变合并回权威 Ledger 的过程。
    *   $Merge(L_{current}, \Delta_{fs}) \to L_{next}$。
*   **Peer (节点)**：P2P 网络拓扑图 $G=(V, E)$ 中的顶点 $v \in V$。
    *   所有 Peer 在协议层完全对等，拥有全量或子集 Ledger 副本。
*   **Relay (中继)**：具有特殊属性 $Attr_{always\_on}$ 的 Peer。
    *   功能降级：只作为加密数据的 "Blind Storage" 和流量转发者，不参与业务逻辑解密。


*   **OpSeq (操作序列数)**：Peer 维度的单调递增计数器。
    *   **Definition**: $Seq(P, i) \in \mathbb{N}$ (Implementation: `u64`)，表示 Peer $P$ 产生的第 $i$ 个操作。
    *   **Property**: 单调递增，决定操作的全序关系。
*   **Vector Clock (向量时钟)**：因果历史的数学表达。
    *   **Definition**: $VC = \{ (PeerID_1, Seq_1), (PeerID_2, Seq_2), ... \}$。
    *   **Usage**: 用于检测数据差异（Diff Calculation）和并发冲突（Conflict Detection）。

## 数据结构术语 (Data Structure Terms)

* **Three Stores (三库隔离)**：
    * **Store A (Vault)**：用户工作区 $W_{user}$。
        *   **Description**: 用户直接操作的文件目录，包含 Markdown 文件。
        *   $W_{user} \approx Project(L_{local})$。允许包含未通过 Reconciliation 进入 Ledger 的脏数据（Dirty State）。
    * **Store B (Local Branch)**：本地权威分支 $B_{local}$。
        *   **Description**: 本地用户的全量数据集合，是当前设备的数据真源，对应文件系统目录 `ledger/local/`。
        *   **Structure**: 包含多个 `.redb` 文件，每个对应一个 Repo。
        *   Constraint: $Write(B_{local})$ 仅允许通过 Command/System 写入。
    *   **Store C (Remote Branches)**：远端影子分支集合 $\Sigma_{remote} = \{ B_{peer_1}, B_{peer_2}, ... \}$。
        *   **Physical Layout**: `ledger/remotes/<PeerName>/` (Retrieval by PeerUUID)。
        *   **Constraint**: $\forall B \in \Sigma_{remote}, ReadOnly(B)$ (Editor View), but $Writable(B)$ (Gossip Protocol).
    *   **Branch (分支)**：以节点为单位的数据集合 $B_{peer}$。
        *   **Physical Mapping**: 1 Branch $\leftrightarrow$ 1 OS Folder (e.g. `ledger/local` or `ledger/remotes/ipad`).
        *   **Semantics**: 代表一个特定的 Writer Identity (Local User 或 Peer User)。
        *   **Equality**: Local Branch 与 Remote Branch 在数据结构上严格同构 (Isomorphic)。
    *   **Repo (仓库)**：逻辑聚合体 $U_{logical}$。
        *   **Definition**: 由 **Characteristic Parameter** (默认为 URL) 唯一标识的逻辑集合。
        *   **Relation**: 一个 Logical Repo 对应多个不同 Branch 下的 Repo Instances。
    *   **Repo Instance (仓库实例)**：物理存储单元 $U_{physical}$。
        *   **Identity**: 每个实例拥有独立的 **InstanceUUID** (stored in file header)。
        *   **Naming**: 物理文件名 **MUST** 采用 `repo_name.redb` (Human Readable)。
        *   **Physical Mapping**: `ledger/<branch_path>/<repo_name>.redb`.
        *   **Note**: 在同一 Branch 下，`repo_name` **MUST** 唯一 (Unique Constraint)。`InstanceUUID` 用于内部逻辑检索与去重。

## 界面术语 (UI Terminology)

*   **Workbench**: 交互界面容器集合 $C_{ui} = \{ \text{SideBar}, \text{Editor}, \text{Panel}, \text{ActivityBar} \}$。
*   **View Container**: $V \in C_{ui}$，特定视图组件（Views）的承载者。
*   **Command Palette**: 全局函数调用入口 $Invoke(CommandId, Args)$。
    *   所有系统能力必须可通过此入口访问，实现 $UI \perp Functionality$（界面与功能解耦）。



## 本章相关配置

*   无特定配置项，但涉及全局架构定义。
