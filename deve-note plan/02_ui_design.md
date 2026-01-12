# 第一章：界面设计哲学 (UI Design Philosophy)

### 1. The "Cockpit" Concept (驾驶舱概念)

* **信息分层**：
	* **L1 (Focus)**：编辑区是绝对中心，无干扰。
	* **L2 (Context)**：左侧边栏（文件树），右侧边栏（大纲）提供导航。
	* **L3 (Meta)**：底部状态栏显示“和解状态”（Sync/Watcher）、Git 分支、字数统计、**Branch Switcher (Peer 切换器)**。
	* **L4 (Floating)**：`Cmd+K` 命令面板和悬浮工具栏，按需出现。

* **键盘优先 (Keyboard First)**：
	* 所有 UI 操作（切换侧边栏、分屏、搜索、跳转）必须有快捷键。
	* 模仿 Vim/VS Code/Nano 的操作逻辑，减少鼠标移动。

### 2. Reactive Projection (响应式投影)

* **即时反馈**：当后端 Watcher 检测到磁盘上的文件被 VS Code/Nano/Vim 修改时，前端编辑器不应“刷新页面”，而应通过 **Loro 的 Diff 补丁** 平滑地更新内容，并弹出一个非侵入式的 Toast 提示：“已合并外部修改”。
* **Optimistic UI (乐观 UI)**：
    * **Desktop**: 用户输入立即上屏，WebSocket 同步在后台悄悄进行。如果网络失败，状态栏图标变红，但编辑不中断。
    * **Web (Dashboard)**: 乐观 UI **仅在连接存活时有效**。作为 Server 的“瘦客户端”，一旦检测到心跳丢失，界面 **MUST** 立即进入锁定/只读状态 (ReadOnly/Blurred)，并提示“连接断开”，**严禁**产生离线 Ops。

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

* **风格参考**：UI 视觉与版式借鉴语雀与 SilverBullet 的清爽阅读感，但保持开源可自定义主题。
* **标准 Markdown 导出**：导出/投影坚持通用 GFM/Frontmatter，不添加私有标记或语雀式特有格式；富文本元数据仅存于 Ledger。
* **效率取向**：插件与交互倾向 SilverBullet 的轻量/实用，避免臃肿；默认装载最小可用集。
* **导航体验**：侧边栏结构参考 VitePress（分组/层级清晰），命令/快捷键呼出文件列表参考 SilverBullet 弹出式搜索。
