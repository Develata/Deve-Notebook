# 14_tech_stack.md - 技术栈篇 (Technology Stack)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Reference`
- `Counterpart Feature`: `docs/features/14_tech_stack.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md`
- `Primary Code Areas`: `Cargo.toml`, `apps/web/Cargo.toml`, `apps/cli/Cargo.toml`, `apps/desktop/Cargo.toml`, `apps/mobile/Cargo.toml`, `scripts/check-native-track-boundary.sh`

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
| **Graph**    | **Core read-only projection + CLI JSON surface + d3-force/Pixi.js future renderer** | Verified (Projection Surface) | `deve_core::graph` 只从 repo docs 派生节点/边，不写 ledger authority；`deve graph` 只读导出 projection JSON；高性能 Web Canvas 渲染仍是 future。 |
| **Search**   | **Repo-scoped baseline scan; Tantivy planned** | Verified (Baseline) | Standard + `search` feature 下按当前 repo scope 扫描文档内容；Tantivy 增量索引仍是后续优化。 |
| **Sync**     | **Axum + Tower**         | Verified (Partial) | HTTP 路由成熟；WS 仍持续收紧广播粒度。 |
| **Git Ecosystem** | **First-class mirror bridge** | Partial (Explicit Mirror Replay + Import/Push CLI + Web CLI Notices) | `.notegit/` 保持 authority；`.git/` 作为生态镜像层。当前已落地共存/忽略/status、lazy `git_mirror_commits` side table、结构化 failure stage、显式单-record executor、多-record projection replay、queued export、import apply、push CLI surface 与 Web import/push/repair CLI-only notices；自动后台执行与完整 UI 仍属 P1/P2 后续。 |
| **Build**    | **Tauri v2**             | Partial Skeleton / Planned Packaging | `apps/desktop` 与 `apps/mobile` 已有无 Tauri 依赖的 native shell skeleton，用于固定 adapter/bootstrap/offline/lifecycle 边界；真实 Tauri v2 packaging、菜单、托盘、移动权限、安装包与 auto-update 仍是 future。 |
| **Plugins**  | **Interface Reserved**   | Planned           | 当前只保留 Trusted External Agent Runtime / Calculation Runtime 接口，不要求实现。 |

### 1.1 Graph Visualization {#graph-visualization}

当前已实现部分包括 `deve_core::graph` 的只读 projection baseline 与 `deve graph`
CLI JSON surface：core graph 从当前 repo docs 派生节点、已解析边与未解析链接，不读取或写入
ledger authority、workspace、search index 或 source-control runtime；CLI adapter 只负责从当前
repo 的文档 projection 重建 `GraphDocument`，调用 `project_documents` 并输出 JSON。d3-force /
Pixi.js 等高性能 Web 渲染仍是 future renderer，不属于当前验收阻塞项。

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
- Lazy-created `git_mirror_commits` side table 记录 `DeveCommit -> GitMirrorQueued / GitMirrorCommitted / GitMirrorOutOfSync`，并为 out-of-sync 记录持久化结构化 `failure_stage`（旧记录缺字段时 CLI 只作兼容性 fallback）；`deve_cli git status` 输出 mirror readiness `state`、独立 `queue_state`/queued/committed/out_of_sync summary、per-commit lagging records、`queued_lag_ms` / `updated_lag_ms`、失败位置与 retry command hint。
- `deve_cli git mirror` 可显式执行 queued/out_of_sync records；单个 record 走 worktree preflight 后的 `git add -A` / `git commit`，多个 records 走临时 Git index 的 projection replay，用 `commit-tree` / `update-ref` 按 Deve commit diff 生成逐 commit Git history，并写回 Git commit hash；执行报告会输出 per-record outcome、失败位置与 repair/retry hint。
- `GitMirrorOutOfSync` 当前暴露 CLI-only `GitMirrorRepairAction` schema，把 failure stage / legacy error 映射为 repair action code、subject 与 retryable-after-fix 标记；CLI record 明细同时输出 `repair_guidance[...]`，给出 `manual_only=yes`、具体 next step 与 retry command。该 schema 只用于诊断与显式 CLI retry 指引，不自动执行 Git。
- `deve_cli git export` 复用该 executor 作为 queued projection export surface，输出 `git_export[...]` 报告与 export/retry hint；side table 为空且 Git history 为空时，会从最新 Deve commit 的完整 projection 建立首个 snapshot Git commit，只把最新 Deve commit 映射到该 Git commit，后续增量 commit 再以该映射为 parent replay。
- `deve_cli git import` 当前提供只读 dry-run planning：解析 Git tracked/untracked worktree changes 为 change/blocker；`deve_cli git import --apply` 在无 blocker 时把安全 changes 原子写入 `pending_fs_ops`，并通过 `has_conflict` 保留冲突标记。该 surface 不得被解释为 ledger commit 已完成。
- `deve_cli git push` 当前提供显式 mirror publish surface：只在 `.git` mirror ready、Source Control clean、Git worktree clean、无 queued/out_of_sync mirror record 且当前 Git HEAD 映射到最新 `GitMirrorCommitted` record 时执行远端 push；失败以 blocker 输出，不回滚 ledger，也不写 `.notegit`。
- Web Command Palette 当前只提供 Git import / push / repair 的 CLI-only notices：repair notice 指向 `deve_cli git status --repo <repo>` 的 `repair_action[...]` 与 `deve_cli git export --repo <repo> --retry-out-of-sync`，不直接调用 Web 后端执行 Git。
- 自动后台执行、完整 repair UI 与 Web 后端直接执行 Git import/push/repair 仍是后续实现，不得被当前 executor、import apply、push surface 或 CLI-only notices 替代。

该 bridge 的工程边界：

- `.notegit/` 与 ledger/source-control tables 是唯一业务真相。
- `.git/` 可以与 `.notegit/` 共存，repo-local `.gitignore` 必须忽略 `.notegit/`。
- Git mirror 以 Deve commit/projection 为粒度同步，不镜像 `.notegit/` 的内部 side-table 操作。
- Git mirror failure 只产生 `GitMirrorOutOfSync` 与 retry/repair 需求，不回滚 Deve commit。
- 任何 Git import 都必须生成 Deve ledger facts；任何 Git export 都不得反向改写 ledger authority。

### 1.4 Native Packaging Dependency Gate {#native-packaging-dependency-gate}

当前 native track 的默认构建必须保持无 packaging runtime 依赖。`apps/desktop`
与 `apps/mobile` 的职责是固定 endpoint/session/bootstrap/lifecycle contract；真实
packaging runtime 只能在后续批次引入，并必须满足以下门禁：

- 依赖只允许落在 `apps/desktop` 或 `apps/mobile`，不得进入 workspace root
  dependency、`deve_core`、`deve_cli` 或 `deve_web`。
- 依赖必须挂在对应 crate 的 `native-packaging` feature 后；默认 feature set
  仍编译 no-packaging skeleton。
- `native-packaging` 不得授予 ledger/vault/source-control/search/`.git`/`.notegit`
  authority；业务真相仍在 core/server。
- packaging 验收只覆盖窗口、菜单、托盘、权限、安装包、auto-update、移动平台
  bridge 等壳层能力；adapter/session/readiness correctness 继续由 no-packaging
  skeleton unit tests 保证。
- 每次引入或升级 packaging dependency 都必须更新
  `scripts/check-native-track-boundary.sh`、Desktop/Mobile plan 与 dated report。

Desktop packaging scaffold 当前已作为 `apps/desktop` 的 feature-gated code surface
存在：它声明 planned `tauri` / `tauri-build` dependency batch 与 window/menu/tray/
installer/auto-update acceptance，但仍不引入实际 Tauri dependency。该 scaffold
只能作为下一批 dependency decision 的输入，不得被解释为当前已具备 native packaging。

Mobile packaging scaffold 当前已作为 `apps/mobile` 的 feature-gated code surface
存在：它声明 planned `tauri` / `tauri-build` dependency batch 与 WebView shell/
permission bridge/share sheet/deeplink/file picker/push notification/store package
acceptance，但仍不引入实际 Tauri Mobile dependency。移动端 foreground/background
lifecycle reprobe、session/readiness correctness 继续由 no-packaging skeleton tests
保证，packaging 不得取得业务 authority。

Native embedded service supervision 当前是 no-runtime contract：`deve_core::native_adapter`
提供 `NativeServiceSupervisor`、health probe、retry budget 与 session handoff failure
分类；`apps/desktop`、`apps/mobile` 与 native loopback launch surface 复用该 contract。
它不启动真实子进程，不引入 Tauri dependency，也不授予 native shell 任何 core authority。

Native packaging dependency gate 当前仍是 closed/deferred：
`CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY` 固定为
`DeferredUntilRuntimeBatch`，真实 `tauri` / `tauri-build` dependency 不允许进入当前
workspace 构建。`apps/desktop` 与 `apps/mobile` 的 packaging scaffold 只记录 planned
capabilities 与 forbidden authorities；它不是 gate 已打开的证明。

真实 native process adapter 当前被显式推迟到 packaging gate 之后：
`CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` 固定为 `DeferredUntilPackagingGate`，
`child_process_runtime_enabled = false`、`packaging_gate_required = true`、
`authority_writes_allowed = false`。这表示默认 desktop/mobile skeleton 只验证
adapter/session/bootstrap/readiness contract；后续如需真实 child-process runtime，
必须在对应 native crate 的 `native-packaging` feature 后实现，并继续禁止 core
authority writes。

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
