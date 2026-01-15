# 02_positioning.md - 项目定位篇 (Project Positioning)

## 版本与状态 (Meta)

**版本**: 0.0.1
**状态**: 后端逻辑闭环 + 前端交互定义完整 + 数据/安全强化落地。
**核心理念**: 账本为真源 + **三位一体隔离 (Trinity Isolation)** + **Git-Flow 数据主权** + 工业级内核。

## 项目定位 (Positioning)

**项目定位**: 个人部署在服务器上，仅供自己使用的开源个人 Wiki Markdown 笔记项目（支持 LaTeX 数学公式）。

## 核心边界（最高优先级）

以下边界用于防止核心膨胀；与任何其它章节冲突时，以本节为准。

### Phase 0: 核心验证原型 (Headless Core Verification) - [必选项]

在构建任何 UI 之前，**必须**先行构建并通过验证的纯命令行原型（Headless CLI）。

*   **目标**：验证 `Reconciliation`（和解）的鲁棒性，确保 Ledger 与大量不可靠外部写入并存时数据不丢失。
*   **功能清单**：
    1.  `init`: 初始化 Ledger 和 Vault。
    2.  `watch`: 启动文件监听，能够正确处理 `vim`/`vscode`/`nano` 的保存行为（含重命名/原子写入）。
    3.  `append`: 通过 API 追加 Ops，验证 Vault 能否正确更新。
*   **验收标准**：
    *   **双向同步闭环**：`VS Code 修改 -> Watcher -> Ledger -> Vault 更新` 必须稳定，无死循环。
    *   **重命名测试**：在此阶段必须解决“文件重命名被识别为删除+新建”导致的 DocId 丢失问题（实现 Inode/FileID 追踪）。

### Core MUST（核心必须）

* 只提供“登录界面 + **Workbench 布局 (Activity Bar/Side Bar)** + **Command Palette** + Markdown 撰写体验（含 TeX 公式渲染）”的最小闭环。
* 维护 Ledger/Vault 的一致性闭环：写入、同步、和解、冲突处理、可恢复性与可观测性。
* 提供稳定的 Host Functions、事件总线、作业队列与 Capability 校验，用于承载插件扩展。
* 在非linux端使用时，先将命令和路径转化为 linux 命令与路径，再按照原定linux程序进行操作。
* **Cross-Platform Normalization (跨平台规范化)**：
    *   **MUST Form-First**: 所有输入转为 Linux 风格路径。
    *   **Shell Adapter**: OS API 调用前还原为原生格式。
* **Code Modularity & Documentation (模块化与文档铁律)**：所有代码文件 **MUST** 保持高度模块化（单一职责）；文件头部 **MUST** 包含中文注释块，明确说明：(1) 本文件的架构作用；(2) 核心功能清单；(3) 是否属于核心必选路径 (Core MUST) 或可选扩展 (Optional)。

### Core MUST NOT（核心禁止）

* 不内置任何重能力为默认必选路径：AI、全文索引、计算/执行型代码块、批量导入导出管线、图像处理、复杂渲染/排版等。
* 不让任何重任务阻塞交互：核心 UI 路径必须恒轻；重任务必须走作业队列并可取消/可降级。
* 不引入私有格式污染导出：对用户可见的文本投影必须保持标准 Markdown 语义。
* **Ignored Files Strategy (忽略策略)**：对于 Vault 中无法解析或过大的非 Markdown/非 Asset 文件（如编译产物、系统临时文件），核心 **MUST** 依据 `.deveignore` 或内置规则直接忽略，**MUST NOT** 尝试将其摄入 Ledger，避免核心膨胀或阻塞。

### Plugin MAY（插件可选）

* 实现并按需启用重能力（AI、索引、可执行 fenced blocks、图像/表格/图形、批处理工具等），并通过 Capability、资源配额与队列上限进行隔离。
* 扩展 UI（面板/命令/预览器/侧栏小组件）与数据能力（资产处理、外部集成），但不得破坏 Ledger 真源与导出约束。

## 本章相关命令

* 无。

## 本章相关配置

*   `.deveignore`: 用于定义核心忽略的文件规则。
