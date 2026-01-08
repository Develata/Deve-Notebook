# 📑 Deve-Note plan - 系统架构

**版本**：0.0.1
**状态**：后端逻辑闭环 + 前端交互定义完整 + 数据/安全强化落地。
**核心理念**：账本为真源 + ID 化 VFS + 工业级内核 (Industrial Kernel) + 沉浸式驾驶舱 (Immersive Cockpit)。

**项目定位**：个人部署在服务器上，仅供自己使用的开源个人 Wiki Markdown 笔记项目（支持 LaTeX 数学公式）。

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

## 核心边界（最高优先级）

以下边界用于防止核心膨胀；与任何其它章节冲突时，以本节为准。

**Core MUST（核心必须）**：

* 只提供“侧边栏导航 + 命令系统 + Markdown 撰写体验（含 TeX 公式渲染）”的最小闭环。
* 维护 Ledger/Vault 的一致性闭环：写入、同步、和解、冲突处理、可恢复性与可观测性。
* 提供稳定的 Host Functions、事件总线、作业队列与 Capability 校验，用于承载插件扩展。

**Core MUST NOT（核心禁止）**：

* 不内置任何重能力为默认必选路径：AI、全文索引、计算/执行型代码块、批量导入导出管线、图像处理、复杂渲染/排版等。
* 不让任何重任务阻塞交互：核心 UI 路径必须恒轻；重任务必须走作业队列并可取消/可降级。
* 不引入私有格式污染导出：对用户可见的文本投影必须保持标准 Markdown 语义。
* **Ignored Files Strategy (忽略策略)**：对于 Vault 中无法解析或过大的非 Markdown/非 Asset 文件（如编译产物、系统临时文件），核心 **MUST** 依据 `.deveignore` 或内置规则直接忽略，**MUST NOT** 尝试将其摄入 Ledger，避免核心膨胀或阻塞。

**Plugin MAY（插件可选）**：

* 实现并按需启用重能力（AI、索引、可执行 fenced blocks、图像/表格/图形、批处理工具等），并通过 Capability、资源配额与队列上限进行隔离。
* 扩展 UI（面板/命令/预览器/侧栏小组件）与数据能力（资产处理、外部集成），但不得破坏 Ledger 真源与导出约束。


---

## Phase 0: 核心验证原型 (Headless Core Verification) - [新增必选项]

在构建任何 UI 之前，**必须**先行构建并通过验证的纯命令行原型（Headless CLI）。

*   **目标**：验证 `Reconciliation`（和解）的鲁棒性，确保 Ledger 与大量不可靠外部写入并存时数据不丢失。
*   **功能清单**：
    1.  `init`: 初始化 Ledger 和 Vault。
    2.  `watch`: 启动文件监听，能够正确处理 `vim`/`vscode` 的保存行为（含重命名/原子写入）。
    3.  `append`: 通过 API 追加 Ops，验证 Vault 能否正确更新。
*   **验收标准**：
    *   **双向同步闭环**：`VS Code 修改 -> Watcher -> Ledger -> Vault 更新` 必须稳定，无死循环。
    *   **重命名测试**：在此阶段必须解决“文件重命名被识别为删除+新建”导致的 DocId 丢失问题（实现 Inode/FileID 追踪）。

---

## 第一章：界面设计哲学 (UI Design Philosophy) - [新增核心]

### 1. The "Cockpit" Concept (驾驶舱概念)

* **信息分层**：
	* **L1 (Focus)**：编辑区是绝对中心，无干扰。
	* **L2 (Context)**：侧边栏（大纲、文件树）提供导航。
	* **L3 (Meta)**：底部状态栏显示“和解状态”（Sync/Watcher）、Git 分支、字数统计。
	* **L4 (Floating)**：`Cmd+K` 命令面板和悬浮工具栏，按需出现。

* **键盘优先 (Keyboard First)**：
	* 所有 UI 操作（切换侧边栏、分屏、搜索、跳转）必须有快捷键。
	* 模仿 Vim/VS Code 的操作逻辑，减少鼠标移动。



### 2. Reactive Projection (响应式投影)

* **即时反馈**：当后端 Watcher 检测到磁盘上的文件被 VS Code 修改时，前端编辑器不应“刷新页面”，而应通过 **Loro 的 Diff 补丁** 平滑地更新内容，并弹出一个非侵入式的 Toast 提示：“已合并外部修改”。
* **乐观 UI (Optimistic UI)**：用户输入立即上屏，WebSocket 同步在后台悄悄进行。如果网络失败，状态栏图标变红，但编辑不中断。

### 3. Mathematical Aesthetics (数学美学)

* **排版**：默认集成 **KaTeX** (快速) 或 **MathJax 3** (精确)，支持复杂的数学公式渲染。
* **字体**：预设适合代码和数学公式的等宽字体 (如 JetBrains Mono, Fira Code) 和衬线字体 (如 Merriweather)。

### 4. Ledger/VFS/Security Principles (真源与安全)

* **Ledger is Truth（真源不变量）**：
	* 权威状态 MUST 仅由 Ledger 表达（Ops Log + Snapshot）。
	* **Patch Semantics**：外部编辑器对 Vault 的修改被视为一次 **"Patch"**（补丁）。核心 **MUST** 将其通过 Diff 算法转化为 Ops 合并入 Ledger，而 **MUST NOT** 简单地以文件内容覆盖账本。
	* Vault/Markdown MUST 可由 Ledger 重建；不得存在“仅在 Vault 才存在、且无法由 Ledger 推导”的用户可见事实。

* **ID-Based VFS（标识不变量）**：
	* 所有实体（文档/资产）MUST 以 `DocId (UUID)` 为主键。
	* 路径 MUST 仅作为显示属性；重命名/移动 MUST 只修改 Path Mapping。
	* 内部链接 MUST 解析为 DocId；导出时 MAY 转为相对路径链接（保持标准 Markdown）。

* **Capability-Based Security（权限不变量）**：
	* 脚本/插件 MUST 声明能力清单（网络域名、FS 路径、env 变量白名单）。
	* **Manifest Enforcement**：若插件未在 manifest 中声明某权限，运行时请求该权限 **MUST** 直接拒绝（Panic/Error），不予弹窗询问。
	* Host Functions MUST 执行最小权限校验与审计（default deny）。

### 5. 体验取舍与兼容性 (Inspiration & Compatibility)

* **风格参考**：UI 视觉与版式借鉴语雀的清爽阅读感，但保持开源可自定义主题。
* **标准 Markdown 导出**：导出/投影坚持通用 GFM/Frontmatter，不添加私有标记或语雀式特有格式；富文本元数据仅存于 Ledger。
* **效率取向**：插件与交互倾向 SilverBullet 的轻量/实用，避免臃肿；默认装载最小可用集。
* **导航体验**：侧边栏结构参考 VitePress（分组/层级清晰），命令/快捷键呼出文件列表参考 SilverBullet 弹出式搜索。

---

## 第二章：UI 架构与组件系统 (UI Architecture) - [新增核心]

前端采用 **Leptos (Signals)** + **Tailwind CSS**，构建一套高性能组件库。

### 1. 布局系统 (The Layout Engine)

采用 **"Resizable Slot" (可缩放插槽)** 布局：

* **Left Slot**: 文件树（核心）/ 双向链接图谱 (Mini Graph, 插件可选)。
* **Main Slot**: 多标签页 (Tabs) 编辑器 / 分屏 (Split View)。
* **Right Slot**: 大纲 (TOC) / 属性面板 (Metadata) / 插件面板。
* **Bottom Slot**: 日志输出（核心）/ 终端面板 (Terminal, 插件可选)。
* **Internationalization (i18n)**: 核心 UI 文本 **MUST** 使用 `leptos_i18n` 进行管理，支持编译时类型检查；默认提供 En/Zh-CN，根据浏览器自动协商。
* **特性**：所有面板状态（宽度、折叠）持久化存储在 Redb 的 `ui_state` 表中，重启后完全恢复；侧边栏分组/层级可配置，风格靠近 VitePress 的导航结构。

### 2. 编辑器内核 (The Editor Kernel)

不仅仅是一个 `<textarea>`，而是一个分层渲染器：

* **Layer 1 (Input)**: `ContentEditable` 或 CodeMirror 6，负责捕获输入。
* **Layer 2 (State)**: 绑定 Loro CRDT 状态，处理并发冲突。
* **Layer 3 (Render)**：
	* **Block Mode（插件可选）**：类似 Notion，支持拖拽块。
	* **Source Mode**：纯 Markdown 源码模式（配合 Monaco/CodeMirror 高亮）。
	* **Live Preview**：类似 Obsidian/Milkdown，源码即所见。

* **技术选型（轻/重双模式）**：
	* **默认（轻核心）**：以 CodeMirror 6 的 Source Mode 作为主编辑体验，确保性能、兼容与导出稳定。
	* **可选（重扩展）**：Live Preview / 富交互编辑器作为可选模块（feature/插件）启用，推荐 **Milkdown (基于 Prosemirror)**，插件生态丰富。

**Markdown 基线能力（核心）**：标题/段落/粗斜体/删除线、无序/有序/任务列表、引用、代码块（语言高亮）、表格、分隔线、链接/图片、脚注、行内/块级数学；快捷键与命令面板覆盖常用格式；粘贴/拖拽图片自动入库并生成 DocId 引用；撤销/重做。

**可选增强（插件/feature）**：块拖拽/上移下移、复杂可视化（图谱/时光轴）、PDF/批处理导出、计算/执行型能力等；默认不进入核心必选路径。

**数学体验细节**：支持 `$...$` 行内与 `$$...$$` 块级模式；输入 `$$` 自动切块；渲染 KaTeX 优先、可切 MathJax；公式错误高亮与 fallback 文本；公式块支持一键复制为 LaTeX/导出 SVG/PNG；离线打包 KaTeX 资源；大文档中按需渲染（虚拟化）。

**LaTeX 渲染约定（标准 Markdown）**：

* `$...$` 作为行内公式直接渲染。
* `$$...$$` 作为行间公式直接渲染。
* **禁止裸 `$` 字符**：在普通文本中 `$` MUST 以 `\$` 形式出现；仅当 `$` 用作公式定界符（`$...$` / `$$...$$`）时允许裸写。
* **校验与修复**：编辑器 SHOULD 提供 lint 提示或一键修复，将非公式语境的 `$` 自动替换为 `\$`。
* 导出/投影仍保持原始 Markdown 语法（不插入私有标记）。

**性能优先原则**：任何“重能力”（Live Preview、图谱、AI、PDF）不得成为核心必选依赖；核心路径必须在低配机器与大文档下保持可用。



### 3. 可视化系统 (Visualization System)

本小节整体属于 **Plugin MAY**（默认关闭）；核心只提供 UI 插槽、命令注册与能力/资源隔离。

* **Global Graph (全域图谱，插件可选，默认关闭)**：
	* 使用 **Rust -> Wasm** 编译的力导向图引擎 (Force-directed Graph)，支持 10,000+ 节点流畅渲染（Canvas/WebGL）。
	* 支持按 Tag、文件夹颜色聚类。

* **Time Travel Slider (时光轴，插件可选，默认关闭)**：
	* UI 底部的一条交互式热力图。
	* 颜色深浅代表修改频率。
	* 拖动滑块，编辑器内容按需回放（基于 Loro 历史；需要分段加载/降级以避免卡顿与内存峰值）。



### 4. 命令面板 (The Commander)

* 呼出：`Cmd/Ctrl + K`。
* 功能：
	* **导航**：`Go to file...`（核心提供命令与路由；全文索引/模糊搜索由插件可选提供，例如 Tantivy）。
	* **操作**：`Toggle Dark Mode`、`Git Push`、`Export PDF`（由插件注册命令；核心只提供受控执行与能力校验）。
	* **插入**：`Insert Math Block`、`Insert Date`。
	* **快速文件列表**：参考 SilverBullet，提供即时文件/笔记列表弹出，支持键盘过滤/跳转。



---

## 第三章：统一后端架构 (The Vibranium Backend)

*(继承既有核心，确保逻辑闭环)*

* **存储（真源 + 投影）**：
	* **双存储**：`/data/ledger` 保存 append-only 二进制日志分段（`log_001.bin`）+ 周期性 Snapshot（`snap_v100.bin`）。
	* `/data/vault` 为 Markdown 投影目录（投影非真源）；支持延迟写/按需写；外部写入 Vault MUST 经由和解转换为 Ops 并追加到 Ledger。

* **索引与检索**：
	* Redb/Sled 维护元数据、UI State、`DocId <-> Path` 映射。
	* Tantivy 全文检索索引为插件可选能力；启用时强调资源上限与节流，默认不进入核心必选路径。

* **同步/流控**：
	* **输入**：客户端 Ops、服务端 Ops、Snapshot、订阅/心跳。
	* **输出**：增量 Ops 推送或 Snapshot 下发；同步状态可观测（落后/追平/失败）。
	* **约束**：
		* MUST 有背压：所有收发队列有硬上限；超限 MUST 触发降级（断开/改发 Snapshot/拒绝低优先级任务）。
		* MUST 支持离线：断网期间写入本地 Ops；重连后上推并对齐。
		* SHOULD 分级：轻微落后走 Ops replay，严重落后走 Snapshot。
	* **失败语义**：网络失败不阻塞编辑；重连后最终一致；连续失败 MUST 提供可见告警与手动重试入口。

* **和解与外部修改**：
	* **输入**：Vault 文件变更事件（Notify）+ 变更内容（或 diff）。
	* **输出**：对应 Ops（追加写入 Ledger）+ 广播“外部修改已合并”。
	* **约束**：
		* MUST 幂等（同一改动不重复吸收）；MUST 防死循环（Sentinel Lock）。
		* **MUST 识别重命名**：利用 **Inode 追踪 (Linux/macOS)** 或 **File ID (Windows)** 识别文件移动/重命名。
		* **MUST Fallback Identity**：当 Inode 失效（如 Git Pull 导致重建）时，**MUST** 检查 Frontmatter 中的 `uuid` 字段或计算内容 Hash 来重新关联 DocId，防止“重命名风暴”导致的历史丢失。
		* **MUST 防抖 (Debounce)**：外部写入后应有静默期（如 200ms），等待文件写入稳定（防 Editor 临时文件/原子保存干扰）。
	* **失败语义**：无法解析/冲突不可合并时 MUST 明确提示并保留原内容（不静默丢失）。

* **运行时安全（插件/脚本）**：
	* **输入**：插件包（Rhai/Extism）+ 能力清单 + 命令/事件触发。
	* **输出**：受控 Host API 调用结果（读写/网络/搜索/UI 插槽）。
	* **约束**：default deny；Host Functions MUST 做 Capability 校验、路径/域名白名单与审计。
	* **失败语义**：越权调用 MUST 失败并可诊断；插件崩溃 MUST 隔离，不影响核心编辑路径。

### 认证与登录 (Auth & Login)

* **12-Factor Auth (环境驱动安全)**：
    * **No Init UI**：严禁提供“首次启动管理员设置”界面。所有敏感配置（管理员密码/Secret Key）**MUST** 通过环境变量 (`DEVE_NOTE_PASSWORD_HASH`) 或 `.env` 文件注入。
    * **无状态**：服务 **MUST** 符合 12-Factor App 原则，不依赖本地状态存储凭据。
* **单用户配置**：支持管理 token（如 personal access token）以便脚本或 CLI 使用。
* **2FA 可选**：支持 TOTP（本地生成）或 Passkey；可在配置中开启/关闭；本地部署默认关闭但预留入口。
* **安全基线**：强制 HTTPS；登录表单抗 CSRF；错误提示不暴露账号存在性；限制暴力尝试（速率限制、锁定或简单验证码可选）。
* **会话管理**：短期访问令牌 + 长期刷新令牌；可选“记住本机”延长会话；支持一键失效所有会话。
* **体验细节**：键盘友好（回车提交、焦点管理）、显示/隐藏密码、加载/禁用态、移动端软键盘不遮挡；亮/暗主题适配。
* **离线场景**：离线仅允许已登录设备访问本地数据；重连时若会话过期需重新校验本地存储的凭据。

---

## 第四章：数据完整性与灾备 (Integrity & Recovery)

* **Append-only**：所有写操作追加账本，不改历史；后台定期 compaction，合并旧日志为 Snapshot，回收过期 Ops。
* **投影策略**：Markdown 投影非真源，正常从 Ledger 渲染（可配置 1-2 秒延迟或特定事件触发写出）；外部编辑 Vault 时必须经“和解”回写 Ledger。
* **恢复场景**：
	* Markdown 误删 -> 从 Ledger 全量重建。
	* Ledger 损坏 -> 反向导入 Markdown 生成新 Ledger（保现状，失部分历史）。
	* 客户端错乱 -> Hard Reset，丢弃本地 DB，重拉 Snapshot。

* **Portable Ledger Export (灾难恢复/逃生舱)**: 
    * 提供 `deve-note export-ledger --format=jsonl` 命令，将二进制 Ledger 导出为人类可读的 JSON Lines 格式（Ops 或纯文本历史）。
    * **目的**：确保即使不再使用 Deve-Note 软件，用户的数据（含完整历史版本）也能以通用格式被解析和迁移。这消除了“二进制格式锁死”的恐惧。

---

## 第五章：数据流与交互 (Interaction Flows)

### 场景一：外部编辑器协同 (The "Alt-Tab" Flow)

1. 用户在 VS Code 中打开 `/data/vault/thesis.md`，修改了一段公式，保存。
2. **后端**: `Notify` 捕获 -> **Debounce (防抖)** -> **Inode 检查 (确认非重命名)** -> 检查锁 -> 读取文件 -> `Diff` -> 生成 Ops -> 存入 Redb。
3. **推送**: 后端通过 WebSocket 推送新的 Ops 到 Deve-Note 客户端。
4. **前端**:
	* 编辑器光标位置**保持不变**。
	* 修改的内容在视图中**平滑更新**（Flash Highlight 效果）。
	* 右下角 Toast 提示: *"External change merged (2ms ago)"*。



### 场景二：数学公式编写 (The Math Flow)

1. 用户输入 `$$`。
2. **前端**: 立即切换为“公式编辑块”，启用等宽字体。
3. **输入**: `\int_{a}^{b} x^2 dx`。
4. **预览**: 编辑块下方实时显示 KaTeX 渲染结果。
5. **完成**: `Ctrl+Enter` 跳出，源码折叠，只显示渲染后的 SVG 图片（点击可再次编辑）。

### 场景三：Git 同步 (The Git Flow)

1. 用户点击状态栏的 "Git" 图标，或 `Cmd+K` -> `Git Sync`。
2. **前端**: 弹窗显示差异统计 "+12 / -5"。
3. **权限检查**: Commander 校验能力清单是否包含 `sys.git` 且远端仓库在白名单。
4. **调用**: 触发 Rhai 脚本 `git_sync.rhai`，Host Functions 只允许推送到配置的 `GITHUB_REPO`，禁止改 remote。
5. **后端**: 执行 `git add .` -> `git commit` -> `git push`。
6. **反馈**: 状态栏转圈 -> 变绿 "Synced"。

---

## 第六章：技术栈清单 (The Full Stack)

| 层次 | 核心技术 | 选型理由 |
| --- | --- | --- |
| **语言** | Rust (2024) | 全栈统一。 |
| **前端框架** | **Leptos v0.7** | 信号驱动，性能极致，无 Virtual DOM 开销。（*注：需优先验证与 CodeMirror 的集成流畅度*） |
| **UI 组件** | **Tailwind CSS** | 原子化 CSS，配合 Shadcn-UI (Leptos port) 实现一致性设计。 |
| **国际化** | **leptos_i18n** | 编译时校验的 i18n 方案，零运行时开销。 |
| **编辑器** | **CodeMirror 6（默认）/ Milkdown（可选）** | 轻核心保证性能与导出稳定；重编辑器按需启用。（*注：Leptos 绑定层需自行封装或原型验证*） |
| **图标库** | **Lucide Icons** | 统一、现代的 SVG 图标集。 |
| **图谱渲染（可选）** | **Pixi.js** 或 **Cosmic-Graph (Rust)** | WebGL 加速的图可视化。 |
| **存储** | **Redb/Sled** | 纯 Rust 嵌入式 DB，索引 Ledger/Path 映射。 |
| **搜索（可选）** | **Tantivy** | 全文检索引擎（默认不进入核心必选路径）。 |
| **同步/流控** | **Axum + Tower** | 背压、限流、超时、熔断。 |
| **和解** | **Notify + Dissimilar** | 文件监听与 Diff。 |
| **构建** | **Tauri v2** | 跨平台外壳。 |
| **插件** | **Rhai + Extism** | Wasm/脚本引擎，能力受控。 |

**核心数据结构**：

* `struct DocId(Uuid);`
* `struct LedgerEntry { op: Vec<u8>, timestamp: i64, user_id: Uuid }`
* `struct CapabilityManifest { allow_net: Vec<String>, allow_fs_read: Vec<PathBuf>, allow_fs_write: Vec<PathBuf>, allow_env: Vec<String> }`

**能力清单示例**：

```toml
[capabilities]
allow_net = ["api.github.com"]
allow_fs_read = ["/notes/public"]
allow_fs_write = ["/notes/public"]
allow_env = ["GITHUB_TOKEN"]
```

---

### Markdown 兼容性与回归清单 (Compatibility Checklist)

* **导出原则**：所有导出/投影必须是通用 Markdown（GFM + YAML Frontmatter），不输出任何私有语法；无法表达的富文本信息只保存在 Ledger。
* **语法基线**：标题、段落、列表/任务列表、引用、表格、代码块（fenced + language info）、链接、图片、脚注、行内/块级数学、Frontmatter。
* **链接约定**：运行时内部链接以 DocId 为真源（如 `doc://<uuid>` 或 `[[title]]` 由核心解析为 DocId）；导出时可选择“解析为相对路径 Markdown 链接”或“保留 wiki link（可配置）”。
* **资产约定**：运行时可用 `asset://<uuid>`；导出时必须落为标准 Markdown 图片/链接引用（相对路径或可配置的 `base_url`）。
* **回归用例**：维护一组 fixture 文档（数学/表格/脚注/代码块/大文件/大量链接/大量图片），CI 对渲染与导出做快照对比。

### 性能预算与极致瘦身 (Performance & Footprint)

* **前端默认策略**：首屏只加载编辑器 + 导航；图谱/时光轴/AI/插件 UI 按需加载；大文档渲染与数学渲染必须虚拟化。
* **体积控制**：默认 KaTeX（避免 MathJax 体积与运行时开销）；Wasm/JS 分包（code splitting）+ tree-shaking；生产构建启用 LTO/strip；按 feature flags 裁剪非核心模块。
* **内存预算（目标）**：空闲态 < 150MB；打开 10k 行 Markdown < 300MB；公式/图谱渲染不得常驻全量缓存，采用 LRU/分页。
* **数学渲染策略**：仅渲染可视区域公式；渲染结果按块缓存（LRU + 硬上限）；滚动时延迟渲染，避免尖峰卡顿。
* **同步/索引预算**：WebSocket 收发队列长度可配置且有硬上限；索引能力仅在启用相应插件/feature 时生效，且索引更新必须节流/批处理（避免每次保存都触发重建）；若使用 Tantivy，索引支持 mmap/分段合并并设置内存上限；后台任务（投影/索引/压缩）必须限并发。
* **Low-Spec Server Profile (512MB RAM)**: 
    *   **CSR Only**: 在低配模式下 **MUST** 自动禁用服务端渲染 (SSR)，仅下发静态 HTML + Wasm，将渲染负载转移至客户端。
    *   **No Search Index**: 默认关闭 Tantivy 或限制其堆内存 < 50MB。
    *   **Snapshot Pruning (快照裁剪)**：保留最近 10 个快照 + 每日/每周归档，旧快照激进清理，防止磁盘/索引膨胀。
    *   **Cross-Compilation**: 明确告知用户，512MB 服务器仅用于**运行**，构建 (Build) 必须在高性能机器或 CI 上完成。

* **Standard Server Profile (1GB+ RAM)**:
    *   **Full Features**: 支持全功能开启，包括 SSR（服务端渲染）、Tantivy 全文检索、图谱分析与 LaTeX 预渲染缓存。
    *   **推荐配置**：1GB RAM 即可流畅运行完整版（Docker 限制建议设为 1024M）；2GB+ RAM 可支持更激进的缓存与大文件索引。

### Server Configuration Profiles & Feature Flags (配置清单)

系统提供精细的配置开关，以适配从树莓派 (Low-Spec) 到高性能服务器 (Standard) 的不同环境。

| 环境变量 / 配置项 | 默认值 (Standard) | 512MB 推荐值 (Low-Spec) | 功能影响说明 |
| :--- | :--- | :--- | :--- |
| `DEVE_PROFILE` | `standard` | `low-spec` | **一键预设**。设置为 `low-spec` 时，会自动覆盖下表的默认值为推荐值。 |
| `FEATURE_SSR` | `true` | `false` | **服务端渲染**。`false` = 仅下发 HTML 骨架，浏览器加载 WASM 后渲染。极大降低服务器内存峰值，但首屏加载稍慢。 |
| `FEATURE_SEARCH` | `true` | `false` | **全文搜索 (Tantivy)**。`false` = 仅支持文件名搜索。禁用 Tantivy 引擎可节省 50-150MB 堆内存与 CPU 突发占用。 |
| `FEATURE_GRAPH` | `true` | `false` | **全域图谱后台分析**。`false` = 服务器不构建引用图谱数据，前端图谱可视化功能将不可用。 |
| `MEM_CACHE_MB` | `128` | `32` | **内存缓存上限**。用于图片缩略图、LaTeX 渲染结果的 LRU 缓存大小。 |
| `CONCURRENCY` | `4` | `1` | **后台并发度**。控制索引构建、压缩、导入等重任务的最大并发线程数。 |
| `SNAPSHOT_DEPTH` | `100` | `10` | **快照保留深度**。保留最近多少个版本的快照。减少数量可显著降低 Redb 索引大小。 |

* **Ledger 与资产 I/O**：读写必须流式（streaming），避免整文件载入内存；Snapshot/日志段可选 zstd 压缩（以段为单位），不破坏 append-only 语义。
* **插件/AI 资源限制**：插件与 AI 作业统一进入作业队列，具备超时/取消；默认并发低且可配置；任何扩展不得阻塞编辑主线程。

---

## 第七章：插件运行时与资产模型 (Plugin Runtime & Assets)

* **插件 ABI 与生命周期**：
	* **输入**：插件清单（名称/版本/能力/入口）+ install/activate/deactivate + 事件订阅。
	* **输出**：命令注册、事件回调、可选 UI 插槽扩展。
	* **约束**：插件只能订阅已声明事件；核心事件流 MUST 可取消/可限流。
	* **失败语义**：加载失败/版本不兼容 MUST 明确报错；不得影响核心编辑路径。

* **Host Functions（受控 API）**：
	* **输入**：按 DocId 的读写、搜索、命令注册、网络请求。
	* **输出**：确定性结果或结构化错误。
	* **约束**：所有调用 MUST 经过 Capability 校验与白名单；敏感调用 MUST 可审计。
	* **失败语义**：越权 MUST 失败；错误 MUST 可定位到插件与能力项。

* **Plugin RPC Bridge (前端-后端通信)**：
    * 插件通常包含前端 UI（JS/Wasm）与后端逻辑（Rhai/Wasm）。
    * **机制**：提供标准化的 `client.call("plugin_id", "fn_name", args)` 方法，通过 WebSocket 透明转发至后端插件运行时。
    * **安全**：RPC 调用遵循同样的 Capability 检查；前端侧只能发起调用，后端侧执行实际的敏感操作（如文件读写/Git Push）。

* **资源配额与隔离**：
	* **输入**：插件任务与执行请求。
	* **输出**：任务结果或超时/取消。
	* **约束**：CPU/内存/并发/超时 MUST 可配置且默认保守；崩溃隔离 MUST 生效。
	* **失败语义**：超时/取消 MUST 可恢复；反复异常 MAY 自动停用插件。

* **资产模型与导出**：
	* **输入**：资产写入（图片/附件/渲染产物）、引用创建、导出请求。
	* **输出**：运行时 `asset://<uuid>`；导出为标准 Markdown 图片/链接引用。
	* **约束**：资产 MUST 有 DocId；导出 MUST 不写入私有格式；链接 MAY 选相对路径或 `base_url`。
	* **失败语义**：资产缺失 MUST 可诊断（能定位缺失 DocId 与引用位置）。

* **产物写回（计算/渲染类）**：
	* **输入**：计算/渲染结果（二进制或文本）。
	* **输出**：写入资产存储（生成 DocId）并返回可插入的 Markdown 引用。
	* **约束**：写回 MUST 原子化（要么成功并可引用，要么失败并可重试）。
	* **失败语义**：失败 MUST 不污染文档正文（不插入坏引用）。

### 可执行代码块扩展 (RStudio/Knitr-style Fenced Blocks)

* **输入**：形如 ```` ```{latex} ```` / ```` ```{r} ```` / ```` ```{python} ```` 的 fenced block +（可选）参数。
* **输出**：纯文本、Markdown 片段或资产 DocId；核心回写为“紧邻输出块”或“资产引用”。
* **约束**：
	* 核心只识别与路由；未安装插件 MUST 按普通代码块显示。
	* 执行 MUST 进入作业队列，并受能力清单与配额约束（超时/取消/并发上限）。
* **失败语义**：失败 MUST 生成可折叠日志与错误摘要；不得改写原 fenced block；缓存命中/失效 MUST 可解释。

---

## 第八章：AI 与计算扩展 (AI & Compute Extensions)

* **AI 抽象层**：
	* **输入**：provider/模型选择 + 提示词/上下文 +（可选）工具调用请求。
	* **输出**：流式文本、结构化函数调用、或资产 DocId（图像/渲染产物）；不污染 Markdown 导出格式。
	* **约束**：接口 MUST provider-agnostic（`AiClient/ModelRegistry`）；MUST 支持流式与速率限制。
	* **失败语义**：失败 MUST 可恢复（重试/切换 provider）且可观测（状态栏/日志）。

* **安全与能力绑定**：
	* **输入**：AI 访问网络/文件/工具的请求。
	* **输出**：允许/拒绝的决策与审计记录。
	* **约束**：默认无权访问本地文件；网络域名、文件读写、工具调用必须显式授权。
	* **失败语义**：拒绝 MUST 明确到具体能力项；不得静默降权。

* **计算/渲染作业**：
	* **输入**：长任务（AI、渲染、索引等）。
	* **输出**：结果（文本/资产 DocId）+ 可折叠日志。
	* **约束**：MUST 进入作业队列（超时/取消/重试/并发上限）。
	* **失败语义**：失败 MUST 可重试且不阻塞编辑主线程。

* **隐私与遥测**：默认关闭；开启时 MUST 声明收集字段与用途，并提供关闭开关。

---

## 第九章：多端发布与封装策略 (Cross-Platform Delivery)

* **单内核，多外壳**：核心逻辑 (CRDT/Ledger、VFS、权限、同步协议、加密) 均在纯 Rust crate 中实现，无平台绑定；编译为 wasm (web)、静态/动态库 (桌面/移动) 共用同一套代码。
* **UI 复用**：Leptos + Wasm 作为 UI/状态层；组件保持平台无关，平台差异通过极薄适配层注入（文件对话框、剪贴板、通知、窗口）。
* **外壳适配**：
  * Desktop (Windows/Linux/macOS)：Tauri v2 提供窗口/菜单/托盘/文件访问。
  * Web：纯浏览器/PWA，直接使用 wasm 版本核心 + Leptos。
  * Mobile (Android/iOS)：Tauri Mobile / WebView 外壳 + Rust 核心静态库；
    * **UI 策略**：不直接移植桌面版“驾驶舱”。移动端 **MUST** 采用简化的 **"Reader/Capture Mode"**，专注于阅读和快速记录，避免复杂的快捷键与多面板交互。
    * 必要时用 Capacitor/FFI 提供系统权限、文件选择、通知。
* **存储适配**：定义 `Storage` trait，桌面用 sqlite/Redb，移动用 sqlite/IndexedDB，Web 用 IndexedDB；VFS 仍以 `DocId` 为主键，路径为属性。
* **网络适配**：定义 `NetClient` trait，统一 WebSocket/HTTP；Web/移动使用浏览器 API，桌面/CLI 使用 reqwest/tungstenite。
* **权限与能力**：能力清单由核心校验，外壳负责请求 OS 权限（文件/网络/通知等）；Host Functions 仍做路径和域名白名单。
* **构建与 CI**：单仓多目标 Workspace；CI matrix 覆盖 windows/linux/macos + wasm32-unknown-unknown + aarch64-apple-ios + aarch64-linux-android；产出 Tauri bundle、PWA、Android/iOS 包。

* **极致压缩开关**：默认不编译/不加载图谱与时光轴等重模块（可通过 feature/插件启用）；AI 相关依赖与 UI 组件默认按需加载；移动端与 Web 优先选择轻量渲染路径。

---

## 第十章：开源发布与社区运营 (Open Source Playbook)

* **许可证**：推荐 MIT 或 Apache-2.0；确保依赖链兼容，附带第三方 NOTICE。
* **贡献流程**：提供 CONTRIBUTING、Code of Conduct；规范 PR 模板、Issue 模板；强制 CI（lint/test）通过后合并。
* **发行节奏**：主分支保持可发布；按月发布 beta，按季度发布 stable；发布说明含 Breaking/Features/Fixes；维护变更日志。
* **多端发行物**：
  * 桌面：Tauri bundle（Win/MSI、macOS dmg、AppImage/DEB/RPM）。
  * Web：PWA + wasm 包；提供线上 Demo 与 self-host 部署指引。
  * 移动：Android APK/AAB，iOS TestFlight；同一 Rust 核心。
* **包管理/镜像**：如需 CLI/库发布，推 crates.io；提供 Docker 镜像（带健康检查）；生成 SBOM 供用户审计。
* **隐私与遥测**：默认关闭遥测，提供显式 opt-in；如启用，记录最低限度的匿名性能/崩溃数据，并公开数据字段与用途。
* **安全响应**：提供安全邮件/私报渠道；设立 embargo 流程与补丁发布节奏。

### 获取与安装（面向用户）

本节描述“发布到 GitHub 后，用户如何拉取/安装 Deve-Note 与插件”。该部分属于发布规范：实现可以逐步到位，但发布物的命名与目录约定 SHOULD 稳定。

* **GitHub Releases（二进制直装）**：
	* 每个版本 SHOULD 在 GitHub Releases 提供：服务端二进制（server/cli）、桌面安装包、`SHA256SUMS`（或等价校验文件）。
	* 用户下载后 SHOULD 校验哈希再运行；Windows 可用 `CertUtil -hashfile <file> SHA256`，Linux/macOS 可用 `sha256sum`。
	* 版本号与构建信息 SHOULD 可在 `deve-note --version`（或等价命令）中查询，便于排障。

* **Docker 镜像（自部署推荐）**：
	* 镜像 SHOULD 发布到 GitHub Container Registry（GHCR），并提供稳定标签：`latest`、`vX.Y.Z`。
	* 镜像启动时 MUST 支持将数据目录挂载出来（至少 `ledger` 与 `vault`），以保证升级/迁移安全。
	* `docker compose` 示例（路径与镜像名以实际仓库为准）：
	
	```yaml
	services:
	  deve-note:
	    image: ghcr.io/develeta/deve-note:vX.Y.Z
	    ports:
	      - "28000:28000"
	    volumes:
	      - ./data/ledger:/data/ledger
	      - ./data/vault:/data/vault
	      - ./data/plugins:/data/plugins
	    environment:
	      - DEVE_NOTE_PROFILE=low-spec  # Enable 512MB mode (Disable SSR, etc)
	      - DEVE_NOTE_USER=<username>
	      - DEVE_NOTE_PASSWORD_HASH=<argon2-hash>
	    deploy:
	      resources:
	        limits:
	          memory: 450M # Reserve buffer for OS
	    restart: unless-stopped
	```

* **插件下载与安装**：
	* 发布时 SHOULD 提供“官方插件索引”（例如仓库内 `plugins/` 或独立 `deve-note-plugins` 仓库），每个插件包含：插件包、版本号、能力清单、`SHA256` 校验。
	* 安装方式 MUST 支持离线：用户可直接将插件包放入插件目录（例如 `/data/plugins/<plugin-id>/`），重启或热加载后生效。
	* 若提供插件管理命令，则 SHOULD 具备最小集合：
		* `deve-note plugin list`
		* `deve-note plugin install <url|id>`（下载 + 校验 + 解包到插件目录）
		* `deve-note plugin uninstall <id>`
		* `deve-note plugin enable/disable <id>`
	* 插件安装与更新 MUST 不改变核心数据语义：插件只能通过 Host Functions + Capability 受控地读写 Ledger/Vault/资产。

