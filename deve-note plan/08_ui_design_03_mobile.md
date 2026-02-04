# 08_ui_design_03_mobile.md - 移动端设计 (Mobile UI)

> **Status**: Core Specification
> **Platform**: Mobile (iOS / Android)

本节定义了 Mobile 端基于 **Content-First** 哲学的适配策略。

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

## 本章相关配置

*   `ui.mobile.font_size`: 移动端专用字号 (Default: 16).
*   `ui.mobile.toolbar_visible`: 是否显示辅助键盘栏 (Default: true).
