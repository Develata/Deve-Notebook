# 13_settings.md - 设置体验篇

本章描述设置界面的分组、即时反馈和当前有效配置的用户体验。

原子操作示例：[`operations/settings_update.md`](./operations/settings_update.md)

细粒度操作链：
[`settings_surface_open.md`](./operations/settings_surface_open.md),
[`settings_value_mutation.md`](./operations/settings_value_mutation.md),
[`settings_persistence_apply.md`](./operations/settings_persistence_apply.md),
[`settings_feedback_render.md`](./operations/settings_feedback_render.md),
[`locale_surface_switch.md`](./operations/locale_surface_switch.md)

## 功能目标

- 用户应能找到主要设置入口。
- 设置变更后应得到可见反馈。
- 已生效设置与未来预留项应清楚区分。

## 功能项

### 1. 设置分组

- 设置项应按主题分组展示，如界面、语言、编辑体验、运行模式等。
- 当前 v1 分组至少覆盖主题、自动换行、编辑器密度、最大文档标签页数、语言、同步模式、AI 后端、AI Chat 面板可见性、native Backend 与运行诊断入口。
- 用户不应在设置中迷失于无结构的长列表。
- 细粒度来源链：[`settings_env_defaults.md`](./operations/settings_env_defaults.md)、[`settings_file_config.md`](./operations/settings_file_config.md)、[`settings_ui_preferences.md`](./operations/settings_ui_preferences.md)、[`settings_runtime_feedback.md`](./operations/settings_runtime_feedback.md)

### 2. 即时反馈

- 修改主题、语言、面板显示等设置后，界面应出现即时变化。
- 浏览器本地主题与编辑器基础偏好应通过根节点标记、可见按钮状态或数字输入反馈；不得写入 repo authority。
- 最大文档标签页数是本地 UI 偏好，只限制 Markdown 文档标签页自动淘汰，不限制 Diff 标签页，也不持久化打开文档列表；该数字输入必须在 blur / Enter / change 时提交，输入两位数的中间过程不应触发临时淘汰。
- AI Chat 面板可见性是本地 UI 偏好；关闭后应同时移除 Chat 面板和桌面 Chat 分界线，不应改变 AI 后端配置。
- Backend section 在普通浏览器中必须显示 native-only unavailable；在 Desktop/Mobile native 中可选择 Local Backend 或 Remote Backend。Remote URL 未经 native node-role 校验成功不能保存，校验中/失败/成功都应有可见反馈。
- 切回 Local Backend 应保存 native host-local preference，并触发本机后端启动与 Web shell 重载；该操作不写浏览器 localStorage、不写 `config.toml`、不保存 session/token。
- Settings 模态框只保留右上角关闭按钮；底部不应再渲染大号关闭按钮。
- 设置不应表现为“点了没反应”。

### 3. 当前有效 vs 预留能力

- 当前真实可用的设置应清楚标记并生效。
- 预留或未来能力不应伪装成已经完成。
- 运行时配置写入口应复用 plan 中的类型与范围约束；例如 `p2p.connect_interval_ms = 0` 这类会破坏重连节流语义的值必须被拒绝，不能写入 `config.toml` 后再依赖运行时兜底。

## 非目标

- 当前阶段不要求所有工程内部参数都暴露到用户设置界面。
- 当前阶段不允许设置页直接操控 authority 真相或绕过 runtime。

## Chrome MCP 验收实例

### SETTINGS-FEAT-01: 设置可见且有反馈

前置条件：

- 打开应用主界面。

步骤：

1. 打开 Settings。
2. 修改语言、主题或面板显示类设置。
3. 观察界面变化。
4. 检查 Backend section：普通浏览器应显示 native-only unavailable；native shell 中 remote URL 必须校验成功后才能保存。
5. 检查预留项是否被明确区分。

期望结果：

- 设置入口清晰可达。
- 已生效设置有即时反馈。
- native Backend 选择不会伪装成 server-backed Settings API 或 browser-local preference。
- 预留项不会误导成已完成能力。
