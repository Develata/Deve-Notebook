# 08_ui_design_03_mobile.md - 移动端设计 (Mobile UI)

> **Status**: Core Specification
> **Platform**: Mobile (iOS / Android)

本节定义了 Mobile 端基于 **Content-First** 哲学的适配策略。

> **Native-First**: Mobile 端 **MUST** 以原生 UI 为标准实现（非 WebView）。
> **Offline-First**: Mobile 端 **MUST** 在无网络环境下保持完整可用。

> **Web Mapping**: 当 Web 端 $W_{view} \le 768px$ 时，界面 **MUST** 遵循本章 Mobile 规范。

## 规范性用语 (Normative Language)
*   **MUST**: 绝对要求。
*   **SHOULD**: 强烈建议。

## 1. 响应式架构 (Responsive Architecture)

### 1.1 布局状态机 (Layout State Machine)
系统布局 $L$ 根据视口宽度 $W_{view}$ 在两种状态间切换：

*   **Desktop State**: $W_{view} > 768px \implies$ Grid Layout.
*   **Mobile State**: $W_{view} \le 768px \implies$ Stack Layout.

### 1.2 视口配置 (Viewport Configuration)
为了适配刘海屏 (Notch) 并防止 iOS 自动缩放，HTML Header **MUST** 包含：

```html
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
```

并且所有固定定位元素 **MUST** 使用 CSS `env()` 适配安全区域：
*   `padding-top: env(safe-area-inset-top)`
*   `padding-bottom: env(safe-area-inset-bottom)`

## 2. 交互设计 (Interaction Design)

### 2.1 导航策略 (Navigation)
移动端移除常驻侧边栏，改为 **Drawer (抽屉)** 模式。

*   **Left Drawer (Sidebar)**:
    *   **Trigger**: 左上角汉堡菜单 (`≡`) 或 **屏幕左边缘右滑 (Edge Swipe)**。
    *   **Visual**: 覆盖在编辑器之上，背景带有半透明 Backdrop (`z-index: 100`).
*   **Right Drawer (Outline)**:
    *   **Trigger**: 右上角图标 或 **屏幕右边缘左滑**。

### 2.2 面板宽度策略 (Panel Width Policy)

*   **Resizable Handles**: 移动端 **SHOULD NOT** 显示左右拉伸手柄。
*   **Persistence**: 仍可读取已保存的桌面宽度，但移动端不提供调整入口。
*   **Outer Gutter**: 移动端 **SHOULD NOT** 提供外边距拖拽。

### 2.3 虚拟辅助键盘栏 (Mobile Toolbar)
为了解决移动端输入 Markdown 符号的痛点，系统 **MUST** 在软键盘上方渲染 Accessory View。

**Key Layout (Visual Representation)**:

| `⇥` Tab | `H`ead | `•` List | `☑` Task | `B`old | `I`talic | `<>` Code | `↩` Undo |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Indent | `#` | `-` | `[ ]` | `**` | `_` | \` | Cmd+Z |

**Technical Constraint**:
必须使用 `visualViewport` API 监听键盘高度变化，动态调整 Toolbar 的 `bottom` 偏移量，防止被键盘遮挡。

### 2.4 手势系统 (Gesture System)
仅支持轻量级 Edge Swipe，参数定义如下：
*   $Zone_{edge} = 20px$ (从屏幕边缘起算的响应区)。
*   $Threshold_{swipe} = 50px$ (触发滑动的最小距离)。

## 3. 视觉适配 (Visual Adaptations)

### 3.1 布局约束
*   **Diff View**: 移动端 **MUST NOT** 使用左右并排 (Side-by-Side) 对比，而应强制回退到 **Unified View** (单列混合)。
*   **Font Size**: 默认字号 **SHOULD** 设为 `16px` 以避免 iOS Safari 输入时强制放大页面。

### 3.2 只读模式指示器 (Spectator Indicator)
在 Spectator Mode 下，顶部导航栏下方 **MUST** 插入一条醒目的橙色横幅 (`Height: 24px`)，提示 "Read-Only Mode"。

## 4. UI 结构设计 (Mobile UI Layout)

### 4.1 结构层级 (Hierarchy)
*   **Top App Bar**: 固定顶部，包含导航与核心操作。
*   **Content Stack**: 单列内容区，默认全屏编辑器。
*   **Bottom Bar**: 简化状态栏（连接/同步/只读）。

### 4.2 顶部导航栏 (Top App Bar)
*   **Left**: Hamburger Menu (`≡`) 打开 Sidebar Drawer。
*   **Center**: 文档标题/仓库名（省略溢出）。
*   **Right**: Search / Outline / More (菜单)。

### 4.3 Drawer 规范 (Side Drawers)
*   **Sidebar Drawer**:
    *   内容：文件树、快速操作、新建。
    *   行为：点击文件后自动收起。
*   **Outline Drawer**:
    *   内容：标题结构、大纲条目。
    *   行为：点击条目后自动收起并滚动定位。

### 4.4 编辑器区 (Editor)
*   **Mode**: 单列布局，支持全屏编辑。
*   **Diff**: 强制 Unified View。
*   **Selection**: 单指选区，长按弹出操作菜单。

### 4.5 快捷入口 (Quick Actions)
*   **Search**: 打开 Quick Open / Command Palette（移动端应为底部抽屉）。
*   **Sync**: 可在 More 菜单中触发。

## 5. 交互流程 (Key Flows)

### 5.1 打开文档
1. 点击 Hamburger -> 打开 Sidebar Drawer。
2. 选择文档 -> Drawer 自动收起 -> Editor 渲染。

### 5.2 查看大纲
1. 点击 Outline 图标 -> 打开右侧 Drawer。
2. 点击条目 -> Drawer 自动收起 -> Editor 滚动定位。

### 5.3 搜索/命令
1. 点击 Search -> Bottom Sheet 打开。
2. 选择结果 -> 自动关闭并跳转。

## 6. 性能与体积 (Performance & Size)
*   **Target**: 首屏渲染 < 1s，输入延迟 < 16ms。
*   **Memory**: 低端设备 **MUST** 平稳运行。
*   **Dependency**: 移动端 **MUST** 避免重型 UI 框架。

## 6.1 视觉参考 (Yuque-Inspired)
*   **Tone**: 轻盈、克制、阅读优先。
*   **Layout**: 卡片化信息层级，内容区留白适中。
*   **Typography**: 标题略微加重，正文中性字重，行高舒适。
*   **Surface**: 浅色背景 + 轻阴影，强调内容层次而非装饰。
*   **Interaction**: 抽屉与底部面板动效柔和，避免夸张动画。

## 本章相关配置

*   `ui.mobile.font_size`: 移动端专用字号 (Default: 16).
*   `ui.mobile.toolbar_visible`: 是否显示辅助键盘栏 (Default: true).

## 7. 实现策略 (Implementation Strategy)

### 4.1 原生 UI 方案 (Native UI)
*   **Rule**: Mobile **MUST** 使用原生 UI 实现（非 WebView）。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致，行为不以 Web 端为准。

### 4.2 内嵌服务 (Embedded Service)
*   **Rule**: 后端服务 **MUST** 内嵌到安装包中，应用启动时自动拉起。
*   **Local API**: 前端通过本机回环接口访问内嵌服务，禁止依赖公网。

### 4.2.1 服务启动流程 (Service Boot)
*   **Rule**: App 启动 **MUST** 先拉起内嵌服务，再启动 UI。
*   **Port**: 端口 **MUST** 使用本机随机可用端口并保存在运行时内存中。
*   **Lifecycle**: App 进入后台时 **SHOULD** 降低服务资源占用；恢复前台时自动唤醒。
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
*   **Rule**: 无网络时 **MUST** 提供完整读写能力。
*   **Sync**: 网络恢复后执行增量同步，失败时不影响本地编辑。

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
*   **Size**: 体积 **MUST** 可控，避免引入重型依赖。
*   **Perf**: 输入延迟与滚动流畅性必须优先保障。

## 8. Web Mobile 映射对齐清单 (2026-02)

> 目标：在 `W_view <= 768px` 的 Web 视口中，持续对齐本章 Mobile 规范。

### 8.1 当前已完成
*   移动布局模块化拆分（`mobile_layout/{mod,header,content,footer,effects,gesture}`）。
*   Drawer 模块化拆分（`mobile_layout/drawers/{mod,left,right}`）。
*   左右抽屉、边缘滑动开关、抽屉互斥、抽屉打开时 body 锁滚。
*   Safe-area 适配、Bottom Sheet 搜索面板、空态与 CTA、基础触控反馈。

### 8.2 本轮优先对齐项
*   **Bottom Sheet 手势关闭**：
    *   **MUST** 具备下拉关闭阈值（避免轻微位移误关闭）。
    *   **MUST** 增加误触防抖（短时微位移不触发关闭）。
    *   **MUST** 处理与滚动冲突：仅在列表位于顶部且判定为下拉意图时才允许关闭。
*   **Drawer 可达性一致性**：
    *   **MUST** 统一标题栏与关闭按钮交互语义。
    *   **SHOULD** 保障触控命中高度不低于 `44px`。
*   **列表触控反馈一致性**：
    *   Sidebar / Outline / Search Result 的 `selected`、`hover`、`active` 语义 **MUST** 保持一致。

### 8.3 执行与验证
*   小步迭代，每轮改动后执行：`cargo clippy --all-targets --all-features -- -D warnings`。
*   保持低复杂度与模块化；单文件目标 `< 130` 行，熔断阈值 `250` 行。

### 8.4 本轮落地记录 (Web, 2026-02)
*   Bottom Sheet 手势关闭已加入三段判定：
    *   距离阈值：`72px`。
    *   防抖：`<=90ms` 且位移 `<=20px` 视为误触。
    *   滚动冲突：仅当结果列表 `scrollTop == 0` 且判定为下拉意图时允许关闭。
*   Drawer 交互已统一：标题栏/关闭按钮命中区与反馈一致（目标触控高度 `44px+`）。
*   Sidebar / Outline / Search Result 已对齐 `hover` / `active` / `selected` 的移动优先语义。
*   视口配置已对齐：`index.html` 的 `meta viewport` 已补齐 `maximum-scale=1.0`、`user-scalable=no`、`viewport-fit=cover`。
*   Spectator 指示条已对齐：移动端内容区顶部增加 `24px` 橙色 "Read-Only Mode" 横幅。
*   Diff 视图已对齐：移动端强制单列 Unified 渲染，避免左右并排。

## 9. SHOULD 条目映射矩阵 (Web Mobile)

| 条目 | 规范原文 (SHOULD) | 代码路径 | 状态 | 备注 |
| :--- | :--- | :--- | :--- | :--- |
| MOB-SHOULD-001 | Resizable Handles: 移动端 SHOULD NOT 显示左右拉伸手柄 | `apps/web/src/components/main_layout.rs` | 已实现 | `is_mobile` 分支渲染 `MobileLayout`，不挂载桌面拖拽手柄 UI。 |
| MOB-SHOULD-002 | Outer Gutter: 移动端 SHOULD NOT 提供外边距拖拽 | `apps/web/src/components/main_layout.rs` | 已实现 | 外边距拖拽仅在 `DesktopLayout` 生效。 |
| MOB-SHOULD-003 | Font Size: 默认字号 SHOULD 设为 16px | `apps/web/src/editor/mod.rs` | 部分实现 | 编辑器基础字号尚未统一锁定为 16px（后续可在编辑器容器样式或主题变量中强制）。 |
| MOB-SHOULD-004 | App 后台时服务 SHOULD 降低资源占用 | N/A (Web Scope) | 不适用 | 属于原生 Mobile App 进程生命周期，不在 Web 映射实现范围。 |
| MOB-SHOULD-005 | Firewall SHOULD 显式阻断非回环访问 | N/A (Embedded Service) | 不适用 | 属于移动端内嵌服务与系统防火墙策略。 |
| MOB-SHOULD-006 | Export SHOULD 支持单文档/全量导出 | N/A (Mobile Native Service) | 不适用 | 属于原生端导出与存储能力。 |
| MOB-SHOULD-007 | Audit SHOULD 记录关键操作日志 | N/A (Core/Service) | 不适用 | 属于后端审计链路，不在 Web UI 直接实现。 |
| MOB-SHOULD-008 | Recovery Drill SHOULD 提供恢复演练流程 | N/A (Release/Ops) | 不适用 | 属于发布与运维流程规范。 |

### 9.1 与本轮实现直接相关的 SHOULD 细化

| 条目 | 代码路径 | 状态 | 备注 |
| :--- | :--- | :--- | :--- |
| MOB-UX-SHOULD-001 | `apps/web/src/components/mobile_layout/drawers/left.rs`, `apps/web/src/components/mobile_layout/drawers/right.rs`, `apps/web/src/components/mobile_layout/header.rs` | 已实现 | 触控命中区统一到 `44px+`，标题栏/关闭按钮语义一致。 |
| MOB-UX-SHOULD-002 | `apps/web/src/components/search_box/ui.rs`, `apps/web/src/components/search_box/sheet_gesture.rs` | 已实现 | Bottom Sheet 手势关闭已做阈值/防抖/滚动冲突判定。 |
| MOB-UX-SHOULD-003 | `apps/web/src/components/sidebar/item.rs`, `apps/web/src/components/outline.rs`, `apps/web/src/components/search_box/result_item.rs` | 已实现 | 列表项 `hover/active/selected` 语义对齐，移动端优先 `active`。 |
