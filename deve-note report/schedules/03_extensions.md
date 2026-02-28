# 扩展系统进度 (Extensions Schedule)

> 涵盖计划: 11_plugins

## 1. 插件运行时 (Plugin Runtime)
- [x] **Engine A (Application)**:
    - [x] **Rhai**: 集成脚本引擎 (RhaiRuntime + FileModuleResolver + 16 host funcs).
    - [ ] **WASM**: 集成 Extism/Wasmtime.
    - [x] **Manifest**: 插件清单解析 (PluginManifest + Capability 5 权限类型).
- [ ] **Engine B (Computation)**:
    - [ ] **Podman**: 容器运行时集成.
    - [ ] **Socket Bridge**: WebSocket 转发.

## 2. AI 集成 (AI Integration)
- [x] **Chat UI**: 右侧边栏 AI 面板 + 动态插件选择 (ai_mode signal).
- [x] **Provider API**: OpenAI 兼容 SSE 流式 + ToolCalling + ai-chat 内置 Rhai 插件.
- [ ] **Context Injection**: 选中代码自动注入.

## 3. Git 集成 (Git Integration)
- [x] **Git Hook**: Rhai host API (sc_status/sc_diff/sc_stage/sc_commit).
- [ ] **UI Actions**: 绑定 Sync/Push 按钮事件.

## 4. Agent Bridge (外部 CLI 集成)
- [x] **Subprocess spawner**: 外部 CLI 进程桥 (默认 opencode).
- [x] **Settings toggle**: CLI/API 后端切换 UI.
