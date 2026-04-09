# 08_ui_design_02_desktop.md - Desktop 壳层体验篇

本章描述 Desktop 端在宽屏工作台语义下应呈现的用户体验。当前 Chrome MCP 可通过宽视口的共享 Web shell 验证大部分交互行为。

## 功能目标

- 用户能获得稳定的 desktop-style workbench。
- 用户能在宽屏布局下高效访问 editor、diff、outline、chat、source control。
- 桌面壳层不应因为菜单、pin、panel 切换而产生语义混乱。

## 功能项

### 1. 宽屏工作台布局

- 用户应看到清晰的多列工作区。
- sidebar、editor、outline、chat、diff 等区域应在宽屏中合理分布。

### 2. 可调整的面板

- sidebar 和右侧面板应允许调整宽度。
- 用户设置后的布局应保持稳定，不因刷新或切换 view 而混乱。

### 3. Source Control 视图

- staged / unstaged / history / graph 等区域应在桌面壳层内清晰可见。
- source control 列表、菜单、颜色语义应明确。

### 4. 命令与更多菜单

- activity bar、更多菜单、repo switcher、command palette 都应能稳定工作。
- `Pin/Unpin` 与“切换视图”的语义应严格分离。

## 非目标

- 当前阶段不在本章定义 Tauri 原生托盘、系统菜单等平台整合细节。
- 当前阶段不要求 Chrome MCP 覆盖真正的原生窗口管理能力。

## Chrome MCP 验收实例

### DESKTOP-UI-01: 宽屏工作台稳定

前置条件：

- 在宽视口打开应用。

步骤：

1. 观察 sidebar、editor、right panel、status 区。
2. 打开一个文档，再进入 diff 或 source control。
3. 观察布局是否仍然清晰稳定。

期望结果：

- 宽屏工作台结构稳定。
- 用户不会失去当前区域与控制入口的上下文。

### DESKTOP-UI-02: 更多菜单与固定语义分离

前置条件：

- 宽视口下进入 activity bar 或侧栏。

步骤：

1. 打开 `More(...)` 菜单。
2. 点击某个 view 切换项。
3. 再点击 `Pin/Unpin`。

期望结果：

- view 切换只切换 view。
- `Pin/Unpin` 只修改固定状态，不会误触发其他动作。
