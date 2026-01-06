# Deve-Note 计划排期（0.0.1）

## Phase 1: Ledger/Projection 基座
- 搭建 Cargo workspace；实现 append-only 账本写读、分段日志与 Snapshot 序列化。
- 建立 DocId 基础类型、LedgerEntry、CapabilityManifest 结构；持久化 Redb/Sled 索引。
- One-way 投影管线雏形：Ledger -> Markdown（可延迟写）。
- 基础安全：Argon2 + JWT；Tower 中间件限流/超时/熔断。

## Phase 2: VFS 与同步协议
- 建立 `DocId <-> Path` 双向映射；重命名/移动仅改映射。
- WebSocket 握手与同步模式：轻微落后重放 Ops，严重落后直拉 Snapshot。
- 本地存储抽象：桌面 sqlite/Redb，移动 sqlite/IndexedDB，Web IndexedDB。
- 投影任务后台化，支持按需/延迟写。

## Phase 3: 编辑器与 MD/数学基线
- Leptos + Milkdown 集成；实现 Loro <-> Prosemirror 绑定。
- Markdown 基线能力与快捷键、粘贴清洗；图片/粘贴入库生成 DocId 引用。
- 数学体验：行内/块级模式切换、错误高亮、导出 SVG/PNG、离线 KaTeX 资源。

## Phase 4: 插件运行时与资产模型
- 定义插件 ABI、生命周期、事件总线；Host Functions + Capability 校验。
- 资源配额/隔离、崩溃恢复；命令注册与 UI 插槽扩展通道。
- 资产模型：asset://<uuid> 引用、去重/分片、产物写回流程。

## Phase 5: AI 与计算扩展
- AiClient/ModelRegistry 抽象，云端与本地模型并存；流式输出、函数调用、速率限制。
- 工具注册与能力绑定；长任务队列（超时/取消/重试），产物落地 DocId。
- 隐私/遥测默认关闭，显式 opt-in。

## Phase 6: 多端发布与打包
- 单内核多外壳：Tauri Desktop、PWA/Web、Tauri Mobile/WebView。
- Storage/Net adapter 实现；CI matrix 覆盖桌面/wasm/mobile，产出 bundle/PWA/APK/AAB/TestFlight。
- 开源配套：许可证、贡献指南、发布说明/Changelog、Docker 镜像与 SBOM。
# 开发实施路线图