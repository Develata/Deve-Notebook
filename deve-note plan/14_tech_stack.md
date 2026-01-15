# 14_tech_stack.md - 技术栈篇 (Technology Stack)

## 技术栈清单 (The Full Stack)

| 层次          | 核心技术                    | 选型理由                 |
| :------------ | :-------------------------- | :----------------------- |
| **语言**      | Rust (2024)                 | 全栈统一。               |
| **前端框架**  | **Leptos v0.7**             | 信号驱动，性能极致。     |
| **UI 组件**   | **Tailwind CSS**            | 原子化 CSS。             |
| **国际化**    | **leptos_i18n**             | 编译时校验。             |
| **编辑器**    | **CodeMirror 6 / Milkdown** | 默认轻核心，可选重扩展。 |
| **图标库**    | **Lucide Icons**            | 统一 SVG。               |
| **图谱渲染**  | **Pixi.js / Cosmic-Graph**  | WebGL 加速。             |
| **存储**      | **Redb/Sled**               | 嵌入式 DB。              |
| **搜索**      | **Tantivy**                 | 全文检索（可选）。       |
| **同步/流控** | **Axum + Tower**            | 背压、限流。             |
| **和解**      | **Notify + Dissimilar**     | 监听与 Diff。            |
| **构建**      | **Tauri v2**                | 跨平台外壳。             |
| **插件**      | **Rhai + Extism**           | Wasm/脚本引擎。          |

    | **插件**      | **Rhai + Extism**           | Wasm/脚本引擎。          |

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
