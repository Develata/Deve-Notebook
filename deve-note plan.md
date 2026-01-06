# 📑 Deve-Note plan - 系统架构白皮书

**版本**：0.0.1
**状态**：后端逻辑闭环 + 前端交互定义完整。
**核心理念**：工业级内核 (Industrial Kernel) + 沉浸式驾驶舱 (Immersive Cockpit)。

---

## 第一章：界面设计哲学 (UI Design Philosophy) - [新增核心]

### 1. The "Cockpit" Concept (驾驶舱概念)

* **信息分层**：
* **L1 (Focus)**: 编辑区是绝对中心，无干扰。
* **L2 (Context)**: 侧边栏（大纲、文件树）提供导航。
* **L3 (Meta)**: 底部状态栏显示“和解状态”（Sync/Watcher）、Git 分支、字数统计。
* **L4 (Floating)**: `Cmd+K` 命令面板和悬浮工具栏，按需出现。


* **键盘优先 (Keyboard First)**：
* 所有 UI 操作（切换侧边栏、分屏、搜索、跳转）必须有快捷键。
* 模仿 Vim/VS Code 的操作逻辑，减少鼠标移动。



### 2. Reactive Projection (响应式投影)

* **即时反馈**：当后端 Watcher 检测到磁盘上的文件被 VS Code 修改时，前端编辑器不应“刷新页面”，而应通过 **Loro 的 Diff 补丁** 平滑地更新内容，并弹出一个非侵入式的 Toast 提示：“已合并外部修改”。
* **乐观 UI (Optimistic UI)**：用户输入立即上屏，WebSocket 同步在后台悄悄进行。如果网络失败，状态栏图标变红，但编辑不中断。

### 3. Mathematical Aesthetics (数学美学)

* **排版**：默认集成 **KaTeX** (快速) 或 **MathJax 3** (精确)，支持复杂的数学公式渲染。
* **字体**：预设适合代码和数学公式的等宽字体 (如 JetBrains Mono, Fira Code) 和衬线字体 (如 Merriweather)。

---

## 第二章：UI 架构与组件系统 (UI Architecture) - [新增核心]

前端采用 **Leptos (Signals)** + **Tailwind CSS**，构建一套高性能组件库。

### 1. 布局系统 (The Layout Engine)

采用 **"Resizable Slot" (可缩放插槽)** 布局：

* **Left Slot**: 文件树 / 双向链接图谱 (Mini Graph)。
* **Main Slot**: 多标签页 (Tabs) 编辑器 / 分屏 (Split View)。
* **Right Slot**: 大纲 (TOC) / 属性面板 (Metadata) / 插件面板。
* **Bottom Slot**: 终端面板 (Terminal) / 日志输出。
* **特性**：所有面板状态（宽度、折叠）持久化存储在 Redb 的 `ui_state` 表中，重启后完全恢复。

### 2. 编辑器内核 (The Editor Kernel)

不仅仅是一个 `<textarea>`，而是一个分层渲染器：

* **Layer 1 (Input)**: `ContentEditable` 或 CodeMirror 6，负责捕获输入。
* **Layer 2 (State)**: 绑定 Loro CRDT 状态，处理并发冲突。
* **Layer 3 (Render)**:
* **Block Mode**: 类似 Notion，支持拖拽块。
* **Source Mode**: 纯 Markdown 源码模式（配合 Moncao/CodeMirror 高亮）。
* **Live Preview**: 类似 Obsidian/Milkdown，源码即所见。
* **技术选型**：推荐 **Milkdown (基于 Prosemirror)**，因为它对 Vue/React/Leptos 友好且插件丰富。



### 3. 可视化系统 (Visualization System)

* **Global Graph (全域图谱)**:
* 使用 **Rust -> Wasm** 编译的力导向图引擎 (Force-directed Graph)，支持 10,000+ 节点流畅渲染（Canvas/WebGL）。
* 支持按 Tag、文件夹颜色聚类。


* **Time Travel Slider (时光轴)**:
* UI 底部的一条交互式热力图。
* 颜色深浅代表修改频率。
* 拖动滑块，编辑器内容实时回滚（基于内存中的 Loro 历史，无延迟）。



### 4. 命令面板 (The Commander)

* 呼出：`Cmd/Ctrl + K`。
* 功能：
* **导航**: `Go to file...` (模糊搜索 Tantivy 索引)。
* **操作**: `Toggle Dark Mode`, `Git Push`, `Export PDF`.
* **插入**: `Insert Math Block`, `Insert Date`.



---

## 第三章：统一后端架构 (The Vibranium Backend)

*(保留 v7.2 的核心，确保逻辑闭环)*

* **存储**: **Redb** (Metadata + Ledger + UI State)。
* **搜索**: **Tantivy** (驻留内存，实时索引)。
* **同步**: **Axum + Tower** (背压流控)。
* **和解**: **Notify + Dissimilar + Sentinel Lock** (解决外部修改死循环)。
* **插件**: **Rhai + Extism**。

---

## 第四章：数据流与交互 (Interaction Flows)

### 场景一：外部编辑器协同 (The "Alt-Tab" Flow)

1. 用户在 VS Code 中打开 `/data/vault/thesis.md`，修改了一段公式，保存。
2. **后端**: `Notify` 捕获 -> 检查锁 -> 读取文件 -> `Diff` -> 生成 Ops -> 存入 Redb。
3. **推送**: 后端通过 WebSocket 推送新的 Ops 到 Titanium 客户端。
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
3. **调用**: 触发 Rhai 脚本 `git_sync.rhai`。
4. **后端**: 执行 `git add .` -> `git commit` -> `git push`。
5. **反馈**: 状态栏转圈 -> 变绿 "Synced"。

---

## 第五章：技术栈清单 (The Full Stack)

| 层次 | 核心技术 | 选型理由 |
| --- | --- | --- |
| **语言** | Rust (2024) | 全栈统一。 |
| **前端框架** | **Leptos v0.7** | 信号驱动，性能极致，无 Virtual DOM 开销。 |
| **UI 组件** | **Tailwind CSS** | 原子化 CSS，配合 Shadcn-UI (Leptos port) 实现一致性设计。 |
| **编辑器** | **Milkdown (Prosemirror)** | 强大的 Markdown 插件化编辑器内核。 |
| **图标库** | **Lucide Icons** | 统一、现代的 SVG 图标集。 |
| **图谱渲染** | **Pixi.js** 或 **Cosmic-Graph (Rust)** | WebGL 加速的图可视化。 |
| **存储** | **Redb** | 纯 Rust 嵌入式 DB。 |
| **搜索** | **Tantivy** | 全文检索引擎。 |
| **和解** | **Notify + Dissimilar** | 文件监听与 Diff。 |
| **构建** | **Tauri v2** | 跨平台外壳。 |

---

## 第六章：开发实施路线图 (Roadmap) - [含 UI 阶段]

**Phase 1: 坚实底座 (The Bedrock)**

* 实现 `StorageDriver` (Redb) 和 `Reconciler` (Watcher/Diff)。
* *里程碑：后端能正确处理 VS Code 的外部修改。*

**Phase 2: 界面骨架 (The Skeleton)**

* 搭建 Leptos + Tailwind 项目结构。
* 实现 **Resizable Slots Layout** (三栏布局)。
* 实现 `Cmd+K` 命令面板的基础逻辑。
* *里程碑：有一个可以拖拽调整大小、有命令面板的空壳应用。*

**Phase 3: 编辑器核心 (The Editor)**

* 集成 Milkdown。
* 实现 **Loro <-> Prosemirror** 的绑定（这是前端最难的部分，实现协同编辑）。
* 配置 KaTeX 插件。
* *里程碑：可以打字、渲染公式、并与后端 Redb 同步内容。*

**Phase 4: 可视化与高级功能 (The Visuals)**

* 集成 Tantivy 实现搜索 UI。
* 实现 Graph View (WebGL)。
* 实现时光轴滑块。
* *里程碑：真正可用的知识库。*

---
