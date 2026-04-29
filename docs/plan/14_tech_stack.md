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
| **Language** | Rust 1.92 / Edition 2024 | Verified          | 与当前 `Cargo.lock` MSRV 对齐，全栈统一。 |
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
| **MCP**      | **No runtime**   | Retired | 不规划、不保留 MCP runtime；相关需求由 Skills 调用受控 CLI 工具或 Trusted CLI path 承载。 |
| **Graph**    | **Core read-only projection + d3-force/Pixi.js future renderer** | Verified (Projection Baseline) | `deve_core::graph` 只从 repo docs 派生节点/边，不写 ledger authority；高性能 Web Canvas 渲染仍是 future。 |
| **Search**   | **Repo-scoped baseline scan; Tantivy planned** | Verified (Baseline) | Standard + `search` feature 下按当前 repo scope 扫描文档内容；Tantivy 增量索引仍是后续优化。 |
| **Sync**     | **Axum + Tower**         | Verified (Partial) | HTTP 路由成熟；WS 仍持续收紧广播粒度。 |
| **Git Ecosystem** | **First-class mirror bridge** | Partial (Explicit Mirror Executor) | `.notegit/` 保持 authority；`.git/` 作为生态镜像层。当前已落地共存/忽略/status、lazy `git_mirror_commits` side table 与显式单-record executor；projection replay、自动后台执行、export/import/push 仍属 P1/P2 后续。 |
| **Build**    | **Tauri v2**             | Planned (Rising Priority) | Desktop/Mobile native track 逐步提上日程；先明确 adapter、embedded service 与 offline/readiness 边界。 |
| **Plugins**  | **Interface Reserved**   | Planned           | 当前只保留 Trusted External Agent Runtime / Calculation Runtime 接口，不要求实现。 |

### 1.1 Graph Visualization {#graph-visualization}

当前已实现部分只包括 `deve_core::graph` 的只读 projection baseline：它从当前 repo docs 派生节点、已解析边与未解析链接，不读取或写入 ledger authority、workspace、search index 或 source-control runtime。d3-force / Pixi.js 等高性能 Web 渲染仍是 future renderer，不属于当前验收阻塞项。

### 1.2 Search Baseline {#search-baseline}

当前 repo-scoped baseline search 必须保持可禁用、可降级。`search` feature 下允许保留 Tantivy service
作为 Standard profile 的可选索引实现；低配模式不得依赖常驻重型索引，后续增量索引优化不得反向污染
ledger authority 或 repo scope gate。

### 1.3 Git Ecosystem Mirror Bridge {#git-ecosystem-bridge}

Deve 的核心版本管理是 ledger-backed Source Control，不复用 Git object store、Git
index、Git refs 或 `.git/` 目录作为 authority。Git 生态作为 first-class mirror
bridge：projection export、受控 import、backup/publish、远程托管与 release 交付。

当前已实现的 bridge foundation：

- `.git/` 与 `.notegit/` 作为 repo internal path segments 被 watcher / scan / sync / rebuild projection 忽略。
- `materialize` / `rebuild_projection` / `init` / `serve` 会确保 repo-local `.gitignore` 忽略 `.notegit/`。
- `deve_cli git status` 提供只读 mirror 状态骨架：`.git` 是否存在、`.notegit` 是否存在、`.gitignore` 是否保护 `.notegit/`。
- Lazy-created `git_mirror_commits` side table 记录 `DeveCommit -> GitMirrorQueued / GitMirrorCommitted / GitMirrorOutOfSync`，`deve_cli git status` 输出 mirror readiness `state` 与独立 `queue_state`/queued/committed/out_of_sync summary。
- `deve_cli git mirror` 可显式执行单个 queued/out_of_sync record；执行前会检查 Git worktree、`.notegit` tracked 泄漏、Source Control pending/staged 清洁度与当前 Git changed paths 是否属于该 Deve commit diff 或 `.gitignore`，通过后才运行 `git add -A` / `git commit` 并写回 Git commit hash。多个积压 records fail closed 为 `GitMirrorOutOfSync`，避免伪造逐 commit 映射。
- Projection replay、自动后台执行、完整 retry/repair UI、import/export/push 仍是后续实现，不得被当前 executor 骨架替代。

该 bridge 的工程边界：

- `.notegit/` 与 ledger/source-control tables 是唯一业务真相。
- `.git/` 可以与 `.notegit/` 共存，repo-local `.gitignore` 必须忽略 `.notegit/`。
- Git mirror 以 Deve commit/projection 为粒度同步，不镜像 `.notegit/` 的内部 side-table 操作。
- Git mirror failure 只产生 `GitMirrorOutOfSync` 与 retry/repair 需求，不回滚 Deve commit。
- 任何 Git import 都必须生成 Deve ledger facts；任何 Git export 都不得反向改写 ledger authority。

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
