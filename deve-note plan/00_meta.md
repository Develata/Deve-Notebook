# 📑 Deve-Note Plan - Meta & Boundaries

**版本**: 0.0.1
**状态**: 后端逻辑闭环 + 前端交互定义完整 + 数据/安全强化落地。
**核心理念**: 账本为真源 + **三位一体隔离 (Trinity Isolation)** + **Git-Flow 数据主权** + 工业级内核。

**项目定位**: 个人部署在服务器上，仅供自己使用的开源个人 Wiki Markdown 笔记项目（支持 LaTeX 数学公式）。

## 术语与规范性用语 (Terminology)

为避免“想法正确但实现含糊”，本白皮书对关键术语给出**定义**，并使用以下规范性用语：

* **MUST / 必须**：不可违反；违反即视为设计不成立或实现错误。
* **SHOULD / 应**：强烈建议；除非有明确理由与替代方案，否则不应偏离。
* **MAY / 可选**：可按阶段或插件化实现，不进入核心必选路径。

**表达约定（追求精确简练）**：

* 每条要求 SHOULD 可验证（能写测试/能观测/能复现），避免“更好/更强/更优雅”这类不可判定表述。
* 任何影响一致性与安全性的事实 MUST 位于 Ledger，或 MUST 可由 Ledger 唯一推导；Vault/Markdown 仅承载可读投影。
* 需要明确边界时，使用“**非目标**”直接排除。

**核心术语定义**：

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
* **Three Stores (三库隔离)**：
    * **Store A (Vault)**：本地 Markdown 文件工作区。
    * **Store B (Local Repo)**：本地核心数据库 (`local.redb`)，**Local Write Only**。
    * **Store C (Shadow Repo)**：远端影子数据库 (`remotes/peer_X.redb`)，**Remote Write Only**。

## 核心边界（最高优先级）

以下边界用于防止核心膨胀；与任何其它章节冲突时，以本节为准。

**Core MUST（核心必须）**：

* 只提供“登录界面 + 侧边栏导航 + 命令系统 + Markdown 撰写体验（含 TeX 公式渲染）”的最小闭环。
* 维护 Ledger/Vault 的一致性闭环：写入、同步、和解、冲突处理、可恢复性与可观测性。
* 提供稳定的 Host Functions、事件总线、作业队列与 Capability 校验，用于承载插件扩展。
* 在非linux端使用时，先将命令和路径转化为 linux 命令与路径，再按照原定linux程序进行操作。
* **Code Modularity & Documentation (模块化与文档铁律)**：所有代码文件 **MUST** 保持高度模块化（单一职责）；文件头部 **MUST** 包含中文注释块，明确说明：(1) 本文件的架构作用；(2) 核心功能清单；(3) 是否属于核心必选路径 (Core MUST) 或可选扩展 (Optional)。

**Core MUST NOT（核心禁止）**：

* 不内置任何重能力为默认必选路径：AI、全文索引、计算/执行型代码块、批量导入导出管线、图像处理、复杂渲染/排版等。
* 不让任何重任务阻塞交互：核心 UI 路径必须恒轻；重任务必须走作业队列并可取消/可降级。
* 不引入私有格式污染导出：对用户可见的文本投影必须保持标准 Markdown 语义。
* **Ignored Files Strategy (忽略策略)**：对于 Vault 中无法解析或过大的非 Markdown/非 Asset 文件（如编译产物、系统临时文件），核心 **MUST** 依据 `.deveignore` 或内置规则直接忽略，**MUST NOT** 尝试将其摄入 Ledger，避免核心膨胀或阻塞。

**Plugin MAY（插件可选）**：

* 实现并按需启用重能力（AI、索引、可执行 fenced blocks、图像/表格/图形、批处理工具等），并通过 Capability、资源配额与队列上限进行隔离。
* 扩展 UI（面板/命令/预览器/侧栏小组件）与数据能力（资产处理、外部集成），但不得破坏 Ledger 真源与导出约束。
