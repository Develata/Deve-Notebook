# 当前下一步任务

> 更新日期：2026-04-29
>
> 本文件只记录 active execution queue。已完成的实现历史应进入 dated reports，例如
> `code-review-2026-04-28.md` 与 `release-smoke-status-2026-04-28.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Desktop packaging scaffold plan split | P3-10 | `apps/desktop/`, `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/14_tech_stack.md`, native packaging checks | 在 `apps/desktop` `native-packaging` feature 后定义首个真实 packaging dependency 批次和验收口径；默认 no-packaging skeleton 仍通过测试，packaging 不得获得 core authority。 |

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
- P1/P2 Git import UI/conflict polish 已关闭：Command Palette 已提供 `Git: Import Changes` / `Git: Push Mirror` CLI-only notice，Source Control notice 明确 CLI 命令与 blocker 边界，import conflict 条目提示暂存前选择保留文件系统或账本版本。
- P1/P2 Git push blocker/remote polish 已关闭：CLI push 输出与 Web CLI-only notice 已覆盖 remote/upstream、显式 `--remote/--branch`、export/repair、dirty Git worktree/import 与 dirty Deve Source Control blocker 提示；Web 仍不直接执行 Git push。
- P1/P2 Git mirror failure metadata polish 已关闭：`GitMirrorOutOfSync` 兼容字段已记录 `failure_subject`、`failure_command`、`failure_exit_status`，CLI record 明细输出 `failure_meta[...]`，旧记录仍按缺省字段反序列化。
- P1/P2 Git mirror repair-action scope decision 已关闭：`GitMirrorRepairAction` 当前为 CLI-only 诊断 schema，输出 action code / subject / retryable-after-fix，不自动执行 Git，不授权 Web/后台直接写 Git。
- P1/P2 Rendering current/future split 已关闭：`03_rendering` plan/features 已区分当前 editor adapter、lightweight Markdown renderer、大文档批量调度基础设施与 future preview/virtual-render/settings；`render_markdown` 补充 HTML allowlist、secure link 与 unsupported syntax 测试。
- P3-10 Desktop/Mobile native adapter core contract 已关闭：`08_ui_design_02_desktop` 与 `08_ui_design_03_mobile` 已明确 minimal adapter contract，`deve_core::native_adapter` 已落地平台无关状态/事件/endpoint/session/readiness 合同与定向测试；Tauri desktop/mobile shell、embedded service launcher 与 Web bootstrap 消费仍属后续实现。
- P3-10 Web native bootstrap 消费已关闭：Web connection manager 可读取 `window.__DEVE_NATIVE_BOOTSTRAP`，复用 core native endpoint/session 校验，有效时只使用注入 endpoint，失效时 fail-closed 且不回退端口推断；浏览器默认路径保持不变，详见 `native-web-bootstrap-status-2026-04-29.md`。
- P3-10 Server native-safe launch surface 已关闭：新增 `ServerLaunchOptions` 与 hidden `serve --native-loopback` 路径，native 模式只绑定 `127.0.0.1`、占用端口 fail-closed、不进入 proxy fallback，`/api/node/role` 暴露 nullable `native_service` endpoint/session surface；普通 release/Docker `0.0.0.0` 行为保持不变，详见 `native-server-launch-status-2026-04-29.md`。
- P3-10 Desktop native shell skeleton 已关闭：新增 `apps/desktop` 无 Tauri 依赖骨架，固定受控 endpoint、session 绑定、Web bootstrap 注入、service offline 与 session invalid recovery 状态机；真实 Tauri packaging/菜单/托盘/安装包仍为 future，详见 `desktop-native-shell-status-2026-04-29.md`。
- P3-10 Mobile native shell skeleton 已关闭：新增 `apps/mobile` 无 Tauri Mobile 依赖骨架，固定受控 endpoint、session 绑定、Web bootstrap 注入、background/suspended/resumed/foreground reprobe、service offline 与 session invalid recovery 状态机；移动生命周期事件只作为 reprobe hint，不授予写权限，详见 `mobile-native-shell-status-2026-04-29.md`。
- P3-10 Native runtime readiness UI recovery polish 已关闭：Web 端新增 native bootstrap invalid/session pending/service offline/foreground reprobe 结构化状态，header/bottom bar/mobile footer/overlay/Source Control gate 已显示明确恢复语义；desktop/mobile skeleton 可输出不含 secret/reason 的 recovery bootstrap，详见 `native-web-recovery-status-2026-04-29.md`。
- P3-10 Native packaging dependency gate 已关闭：`apps/desktop` 与 `apps/mobile` 声明 `native-packaging` no-op future gate，默认构建保持 no-Tauri skeleton，`check-native-track-boundary.sh` 会阻止 packaging dependency/import 泄漏到 workspace root、core、cli、web 或未开启门禁的 native crates，详见 `native-packaging-dependency-gate-2026-04-29.md`。
- P3-13 Graph visualization read-only CLI projection surface 已关闭：`deve_core::graph` 保持 authority-free projection helper，`deve graph` 只读导出 repo-scoped `GraphProjection` JSON，默认 fail-closed 于损坏 Structure Facts authority；Web Canvas/d3-force/Pixi renderer 仍属 future implementation。
- P2 Docker release smoke 已关闭：Docker Desktop WSL integration 恢复后，`DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh` 完整通过，镜像 build、容器启动与宿主 `/api/node/role` endpoint probe 均已验证；脚本同时补充 local proxy bypass 与容器 health 诊断，详见 `release-smoke-status-2026-04-29.md`。
- P3 Cargo-chef manifest warning triage 已关闭：当前 repo manifests 无 `plugin = ...` 键，`cargo metadata --no-deps --format-version 1` 无 warning；该 warning 需在稳定 Docker context 内复现后再判断是否为 cargo-chef skeleton 或旧缓存噪音，详见 `cargo-chef-warning-triage-2026-04-29.md`。
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
