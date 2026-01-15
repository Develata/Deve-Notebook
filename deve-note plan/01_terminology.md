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

* **Ledger（账本）**：append-only 的 CRDT 操作日志（Ops Log）与其派生的快照（Snapshot）集合；Ledger 是系统唯一真源。
* **Snapshot（快照）**：对某一时点 Ledger 状态的压缩表示，用于快速加载与补偿长日志回放。
* **Projection（投影）**：从 Ledger 派生的可读/可编辑表现形式（例如 Markdown 文件）；投影不是权威源。
* **Vault（投影仓）**：保存投影文件的目录（本方案为 `/data/vault`）。
* **DocId**：文档/资产的稳定主键（UUID），不随重命名/移动改变。
* **Path Mapping（路径映射）**：DocId 与可见路径之间的可变映射；重命名/移动 MUST 只修改映射，不修改 DocId。
* **Capability（能力清单）**：插件/脚本声明的最小权限集合；Host MUST 在运行时强制校验。
* **Host Functions（宿主函数）**：核心暴露给插件的受控 API（例如按 DocId 读写、注册命令、网络请求等），所有调用 MUST 可审计、可限权。
* **Asset（资产）**：图片/附件等二进制对象，拥有独立 DocId；运行时以 `asset://<uuid>` 引用，导出时 MUST 落为标准 Markdown 引用。
* **Reconciliation（和解）**：检测并合并外部修改（例如 VS Code 直接改 Vault 文件）到 Ledger 的过程。
* **Job Queue（作业队列）**：用于执行长任务/插件任务/AI 调用的受控队列，提供超时、取消与并发上限。
* **Peer（节点）**：系统中地位平等的每个设备（PC、Mobile、Server）。
* **Relay（中继）**：特指 Server 节点，其角色降级为 **"Always-on Relay Peer"**，负责存储加密副本与转发流量，但不持有唯一真理。

## 数据结构术语 (Data Structure Terms)

* **Three Stores (三库隔离)**：
    * **Store A (Vault)**：本地 Markdown 文件工作区。
    * **Store B (Local Repo)**：本地核心数据库 (`local.redb`)，**Local Write Only**。
    *   **Store C (Shadow Repo)**：远端影子数据库 (`remotes/peer_X.redb`)，**Remote Write Only**。

## 界面术语 (UI Terminology)

*   **Workbench**：应用整体标准网格布局 (Activity Bar + Side Bar + Editor + Panel)。
*   **View Container**：Side Bar 或 Panel 中承载具体视图（View）的容器。
*   **Command Palette**：全局模态指令中心 (`Cmd+K` / `Cmd+P`)，交互的核心入口。

## 本章相关命令

*   `Cmd+K` / `Ctrl+K`: 呼出 Command Palette (命令面板)。
*   `Cmd+P` / `Ctrl+P`: 呼出 Quick Open (快速打开)。

## 本章相关配置

*   无特定配置项，但涉及全局架构定义。
