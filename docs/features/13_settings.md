# 13_settings.md - 设置体验篇

本章描述设置界面的分组、即时反馈和当前有效配置的用户体验。

原子操作示例：[`operations/settings_update.md`](./operations/settings_update.md)

细粒度操作链：
[`settings_surface_open.md`](./operations/settings_surface_open.md),
[`settings_value_mutation.md`](./operations/settings_value_mutation.md),
[`settings_persistence_apply.md`](./operations/settings_persistence_apply.md),
[`settings_feedback_render.md`](./operations/settings_feedback_render.md)

## 功能目标

- 用户应能找到主要设置入口。
- 设置变更后应得到可见反馈。
- 已生效设置与未来预留项应清楚区分。

## 功能项

### 1. 设置分组

- 设置项应按主题分组展示，如界面、语言、编辑体验、运行模式等。
- 用户不应在设置中迷失于无结构的长列表。
- 细粒度来源链：[`settings_env_defaults.md`](./operations/settings_env_defaults.md)、[`settings_file_config.md`](./operations/settings_file_config.md)、[`settings_ui_preferences.md`](./operations/settings_ui_preferences.md)、[`settings_runtime_feedback.md`](./operations/settings_runtime_feedback.md)

### 2. 即时反馈

- 修改主题、语言、面板显示等设置后，界面应出现即时变化。
- 设置不应表现为“点了没反应”。

### 3. 当前有效 vs 预留能力

- 当前真实可用的设置应清楚标记并生效。
- 预留或未来能力不应伪装成已经完成。

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
4. 检查预留项是否被明确区分。

期望结果：

- 设置入口清晰可达。
- 已实现设置有即时反馈。
- 预留项不会误导成已完成能力。
