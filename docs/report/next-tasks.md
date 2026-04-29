# 当前下一步任务

> 更新日期：2026-04-29
>
> 本文件只记录 active execution queue。已完成的实现历史应进入 dated reports，例如
> `code-review-2026-04-28.md` 与 `release-smoke-status-2026-04-28.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Git import UI/conflict polish | P2 | Command Palette, Source Control panel, import conflict copy | CLI apply 已写入 pending/import；下一步补 UI 入口、blocker/conflict 文案与验收用例。 |
| 2 | Git push Command Palette polish | P2 | Command Palette, Source Control push copy, push blocker UI | CLI push 已能发布 `.git` mirror；下一步补 UI 入口、远端配置提示与 blocker 文案。 |
| 3 | Docker release smoke rerun | P2 (Host-blocked) | Docker host environment, `scripts/smoke-docker-release.sh` | 2026-04-29 重跑仍因 WSL Docker 不可用阻塞；Docker daemon 可用后重跑，环境阻塞继续与代码失败分开记录。 |

## 最近完成基线

- P0 sync vector wire contract 与 browser storage/degraded write boundary 已关闭：`DEVEWSF3`、显式 `known_vector/server_vector`、Web degraded read-only/write gate 均已测试。
- P1 security hardening small batch 已关闭：`identity.key` owner-only、login audit `timestamp/user_agent`、CORS wildcard fail-closed、dev-only auth/CORS warnings 均已测试。
- P1 path normalization cleanup 已关闭：runtime forward-slash normalization 已集中到 `deve_core::utils::path`，剩余 `replace('\\', "\\\\")` 是测试脚本文字串转义而非路径归一化。
- P1/P2 Git mirror bridge foundation 已关闭：`.git/.notegit` internal path segment 过滤、repo-local `.gitignore` 保护 `.notegit/`、只读 `deve_cli git status` 骨架与定向测试已落地；真实 mirror commit/import/export/push 的 CLI surface 已落地，剩余为 UI/repair polish。
- P1/P2 Git mirror queue/status foundation 已关闭：lazy `git_mirror_commits` side table、`GitMirrorQueued / Committed / OutOfSync` 持久化 API、Deve commit 成功后 mirror-ready queue、`deve_cli git status` 独立 `queue_state` summary 与定向测试已落地；后台自动执行与更完整 repair UI 仍在 active queue 外作为后续 polish。
- P1/P2 Git mirror explicit executor 已关闭：`deve_cli git mirror` 可在 worktree/preflight 通过后显式执行单个 queued/out_of_sync record 的 `git add -A` / `git commit`，成功写回 Git hash，失败写入 `GitMirrorOutOfSync`。
- P1/P2 Git mirror projection replay repair 已关闭：多个 queued/out_of_sync records 可通过临时 Git index 与 `commit-tree` / `update-ref` 从 Deve commit diff 生成逐 commit Git history；失败只标记剩余 records 为 `GitMirrorOutOfSync`。
- P1/P2 Git mirror repair/status detail polish 已关闭：`git status` 输出 per-commit lagging records、`queued_lag_ms` / `updated_lag_ms`、结构化 `failure_stage` 与 retry command；`git mirror` 输出 no-op / repair / retry hint。更细的 failure subject / offending path / command exit metadata 仍保留为 future refinement。
- P1/P2 Git mirror queued export surface 已关闭：`deve_cli git export [--retry-out-of-sync]` 复用 explicit mirror executor，把 queued Deve projection commits 导出为 Git commits 并写回映射。
- P1/P2 Git mirror snapshot bootstrap export 已关闭：当 side table 为空且 Git history 为空时，`git export` 可从最新 Deve commit 的完整 projection 建立首个 Git commit 并映射最新 Deve commit；CLI push 已作为独立 publish surface 落地。
- P1/P2 Git import dry-run planning 已关闭：`deve_cli git import` 会只读检查 ready Git mirror/Git HEAD，并把 Git tracked/untracked worktree changes 输出为 change/blocker；它不写 ledger、pending_fs、staging 或 `.notegit`。
- P1/P2 Git import apply surface 已关闭：`deve_cli git import --apply` 在无 blocker 时把安全 Git changes 写入 `pending_fs_ops`，保留 `has_conflict`，并继续要求后续 Deve stage/commit；它不写 ledger、`StagedEntry` 或 `.notegit`。
- P1/P2 Git push mirror surface 已关闭：`deve_cli git push [--remote] [--branch]` 只发布已映射的 `.git` mirror HEAD，fail-closed 于未导出/失败 mirror record、脏 Git worktree、未映射 HEAD 或 remote/branch 配置错误；它不写 ledger 或 `.notegit`。
- P1/P2 Rendering current/future split 已关闭：`03_rendering` plan/features 已区分当前 editor adapter、lightweight Markdown renderer、大文档批量调度基础设施与 future preview/virtual-render/settings；`render_markdown` 补充 HTML allowlist、secure link 与 unsupported syntax 测试。
- P3-10 Desktop/Mobile native adapter plan 已关闭：`08_ui_design_02_desktop` 与 `08_ui_design_03_mobile` 已明确 minimal adapter contract、boot/lifecycle state machine、endpoint/session injection、offline/readiness 语义与 native forbidden shortcuts；Tauri native shell 代码仍属 future implementation。
- P3-13 Graph visualization read-only CLI projection surface 已关闭：`deve_core::graph` 保持 authority-free projection helper，`deve graph` 只读导出 repo-scoped `GraphProjection` JSON，默认 fail-closed 于损坏 Structure Facts authority；Web Canvas/d3-force/Pixi renderer 仍属 future implementation。
- P2 Docker release smoke 已于 2026-04-29 重跑前置检查：`docker info` 在当前 WSL 中提示 Docker command/daemon 不可用，`scripts/smoke-docker-release.sh` 在 required 模式下仍停在 daemon unreachable；代码 gate 已通过，详见 `release-smoke-status-2026-04-29.md`。
- P0 repo health、`repair --check`、WS structured errors、writer-ready `repo_id + scope_nonce`、Source Control doc identity hardening 已记录在 `code-review-2026-04-28.md`。
- P1 search、settings current boundary、Native AI Chat minimum、graph projection、i18n cleanup、plan_ref sweeps 已记录在 `code-review-2026-04-28.md`。
- Release/runtime smoke 与 Docker daemon blocker 已记录在 `release-smoke-status-2026-04-28.md`。
- File cohesion 与 line-count policy 已记录在 `soft-size-audit-2026-04-27.md`。

## MCP 方向

产品 MCP runtime 已退役。当前扩展方向是 Skills 加显式 trusted controlled CLI path。docs 中的 MCP 只允许表示退役说明，或表示 Chrome MCP 浏览器手工验收工具。

除非重新打开 plan，不要新增 MCP runtime、MCP server management、MCP tool loop 或 MCP-backed Native AI capability。

## 旧分支概览

2026-02-28 的 Branch A-E 拆解已退役。不要恢复旧 checkbox 作为 active TODO。

历史映射：

- 旧 A UI token/component 工作并入 P2 UI/design debt。
- 旧 B dashboard 工作由 runtime observability 与 `/api/node/role` 替代。
- 旧 C E2EE/WebCrypto/IndexedDB 工作并入 browser storage boundary。
- 旧 D plugin/AI 收缩为 Rhai plugin host 与 Native AI Chat minimum；MCP 退役。
- 旧 E docs sync 由 dated baselines 与当前短队列替代。
