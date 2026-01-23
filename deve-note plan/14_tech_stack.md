# 14_tech_stack.md - 技术栈篇 (Technology Stack)

## 技术栈清单 (The Full Stack)

| **Layer**    | **Technology**           | **Status**        | **Selection Reasoning**             |
| :----------- | :----------------------- | :---------------- | :---------------------------------- |
| **Language** | Rust (2024)              | Verified          | 全栈统一。                          |
| **Frontend** | **Leptos v0.7**          | Verified          | 信号驱动 (Signals)，性能极致。      |
| **UI**       | **Tailwind CSS**         | Verified          | 原子化 CSS。                        |
| **Router**   | **leptos_router**        | Verified          | 前端路由管理。                      |
| **I18n**     | **leptos_i18n**          | Verified          | 编译时类型安全校验。                |
| **Editor**   | **CodeMirror 6**         | Verified          | 核心编辑器，Headless 模式。         |
| **Icons**    | **Lucide Icons**         | Verified          | 统一 SVG 图标集。                   |
| **Storage**  | **Redb** (Native)        | Verified          | 嵌入式 KV 数据库 (Zero-copy).       |
| **Auth**     | **Argon2 + Ed25519**     | Verified          | 身份认证与节点签名。                |
| **Diff**     | **Dissimilar**           | Verified          | 文本差异计算算法。                  |
| **CLI**      | **Clap v4**              | Verified          | 命令行解析。                        |
| **Async**    | **Tokio v1**             | Verified          | 异步运行时。                        |
| **Logs**     | **Tracing**              | Verified          | 结构化日志。                        |
| **Graph**    | **d3-force + Pixi.js**   | Planned           | 高性能图谱渲染 (Web Canvas).        |
| **Search**   | **Tantivy** (Rust)       | Planned           | 全文检索、模糊搜索 (Backend).       |
| **Sync**     | **Axum + Tower**         | Planned (Partial) | HTTP/WebSocket 背压与流控。         |
| **Build**    | **Tauri v2**             | Planned           | 跨平台外壳 (Mobile/Desktop)。       |
| **Plugins**  | **Rhai + WASM (Extism)** | Planned           | 双层插件体系 (Scripting + Binary)。 |

## Markdown 兼容性与回归清单 (Compatibility Checklist)

*   **导出原则**：通用 GFM，无私有语法。
*   **语法基线**：CommonMark + GFM。
*   **链接约定**：内部 `doc://` <=> 导出相对路径。
*   **资产约定**：`asset://` <=> 导出图片引用。
*   **回归用例**：CI 快照对比。

## 性能预算与配置 (Performance & Profiles)

### High/Low Profile
*   **Low-Spec (512MB)**: CSR Only, No Search Index, Snapshot Pruning.
*   **Standard (1GB+)**: SSR, Search, Graph.

## 本章相关命令

* 无。

## 本章相关配置

*   `DEVE_PROFILE`: `standard` | `low-spec`.
*   `MEM_CACHE_MB`: Memory cache limit.
