# 第六章：技术栈清单 (The Full Stack)

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

**核心数据结构**：`DocId(Uuid)`, `LedgerEntry`, `CapabilityManifest`。

### Markdown 兼容性与回归清单

*   **导出原则**：通用 GFM，无私有语法。
*   **语法基线**：CommonMark + GFM。
*   **链接约定**：内部 `doc://` <=> 导出相对路径。
*   **资产约定**：`asset://` <=> 导出图片引用。
*   **回归用例**：CI 快照对比。

### 性能预算与极致瘦身

*   **前端策略**：按需加载，虚拟化渲染。
*   **体积控制**：Code splitting, Tree-shaking, LTO。
*   **内存预算**：空闲 < 150MB。
*   **High/Low Profile**：
    *   **Low-Spec (512MB)**: CSR Only, No Search Index, Snapshot Pruning.
    *   **Standard (1GB+)**: SSR, Search, Graph.

### Server Configuration Profiles

| 环境变量         | 默认       | 512MB 模式 | 说明       |
| :--------------- | :--------- | :--------- | :--------- |
| `DEVE_PROFILE`   | `standard` | `low-spec` | 一键预设   |
| `FEATURE_SSR`    | `true`     | `false`    | 服务端渲染 |
| `FEATURE_SEARCH` | `true`     | `false`    | 全文搜索   |
| `FEATURE_GRAPH`  | `true`     | `false`    | 图谱分析   |
| `MEM_CACHE_MB`   | `128`      | `32`       | 缓存上限   |
| `SYNC_MODE`      | `auto`     | `auto`     | 同步模式   |
