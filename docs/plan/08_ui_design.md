# 08_ui_design.md - 界面工程蓝图

本章只定义 UI shell、control surface 与显示层边界，不描述具体功能实例。功能语义见 [../features/08_ui_design.md](../features/08_ui_design.md)，自动化验收见 [../acceptance-cases/05_ui.md](../acceptance-cases/05_ui.md)。

## 1. 目标

- 所有端共享同一套 application/runtime control。
- 显示层必须薄，只负责展示与发出用户意图。
- 关键能力既可由控件触发，也可由 command/control surface 触发。

## 2. 分层

### 2.1 UI Shell

- Web / Desktop / Mobile 仅负责布局、导航与平台壳层适配。

### 2.2 Feature Views

- Explorer、Source Control、Editor、Search、Outline 只消费 runtime 状态。

### 2.3 Application Control

- 文档、repo、source control、session 等 control surface 是唯一业务入口。
- 显示层不得跨模块直接操作 authority state。

## 3. 平台映射

- Web
  - 浏览器壳层，依赖 ws + thin client runtime。
- Desktop
  - 桌面容器，复用相同前端 feature/runtime。
- Mobile
  - drawer / topbar / bottom actions 只是壳层适配，不拥有独立业务真相。

## 4. 控件合同

### 4.1 Button / Menu / Drawer

- 每个控件只负责一个语义动作。
- `More(...)`、`Pin/Unpin`、close、toggle 不能共享含糊回调。

### 4.2 Command First

- 所有核心能力必须有稳定 command/control 入口。
- 控件点击只是触发该入口，不应成为唯一执行路径。

### 4.3 Focus & Overlay

- modal、drawer、sheet、sidebar 必须有明确焦点与关闭合同。
- overlay 层级必须统一管理，避免靠局部样式抢焦点。

## 5. 状态边界

- 纯 UI 偏好可以本地存储。
- repo scope、session、pending writes、source control state 不得由显示层私自持有第二份真相。
- 只读、错误、加载、重连等状态必须由 runtime 提供，显示层只消费。

## 6. 多端一致性

- 多端可以有不同布局，但核心 control surface 必须一致。
- 同一用户动作在不同端应尽量落到同一 runtime/command 链路。

## 7. 禁止事项

- 禁止显示层直接修改 ledger、repo scope、sync vector、auth state。
- 禁止某个控件同时承担多个业务语义。
- 禁止把平台壳层差异扩展成独立业务逻辑分支。
- 禁止用显示层缓存替代 runtime 权威状态。

## 8. 代码边界

- `apps/web/src/components/`
  - shell 与 feature views。
- `apps/web/src/hooks/use_core/`
  - application/runtime control，供组件消费。
- `apps/web/src/i18n/`
  - 只承载显示文本，不承载业务逻辑。
