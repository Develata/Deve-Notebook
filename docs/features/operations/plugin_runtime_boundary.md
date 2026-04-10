# plugin_runtime_boundary.md - Plugin Host / PluginCall 边界流示例

## Metadata

- `Flow ID`: `flow.plugin.runtime-boundary`
- `Domain`: `plugin`
- `Related Feature Chapters`: `docs/features/17_plugins.md`
- `Related Acceptance Cases`: `PLUG-001`, `AI-005`, `AI-006`

## Operations

### `op.plugin.host.resume-runtime`

- `Name`: `Resume Plugin Host Boundary`
- `Surface`: `peripheral-runtime`
- `Trigger`: 外围运行时进入 plugin-host 可用态，或主进程占用端口后切换到 proxy/plugin-host 路径
- `Preconditions`: 插件仍属于外围能力；未越过 trusted / default-off 边界
- `Immediate Result`: 检测主进程、严格加载本地插件，并暴露只接受 `PluginCall` 的 host WS 入口
- `Application Entry`: `apps/cli/src/commands/serve.rs`, `apps/cli/src/commands/serve_support.rs`, `apps/cli/src/server/plugin_host.rs`, `apps/cli/src/server/plugin_host_ws.rs`

### `op.plugin.call.submit`

- `Name`: `Submit PluginCall`
- `Surface`: `peripheral-runtime`
- `Trigger`: 外围能力发起通用插件调用
- `Preconditions`: plugin host 已就绪；`plugin_id`、`fn_name` 与参数已解析
- `Immediate Result`: 发送 `ClientMessage::PluginCall`
- `Application Entry`: `apps/cli/src/server/plugin_host_ws.rs`, `apps/cli/src/server/handlers/plugin.rs`, `apps/cli/src/server/ws/route/core.rs`

### `op.plugin.call.receive-result`

- `Name`: `Receive PluginResponse`
- `Surface`: `peripheral-runtime`
- `Trigger`: 运行时返回合法 plugin result
- `Preconditions`: `op.plugin.call.submit` 已执行，且目标插件存在并成功序列化结果
- `Immediate Result`: 外围调用拿到 `PluginResponse`
- `Application Entry`: `apps/cli/src/server/plugin_response.rs`, `apps/cli/src/server/plugin_host_ws.rs`

### `op.plugin.call.receive-error`

- `Name`: `Receive Plugin Error`
- `Surface`: `peripheral-runtime`
- `Trigger`: 插件不存在、消息不合法、结果不可序列化，或运行时调用失败
- `Preconditions`: `op.plugin.call.submit` 已执行
- `Immediate Result`: 外围调用拿到 fail-closed 错误，而不是静默降级
- `Application Entry`: `apps/cli/src/server/handlers/plugin.rs`, `apps/cli/src/server/plugin_response.rs`, `apps/cli/src/server/plugin_host_ws.rs`

## Notes

- 这条 flow 只建模外围 plugin runtime 边界，不引入安装器、市场或默认开启的插件主线。
- `Native AI Chat` 仍属于第 10 章的原生产品能力；这里只抽出通用 `PluginCall / PluginResponse` 边界。
- `Trusted External Agent` 仍然是 default-off 高级部署位，不能被误读为“插件系统已经是核心主线”。
