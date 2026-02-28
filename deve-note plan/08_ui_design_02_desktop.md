# 08_ui_design_02_desktop.md - 桌面端设计 (Desktop UI)

> **Status**: Core Specification
> **Platform**: Desktop (Windows / macOS / Linux)

本节定义了 Desktop 端的“驾驶舱”布局规范与交互逻辑。

> **Tauri-Based**: Desktop 端采用 **Tauri v2** 外壳，前端代码与 Web 端共享。
> **Offline-First**: Desktop 端 **MUST** 在无网络环境下保持完整可用。

> **Web Mapping**: 当 Web 端 $W_{view} > 768px$ 时，界面 **MUST** 遵循本章 Desktop 规范。

## 规范性用语 (Normative Language)
*   **MUST**: 绝对要求。
*   **SHOULD**: 强烈建议。

## 1. 布局哲学：驾驶舱 (The Cockpit Concept)

桌面端设计遵循 **"Information Stratification" (信息分层)** 原则，将界面划分为不同关注度的区域。

*   **L1 (Focus)**: 编辑区 (Editor)。绝对中心，无干扰。
*   **L2 (Context)**: 侧边栏 (Sidebar)。提供导航上下文 (Explorer, Outline)。
*   **L3 (Meta)**: 状态栏 (Status Bar)。提供系统元数据 (Git Branch, Sync Status)。
*   **L4 (Floating)**: 悬浮层 (Overlays)。按需出现的命令入口。

## 2. 动态网格系统 (Dynamic Grid System)

### 2.1 布局定义 (Layout Definition)
系统采用 5 列动态网格布局。形式化定义如下：

$$ Grid = [Col_{sidebar}, Col_{diff\_old}, Col_{editor}, Col_{outline}, Col_{chat}] $$

*   **Constraint**: $Col_{editor}$ (Col 3) 总是占据剩余空间 (`1fr`)。
*   **CSS Implementation**:
    ```css
    display: grid;
    grid-template-columns: var(--w-sidebar) var(--w-diff) 1fr var(--w-outline) var(--w-chat);
    ```

### 2.2 布局可视化 (Visualization)

**Main Workbench Structure**:

| Layer      | Col 1 (Resizable) | Col 2 (Fixed) | Col 3 (Flex) | Col 4 (Fixed) | Col 5 (Resizable) |
| :--------- | :---------------- | :------------ | :----------- | :------------ | :---------------- |
| **Top**    | `[Explorer]`      | `Old.rs`      | `New.rs`     | `Outline`     | `AI Chat`         |
| **Body**   | File Tree         | Read-Only     | Writable     | H1..H6        | Chat Log          |
| **Resize** | `[||]` Handle     | -             | -            | -             | `[||]` Handle     |

### 2.3 组件规范 (Component Specs)

*   **Primary Sidebar (Col 1)**:
    *   **Behavior**: **MUST** 支持拖拽调整宽度 (`180px` ~ `500px`)。
    *   **Persistence**: **MUST** 记住用户设置。
*   **Right Panel (Col 5)**:
    *   **Behavior**: **MUST** 支持拖拽调整宽度 (`240px` ~ `520px`)。
    *   **Persistence**: **MUST** 记住用户设置。
*   **Outer Gutter**:
    *   **Behavior**: **MUST** 支持拖拽调整主区域左右边距。
    *   **Persistence**: **MUST** 记住用户设置。
    *   **State**: 包含 `ActivityBar` (Icon Strip) 与 `SideView` (Content)。
*   **Editor Area (Col 2 & 3)**:
    *   **Single Mode**: $Width(Col_2) = 0$。
    *   **Diff Mode**: $Width(Col_2) = 50\%$。
    *   **Scroll Sync**: 当滚动 Col 3 时，Col 2 必须根据文档高度比例同步滚动。
*   **Unified Search Modal (The Brain)**:
    *   **Definition**: 全局统一的输入入口 $I$。
    *   **Modes**:
        *   `Command`: Prefix `>` (e.g., `>Toggle Sidebar`).
        *   `File`: No Prefix (e.g., `src/main.rs`).
        *   `Branch`: Prefix `@` (e.g., `@feature/xyz`).

## 3. 源代码管理界面 (Source Control UI)

### 3.1 视图结构 (View Structure)
定义源代码管理视图 $V_{sc}$ 为三个集合的并集：
$$ V_{sc} = S_{staged} \cup S_{unstaged} \cup H_{commits} $$

*   **Staged ($S_{staged}$)**: 已暂存的文件集合。支持 `Unstage All`。
*   **Unstaged ($S_{unstaged}$)**: 工作区的脏文件集合。支持 `Stage All` / `Discard All`。

### 3.2 变更状态可视化
每个变更项 $Item \in V_{sc}$ **MUST** 使用语义化颜色标记状态：

*   **Modified ($M$)**: Orange (`var(--color-modified)`).
*   **Added ($A$)**: Green (`var(--color-added)`).
*   **Deleted ($D$)**: Red (`var(--color-deleted)`).

## 本章相关命令

*   `view.layout.toggle_sidebar`: 切换侧边栏可见性。
*   `view.layout.toggle_diff`: 切换 Diff/Editor 模式。
*   `git.stage_all`: 暂存所有更改。

## 4. 实现策略 (Implementation Strategy)

### 4.1 跨平台 UI 方案
*   **Rule**: Desktop 采用 **Tauri v2 (WebView)** 作为跨平台外壳，前端代码与 Web 端共享。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致。
*   **Note**: "原生 UI" 在此指用户体验层面（窗口管理、菜单栏、系统托盘等），而非技术实现层面。

### 4.2 内嵌服务 (Embedded Service)
*   **Rule**: 后端服务 **MUST** 内嵌并由桌面端进程拉起。
*   **Local API**: 前端与服务通信 **MUST** 走本机回环或进程内通道。

### 4.2.1 服务启动流程 (Service Boot)
*   **Rule**: Desktop App 启动 **MUST** 先拉起内嵌服务，再启动 UI。
*   **Port**: 端口 **MUST** 使用本机随机可用端口并保存在运行时内存中。
*   **Lifecycle**: 关闭主窗口 **SHOULD** 提供安全退出或后台驻留选项。
*   **Port Conflict**: 若端口占用，**MUST** 自动回退到新的可用端口并重新绑定。

### 4.2.2 本地通信策略 (Local IPC)
*   **Default**: 本机回环 HTTP/WS（`127.0.0.1`）优先。
*   **Fallback**: 若平台限制端口访问，**MUST** 提供进程内通道 (IPC) 替代方案。
*   **Security**: 本地通信 **MUST** 禁止跨进程未授权访问。
*   **Auth**: IPC **MUST** 具备进程级鉴权与会话绑定。

### 4.2.3 端口绑定安全 (Port Binding Security)
*   **Rule**: 服务端 **MUST** 仅监听 `127.0.0.1`。
*   **Firewall**: **SHOULD** 显式阻断非回环访问。

### 4.3 离线优先 (Offline-First)
*   **Rule**: 无网络时 **MUST** 保证完整编辑与索引能力。
*   **Sync**: 恢复网络后增量同步，冲突策略以本地优先。

### 4.3.1 数据持久化 (Persistence)
*   **Rule**: 所有内容 **MUST** 落盘到本地数据库与 Vault。
*   **Crash Safety**: 崩溃后 **MUST** 可恢复到最后一次持久化状态。
*   **Migration**: 数据结构升级 **MUST** 支持自动迁移，失败时 **MUST** 回滚。

### 4.3.2 加密策略 (Encryption)
*   **At-Rest**: 本地存储 **MUST** 支持加密（密钥绑定设备安全模块）。
*   **In-Memory**: 解密后的明文 **SHOULD** 尽量短时保留。
*   **Key Rotation**: **MUST** 支持密钥轮换与失效，轮换过程不得破坏现有数据。
*   **Recovery**: **MUST** 提供密钥恢复策略，避免单点损坏。

### 4.3.3 备份与导出 (Backup & Export)
*   **Backup**: **MUST** 支持本地加密备份。
*   **Export**: **SHOULD** 支持单文档/全量导出。

### 4.3.4 权限与审计 (Permissions & Audit)
*   **Rule**: 本地操作 **MUST** 具备最小权限原则。
*   **Audit**: **SHOULD** 记录关键操作日志（创建/删除/导出/恢复）。

### 4.3.5 恢复演练 (Recovery Drill)
*   **Rule**: 版本升级 **SHOULD** 提供可执行的恢复演练流程。
*   **Goal**: 发生故障时可快速回退到稳定版本。

### 4.4 体积与性能约束 (Size & Performance)
*   **Size**: 体积 **MUST** 控制在可接受范围，避免 UI 框架臃肿。
*   **Perf**: 启动速度与输入延迟优先于视觉特效。
