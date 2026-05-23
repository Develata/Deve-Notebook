# 02_positioning.md - 项目定位与设计篇 (Project Positioning)

## Metadata

- `Layer`: `Foundation`
- `Status`: `Current MUST`
- `Counterpart Feature`: `docs/features/02_positioning.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/02_positioning.md`
- `Primary Code Areas`: `apps/cli/src/commands/`, `crates/core/src/ledger/manager/`

## 1. Meta (版本与状态)

**版本**: 0.0.1
**核心理念**: 账本为真源 + **三位一体隔离 (Trinity Isolation)** + **Git-Flow 数据主权** + 工业级内核。

## 2. Positioning (项目定位)

**项目定位**: 个人部署在服务器上，仅供自己使用的开源个人 Wiki Markdown 笔记项目（支持 LaTeX 数学公式）。

## 3. Core Boundaries (核心边界)

以下边界用于防止核心膨胀；与任何其它章节冲突时，以本节为准。

### Phase 0: 核心验证原型 (Headless Core Verification) - [必选项]

在构建任何 UI 之前，**必须**先行构建并通过验证的纯命令行原型（Headless CLI）。

*   **目标**：验证 `Reconciliation`（和解）的鲁棒性，确保 Ledger 与大量不可靠外部写入并存时数据不丢失。
*   **功能清单**：
    1.  `init`: 初始化 Ledger、本地 repo 与 repo-scoped Projection Locator。
    2.  `watch`: 启动文件监听，能够正确处理 `vim`/`vscode`/`nano` 的保存行为（含重命名/原子写入）。
    3.  `append`: 通过 API 追加 `LedgerEntry / Ledger Facts`，验证 repo Projection Workspace 能否正确更新。
*   **验收标准**：
    *   **受控和解闭环**：`VS Code 修改 -> Watcher -> pending_fs_ops -> Stage / Commit -> Ledger -> Projection Workspace 更新` 必须稳定，无死循环。
    *   **重命名测试**：在此阶段必须解决“文件重命名被识别为删除+新建”导致的 DocId 丢失问题（实现 Inode/FileID 追踪）。

### Core MUST（核心必须）

*   **Minimal Loop (最小闭环)**: 仅提供以下核心模块，其他均为 Extension：
    *   **Authentication**: 登录与身份验证接口。
    *   **Workbench**: 标准布局容器 (Activity Bar + Side Bar + Editor + Panel)。
    *   **Command Palette**: 全局指令系统。
    *   **Editor**: Markdown 编辑器（集成 TeX 公式渲染）。
*   **Ledger Consistency (账本一致性)**: 核心 **MUST** 维护 Ledger 与 repo-scoped Projection Workspace 的一致性，覆盖写入 (Write)、同步 (Sync)、和解 (Reconciliation)、冲突处理 (Conflict Resolution) 及可恢复性。
*   **Extensibility Host**: 提供稳定的 Host Functions、Event Bus、Job Queue 与 Capability 校验机制。
*   **Linux Path Normalization (Linux 路径标准化)**:
    *   **Input Handling**: 非 Linux 平台接收到的任何路径输入 **MUST** 在第一时间转换为 Linux 风格路径（Forward Slash `/`）。
    *   **Execution**: 内部逻辑与命令执行 **MUST** 仅针对 Linux 路径格式编写。
    *   **Adapter**: 仅在最终调用 OS API 时，通过 Adapter 还原为平台原生格式。
*   **Code Modularity & Documentation (模块化与文档铁律)**：代码文件 **MUST** 按职责/API/基础设施边界保持内聚；核心模块 **SHOULD** 用中文文档注释说明架构职责与能力层；实现 plan 条款的 Rust 模块 **MUST** 遵守 `docs/plan/AGENTS.md` 的 `plan_ref` 规则。
*   **UUID-First Retrieval (UUID 优先核心约束)**:
    *   **Rule**: 后端对于任意 File/Folder/Repo 的操作，**MUST** 仅通过 UUID 完成，严禁直接使用 File Path 作为主键。
    *   **Resolution Flow**: 前端传递 `Name` -> 后端查询映射表 (`Name` -> `UUID`) -> 执行业务逻辑。
    *   **Rationale**: 确保路径变更（重命名/移动）不破坏引用一致性。
*   **Data Sovereignty (数据主权)**:
    *   **Repo Instance = File**: 明确 Repo Instance 为 `.redb` 文件。
    *   **Branch = Folder (Identity)**: 明确 Branch 为承载 Repo Instances 的文件夹 (Peer Identity)。
    *   **Constraint**: 系统 **MUST NOT** 混用 Branch 与 Repo 概念。

### Core MUST NOT（核心禁止）

*   **No Heavyweight Defaults (无默认重能力)**: 核心 **MUST NOT** 内置以下能力作为必选路径：AI 推理、Full-Text Search (全文索引)、Code Execution (代码块执行)、Batch Pipeline (批量管线)、Image Processing (图像处理)、Advanced Layout (复杂排版)。
*   **Non-Blocking Interaction (非阻塞交互)**: 核心 UI 线程 **MUST** 保持轻量。耗时操作 (Heavy Tasks) **MUST** 提交至 Job Queue，并支持 Cancel/Degrade (取消/降级)。
*   **No Proprietary Format (无私有格式)**: Projection 层（用户对于 Projection Workspace 的可见视图）**MUST NOT** 引入破坏标准 Markdown 语义的私有语法。
*   **Ignored Files Strategy (忽略策略)**：核心在外部文件摄入边界 **MUST** 先应用 `.deveignore` 与内置忽略规则；被忽略路径 **MUST NOT** 经 watcher、目录重扫或启动扫描进入 Ledger、tree projection 或 `pending_fs_ops`。受控应用内创建属于显式 authority 写入，**MUST NOT** 被 `.deveignore` 隐式拦截。

### Plugin MAY（插件可选）

* 实现并按需启用重能力（AI、索引、可执行 fenced blocks、图像/表格/图形、批处理工具等），并通过 Capability、资源配额与队列上限进行隔离。
* 扩展 UI（面板/命令/预览器/侧栏小组件）与数据能力（资产处理、外部集成），但不得破坏 Ledger 真源与导出约束。

## 4. Related Commands (本章相关命令)

* 无。

## 5. Related Configuration (本章相关配置)

*   `.deveignore`: 用于定义核心忽略的文件规则。
