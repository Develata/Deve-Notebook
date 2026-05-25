# 17_tech_stack.md - 技术栈篇 (Technology Stack)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Reference`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-24`
- `Counterpart Feature`: `docs/features/14_tech_stack.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md`
- `Primary Code Areas`: `Cargo.toml`, `apps/web/Cargo.toml`, `apps/cli/Cargo.toml`, `apps/desktop/Cargo.toml`, `apps/mobile/Cargo.toml`, `scripts/check-native-track-boundary.sh`

## 1. Technology Stack

| **Layer**    | **Technology**           | **Decision**      | **Selection Reasoning**             |
| :----------- | :----------------------- | :---------------- | :---------------------------------- |
| **Language** | Rust 1.92 / Edition 2024 | Selected          | 与 workspace MSRV 对齐，全栈统一。 |
| **Frontend** | **Leptos v0.7**          | Selected          | 信号驱动 (Signals)，性能极致。      |
| **UI**       | **Tailwind CSS**         | Selected          | 原子化 CSS。                        |
| **Router**   | **leptos_router**        | Selected          | 前端路由管理。                      |
| **I18n**     | **自研 `Locale + t::*`** | Selected          | 轻量、零代码生成、调用面稳定。      |
| **Editor**   | **CodeMirror 6**         | Selected          | 核心编辑器，Headless 模式。         |
| **Icons**    | **Lucide Icons**         | Selected          | 统一 SVG 图标集。                   |
| **Storage**  | **Redb** (Native)        | Selected          | 嵌入式 KV 数据库 (Zero-copy).       |
| **Auth**     | **Argon2 + Ed25519**     | Selected          | 身份认证与节点签名。                |
| **Diff**     | **Dissimilar**           | Selected          | 文本差异计算算法 (Myers)。              |
|              | **similar**              | Selected          | 辅助 Diff 计算。                        |
|              | ~~Loro~~                 | Research          | CRDT 框架，baseline 不依赖。                |
| **CLI**      | **Clap v4**              | Selected          | 命令行解析。                        |
| **Async**    | **Tokio v1**             | Selected          | 异步运行时。                        |
| **Logs**     | **Tracing**              | Selected          | 结构化日志。                        |
| **AI Chat**  | **OpenAI-compatible SSE** | Native Baseline | 第一方最小 chat 路径；必须保持 read-first、低常驻成本与 PLAN/BUILD 模式边界。 |
| **Trusted External Agent** | **External CLI Bridge** | Optional Trusted Path | 外部 CLI Agent 仅作为显式启用的受信任路径；默认关闭，不属于通用插件市场能力。 |
| **MCP**      | **No runtime**   | Retired | 不规划、不保留 MCP runtime；相关需求由 Skills 调用受控 CLI 工具或 Trusted CLI path 承载。 |
| **Graph**    | **Read-only projection + optional renderer gate** | Deferred Renderer | Core 只读派生 graph projection；高内存 renderer 必须作为独立性能批次启用。 |
| **Search**   | **Repo-scoped baseline scan; Tantivy optional** | Baseline + Optional Index | 低配默认不得依赖常驻重型索引；Tantivy 仅作为 feature-gated 优化路径。 |
| **Sync**     | **Axum + Tower**         | Core Runtime | HTTP/WS runtime 必须遵守第 5 章协议、repo scope 与结构化错误合同。 |
| **Git Ecosystem** | **First-class mirror bridge** | Mirror Bridge | `.notegit/` 保持 authority；`.git/` 只作为生态镜像层，详见第 4 与第 7 章。 |
| **Build**    | **Tauri v2**             | Native Target / Deferred Packaging | Desktop/Mobile 目标采用 Tauri v2；真实 packaging 依赖必须经过 native-packaging gate。 |
| **Plugins**  | **Compatibility Host + Interface Reserved** | Boundary Reserved | 保留 Rhai/plugin-host 兼容边界与 Trusted CLI/Calculation future 接口；不要求插件市场或完整扩展平台。 |

### 1.1 图谱可视化 {#graph-visualization}

Graph 技术路线分为只读 projection 与可选 renderer 两层：

- Core graph projection 只能从当前 repo 的 Markdown projection 派生节点、已解析边与未解析链接。
- CLI/HTTP adapter 只能导出只读 graph projection JSON，不得写 ledger、workspace、search index、source-control state 或 `.git/.notegit`。
- Web renderer gate 默认关闭；summary、count 或只读 review UI 不等价于高性能图渲染器。
- 间接前端依赖（例如由 Mermaid 带来的 d3 包）不得被解释为 Graph renderer gate 已显式启用。

未来若重新打开 Graph renderer gate，必须作为独立 dependency/performance batch 处理，并满足：

- 依赖不得进入 `deve_core`、`deve_cli` 或 authority path。
- 默认 low-spec profile 不得启用常驻 layout worker 或高内存 renderer。
- Renderer 只能消费 `/api/repo/graph` 只读 projection，不得写 ledger、workspace、search
  index、source-control state 或 `.git/.notegit`。
- 交互状态必须 repo-scoped，并 fail-closed 于 stale `repo_id`、`branch` 或 `scope_nonce`。
- 验收必须包含大图降级策略、加载失败 fallback 与无 renderer 环境 fallback。

### 1.2 搜索基线 {#search-baseline}

Search 技术路线必须可禁用、可降级：

- Baseline search **MUST** 绑定当前 `repo_id/branch/scope_nonce`，不得跨 repo 复用结果。
- 未启用 `search` feature、低配 profile、缺失或过期 scope nonce 时，search **MUST** fail-closed 为结构化错误。
- 前端只接受 request id、repo、branch 与 scope nonce 同时匹配的结果；不匹配结果 **MUST** 丢弃。
- Tantivy 只能作为 feature-gated 优化路径；低配默认 **MUST NOT** 依赖常驻重型索引。
- Search index **MUST NOT** 成为 ledger、source-control 或 repo scope authority。

### 1.3 Git 生态镜像桥 {#git-ecosystem-bridge}

Deve 的核心版本管理是 ledger-backed Source Control；不把 Git object store、index、refs 或 `.git/` 作为 authority。Git 生态只作为 first-class mirror bridge：projection export、受控 import、backup/publish、远程托管与 release 交付。

该 bridge 的工程边界：

- `.notegit/` 与 ledger/source-control tables 是唯一业务真相。
- `.git/` 可以与 `.notegit/` 共存，repo-local `.gitignore` 必须忽略 `.notegit/`。
- Git mirror 以 Deve commit/projection 为粒度同步，不镜像 `.notegit/` 的内部 side-table 操作。
- Git mirror failure 只产生 `GitMirrorOutOfSync` 与 retry/repair 需求，不回滚 Deve commit。
- Git import 只能进入 pending/import；只有后续 Deve stage/commit 才能生成 ledger facts。
- Git export 与 Git push 不得反向改写 ledger authority。
- Web/后台不得从只读 status/review surface 隐式升级为 Git writer；任何可执行 Git repair UI 都必须另立设计批次，并要求人工确认。

### 1.4 原生打包依赖门禁 {#native-packaging-dependency-gate}

Native track 默认构建必须无 packaging runtime 依赖。Desktop/Mobile native adapter 只固定 endpoint/session/bootstrap/lifecycle contract；真实 packaging runtime 只能在后续批次引入，并满足以下门禁：

- 依赖只允许落在对应 Desktop/Mobile native adapter crate，不得进入 workspace root
  dependency、authority core、server runtime 或 web runtime。
- 依赖必须挂在对应 crate 的 `native-packaging` feature 后；默认 feature set
  仍编译 no-packaging skeleton。
- `native-packaging` 不得授予 ledger/Projection Workspace/source-control/search/`.git`/`.notegit`
  authority；业务真相仍在 core/server。
- packaging 验收只覆盖窗口、菜单、托盘、权限、安装包、auto-update、移动平台
  bridge 等壳层能力；adapter/session/readiness correctness 继续由 no-packaging
  skeleton unit tests 保证。
- 每次引入或升级 packaging dependency 都必须更新
  `scripts/check-native-track-boundary.sh`、Desktop/Mobile plan 与对应评审报告。

Gate 状态：

- Desktop packaging dependency spike 已打开：`tauri` / `tauri-build` **MAY** 只作为 `apps/desktop`
  的 optional dependency 存在，并且必须挂在 `native-packaging` feature 后。
- Desktop 默认构建仍 **MUST** 保持 no-Tauri；Desktop packaging dependency spike 不等价于
  Desktop release ready。
- Mobile packaging dependency spike 已打开：`tauri` / `tauri-build` **MAY** 只作为 `apps/mobile`
  的 optional dependency 存在，并且必须挂在 `native-packaging` feature 后。
- Mobile packaging scaffold 只记录 WebView shell/permission bridge/share sheet/deeplink/file picker/push notification/store package acceptance；Android shell-only package execution 可由 `11_ui_design/03_mobile.md#mobile-android-shell-package-execution-gate` 单独打开。
- iOS shell-only package execution 可由 `11_ui_design/03_mobile.md#mobile-ios-shell-package-execution-gate` 单独打开；Mobile runtime entrypoint、process runtime、native authority write path 与 release ready 不得由 Android/iOS package execution 隐式打开。
- Mobile foreground/background reprobe 与 session/readiness correctness 继续由 no-packaging skeleton tests 保证。
- Native embedded service supervision 按 no-runtime contract 处理；该 contract 不启动真实子进程、不依赖 Tauri runtime capability、不授予 native shell core authority。
- `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY = DesktopAndMobileDependencySpikeOpen`；
  Tauri dependency 只允许在对应 native crate 的 `native-packaging` scope 内出现。
- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY = DeferredUntilPackagingGate`；`child_process_runtime_enabled = false`、`packaging_gate_required = true`、`authority_writes_allowed = false`。
- 后续 child-process runtime 必须在对应 native crate 的 `native-packaging` feature 后实现，并继续禁止 core authority writes。

## 2. Markdown Compatibility Checklist

*   **导出原则**：通用 GFM，无私有语法。
*   **语法基线**：CommonMark + GFM。
*   **链接约定**：内部 `doc://` <=> 导出相对路径。
*   **资产约定**：`asset://` <=> 导出图片引用。
*   **回归用例**：CI 快照对比。

## 3. Performance Budget & Profiles

### High/Low Profile
*   **Low-Spec (≤768MB)**: CSR Only, No Search Index, Snapshot Pruning.
*   **Standard (≥1GB)**: SSR, Search, Graph projection.

### Profile → Feature Matrix

| Feature | `low-spec` (≤768MB) | `standard` (≥1GB) |
|:---|:---|:---|
| CSR | ✅ | ✅ |
| SSR | ❌ | ✅ |
| Full-Text Search (Tantivy) | ❌ | ✅ |
| Read-only Graph Projection | ✅ | ✅ |
| High-performance Graph Renderer | ❌ | Default off / feature-gated |
| Snapshot Depth default | 10 | 100 |
| MEM_CACHE_MB default | 32 | 128 |
| Trusted External Agent Bridge | ❌ | Interface reserved / default off |
| Calculation Runtime | ❌ | Interface reserved / no runtime |

## 4. WASM Memory Constraints

*   **Budget**: 前端 WASM 堆目标 < 64MB (Mobile), < 128MB (Desktop)。
*   **Large Doc Strategy**: 超过 100KB 的文档使用分段加载，不将全文存入 WASM 堆。
*   **Monitoring**: 通过 `wasm_bindgen::memory()` 跟踪实际用量并在 DevTools 输出。

## 5. Related Commands

* 无。

## 6. Related Configuration

*   `DEVE_PROFILE`: `standard` | `low-spec`.
*   `MEM_CACHE_MB`: Memory cache limit.
