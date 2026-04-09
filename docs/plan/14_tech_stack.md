# 14_tech_stack.md - 技术栈篇 (Technology Stack)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Reference`
- `Counterpart Feature`: `docs/features/14_tech_stack.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md`
- `Primary Code Areas`: `Cargo.toml`, `apps/web/Cargo.toml`, `apps/cli/Cargo.toml`

## 1. Technology Stack

| **Layer**    | **Technology**           | **Status**        | **Selection Reasoning**             |
| :----------- | :----------------------- | :---------------- | :---------------------------------- |
| **Language** | Rust (2024)              | Verified          | 全栈统一。                          |
| **Frontend** | **Leptos v0.7**          | Verified          | 信号驱动 (Signals)，性能极致。      |
| **UI**       | **Tailwind CSS**         | Verified          | 原子化 CSS。                        |
| **Router**   | **leptos_router**        | Verified          | 前端路由管理。                      |
| **I18n**     | **自研 `Locale + t::*`** | Verified          | 轻量、零代码生成、调用面稳定。      |
| **Editor**   | **CodeMirror 6**         | Verified          | 核心编辑器，Headless 模式。         |
| **Icons**    | **Lucide Icons**         | Verified          | 统一 SVG 图标集。                   |
| **Storage**  | **Redb** (Native)        | Verified          | 嵌入式 KV 数据库 (Zero-copy).       |
| **Auth**     | **Argon2 + Ed25519**     | Verified          | 身份认证与节点签名。                |
| **Diff**     | **Dissimilar**           | Verified          | 文本差异计算算法 (Myers)。              |
|              | **similar**              | Verified          | 辅助 Diff 计算。                        |
|              | ~~Loro~~                 | TBD (远期预研)    | CRDT 框架，当前不依赖。                |
| **CLI**      | **Clap v4**              | Verified          | 命令行解析。                        |
| **Async**    | **Tokio v1**             | Verified          | 异步运行时。                        |
| **Logs**     | **Tracing**              | Verified          | 结构化日志。                        |
| **AI Chat**  | **OpenAI-compatible SSE** | Planned (Native) | 第一方最小 chat 能力，读取 Markdown + 对话。 |
| **Trusted External Agent** | **External CLI Bridge** | Planned (Trusted Only) | 外部 CLI Agent 桥接，可选、默认关闭。 |
| **Graph**    | **d3-force + Pixi.js**   | Planned           | 高性能图谱渲染 (Web Canvas).        |
| **Search**   | **Tantivy** (Rust)       | Planned           | 全文检索、模糊搜索 (Backend).       |
| **Sync**     | **Axum + Tower**         | Verified (Partial) | HTTP 路由成熟；WS 仍持续收紧广播粒度。 |
| **Build**    | **Tauri v2**             | Planned           | 跨平台外壳 (Mobile/Desktop)。       |
| **Plugins**  | **Interface Reserved**   | Planned           | 当前只保留 Trusted External Agent Runtime / Calculation Runtime 接口，不要求实现。 |

## 2. Markdown Compatibility Checklist

*   **导出原则**：通用 GFM，无私有语法。
*   **语法基线**：CommonMark + GFM。
*   **链接约定**：内部 `doc://` <=> 导出相对路径。
*   **资产约定**：`asset://` <=> 导出图片引用。
*   **回归用例**：CI 快照对比。

## 3. Performance Budget & Profiles

### High/Low Profile
*   **Low-Spec (≤768MB)**: CSR Only, No Search Index, Snapshot Pruning.
*   **Standard (≥1GB)**: SSR, Search, Graph.

### Profile → Feature Matrix

| Feature | `low-spec` (≤768MB) | `standard` (≥1GB) |
|:---|:---|:---|
| CSR | ✅ | ✅ |
| SSR | ❌ | ✅ |
| Full-Text Search (Tantivy) | ❌ | ✅ |
| Graph Visualization | ❌ | ✅ |
| Snapshot Depth default | 10 | 100 |
| MEM_CACHE_MB default | 32 | 128 |
| Trusted External Agent Runtime / Calculation Runtime | ❌ | ✅ (Future, interface-only) |

## 4. WASM Memory Constraints

*   **Budget**: 前端 WASM 堆目标 < 64MB (Mobile), < 128MB (Desktop)。
*   **Large Doc Strategy**: 超过 100KB 的文档使用分段加载，不将全文存入 WASM 堆。
*   **Monitoring**: 通过 `wasm_bindgen::memory()` 跟踪实际用量并在 DevTools 输出。

## 5. Related Commands

* 无。

## 6. Related Configuration

*   `DEVE_PROFILE`: `standard` | `low-spec`.
*   `MEM_CACHE_MB`: Memory cache limit.
