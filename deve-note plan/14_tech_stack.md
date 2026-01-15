# 14_tech_stack.md - 技术栈篇 (Technology Stack)

## 技术栈清单 (The Full Stack)

| **Layer**    | **Technology**                        | **Selection Reasoning**             |
| :----------- | :------------------------------------ | :---------------------------------- |
| **Language** | Rust (2024)                           | 全栈统一。                          |
| **Frontend** | **Leptos v0.7**                       | 信号驱动 (Signals)，性能极致。      |
| **UI**       | **Tailwind CSS**                      | 原子化 CSS。                        |
| **I18n**     | **leptos_i18n**                       | 编译时类型安全校验。                |
| **Editor**   | **CodeMirror 6**                      | 核心编辑器，Headless 模式。         |
| **Icons**    | **Lucide Icons**                      | 统一 SVG 图标集。                   |
| **Graph**    | **Pixi.js** (Web) / **Cosmic** (Rust) | 高性能图谱渲染。                    |
| **Storage**  | **Redb** (Native)                     | 嵌入式 KV 数据库 (Zero-copy).       |
| **Search**   | **Tantivy**                           | 全文检索、模糊搜索。                |
| **Sync**     | **Axum + Tower** (Back)               | HTTP/WebSocket 背压与流控。         |
| **Auth**     | **Argon2 + JWT**                      | 标准化身份认证与会话管理。          |
| **Diff**     | **Dissimilar** (Myer's)               | 文本差异计算算法。                  |
| **Build**    | **Tauri v2**                          | 跨平台外壳 (Mobile/Desktop)。       |
| **Plugins**  | **Rhai + WASM (Extism)**              | 双层插件体系 (Scripting + Binary)。 |

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
