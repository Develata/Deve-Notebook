# 17_plugins.md - 外部扩展体验篇

本章描述插件与外部扩展在产品层的当前可见边界。

## 功能目标

- 用户应明确知道插件/外部扩展不是当前核心主线。
- 外部扩展入口不应越权干扰 Markdown、Repo、Source Control 等核心能力。

## 功能项

### 1. 当前暴露范围

- 当前阶段插件能力如果可见，应该被明确标记为外围或预留。
- 外部扩展入口不应压过主工作流。

### 2. 核心隔离

- 插件或外部扩展即使存在，也应与核心 authority、repo scope、写入主链隔离。
- 用户不应被误导为“插件可以直接替代核心工作流”。
- 插件 host 暴露 Source Control writer 时，必须经由当前 writer gate 与 NoteGit/ngit authority；缺失 gate 的本地 writer 必须拒绝。
- 插件扫描只负责有界读取与编译；只有宿主 repo/sync authority 与当前代际专属 HostContext 安装完成后才执行插件初始化。宿主上下文缺失或已经属于另一 backend 代际时，插件文件写入会被拒绝，而不是被当成普通非托管路径。
- 过多或过大的 plugin、manifest、脚本、模块、skill 或文件读取必须在超过累计预算时拒绝；插件错误不得拖垮低内存宿主。
- plugin-host 收到退出信号时会显式关闭当前 WebSocket 会话；仅 listener 停止而已连接仍存活不算退出完成。

### 3. 未来预留

- 产品可以保留外部扩展接口位，但应明确这是未来能力，不代表当前默认可用。

## Operation 示例

- [`docs/features/operations/plugin_runtime_boundary.md`](./operations/plugin_runtime_boundary.md)
  - 建模当前已经存在的外围 `plugin-host / PluginCall` 边界
  - 不把安装器、市场或默认启用 runtime 误写成当前主线
- [`docs/features/operations/trusted_external_agent_boundary.md`](./operations/trusted_external_agent_boundary.md)
  - 建模 `trusted-cli` 的 interface-only / default-off 边界
  - 不把 Trusted External Agent 误写成“插件系统已经默认可用”

## 非目标

- 当前阶段不把插件系统作为核心功能。
- 当前阶段不允许外部扩展伪装成稳定主线入口。

## Chrome MCP 验收实例

### PLUGIN-FEAT-01: 外围扩展不抢占核心入口

前置条件：

- 打开应用主界面。

步骤：

1. 观察是否存在插件或扩展入口。
2. 比较其权重与编辑器、Repo、Source Control 等主线入口。
3. 尝试进入该入口后再返回主工作流。

期望结果：

- 扩展入口如果存在，应明显属于外围能力。
- 不会抢占核心入口，也不会破坏主工作流。
