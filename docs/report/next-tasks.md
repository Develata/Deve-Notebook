# 当前下一步任务

> 更新日期：2026-04-30
>
> 本文件只记录 active execution queue。已完成的实现历史应进入 dated reports，例如
> `code-review-2026-04-28.md` 与 `release-smoke-status-2026-04-28.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Source Control external new-file runtime smoke | P2 | `apps/cli` serve path, Web Source Control refresh, watcher callback/WS bridge | Creating an external markdown file under an active vault produces one stable pending row and no visible refresh/rate-limit loop in the browser runtime; if runtime setup is unavailable, record the blocker and keep the core debounce tests as the code baseline |

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
- P1/P2 Git mirror Web repair notice 已关闭：Command Palette 新增 `Git: Repair Mirror` CLI-only notice，Source Control notice 独立解释 `repair_action[...]`、blocker 修复与 `deve_cli git export --repo <repo> --retry-out-of-sync` 重试路径；Web 仍不直接执行 Git repair，详见 `git-mirror-web-repair-notice-status-2026-04-29.md`。
- P1/P2 Git mirror CLI repair guidance 已关闭：`GitMirrorRepairAction` 为旧 record 补齐 subject fallback，CLI record 明细新增 `repair_guidance[...]`，覆盖所有 failure stage 的 manual-only next step 与 retry command，详见 `git-mirror-cli-repair-guidance-status-2026-04-29.md`。
- P1/P2 Git mirror repair UI boundary split 已关闭：features / acceptance / plan 已明确 future clickable repair UI 必须先只读 review、manual confirmation、fail-closed gates，且禁止 Command Palette 或后台自动 Git writer，详见 `git-mirror-repair-ui-boundary-status-2026-04-29.md`。
- P1/P2 Git mirror read-only repair review scaffold 已关闭：Source Control repair notice 下方新增只读 review 卡片，展示 repair action/guidance/subject/next step/copyable retry command 与 `.notegit` authority note，不调用 clipboard API、不执行 Git，详见 `git-mirror-readonly-repair-review-status-2026-04-29.md`。
- P1/P2 Git mirror repair review data-source decision 已关闭：真实 record-level review 数据源固定为 `GET /api/sc/git-mirror/repair-review` 受保护 HTTP 只读 endpoint，读取 server-side side table 与 core repair-action schema，不运行 Git、不写 `.git/.notegit`、不解析 CLI 输出，详见 `git-mirror-repair-review-data-source-2026-04-29.md`。
- P1/P2 Git mirror repair review Web data consumption 已关闭：Source Control repair notice 会消费只读 endpoint 并优先展示 record-level action/subject/next step/retry command；失败或无 record 回退 CLI-only 静态 review，详见 `git-mirror-repair-review-web-consumption-2026-04-29.md`。
- P2 Git mirror repair review UI multi-record/error polish 已关闭：repair review copy 拆分为独立模块，UI 支持多条 out-of-sync record、loading、load failed 与 empty fallback 状态，详见 `git-mirror-repair-review-ui-polish-2026-04-29.md`。
- P2 Git mirror executable repair UI decision 已关闭：当前批次明确不进入 Web 可执行 repair UI，不新增 Web Git writer；Web 保持只读 review / CLI-only notice，Git 写操作继续由显式 CLI surface 承担，详见 `git-mirror-executable-repair-ui-decision-2026-04-29.md`。
- P1/P2 Rendering current/future split 已关闭：`03_rendering` plan/features 已区分当前 editor adapter、lightweight Markdown renderer、大文档批量调度基础设施与 future preview/virtual-render/settings；`render_markdown` 补充 HTML allowlist、secure link 与 unsupported syntax 测试。
- P1/P2 Rendering current/future boundary guard 已关闭：新增 `scripts/check-rendering-baseline.sh` 守住 lightweight renderer 不是 hybrid engine、full preview/virtual rendering/settings GUI 仍属 future、BUILD Apply 仍走受控 edit gate，详见 `rendering-current-boundary-baseline-2026-04-30.md`。
- P2 Git mirror future/partial wording audit 已关闭：后台 Git writer、Web 后端直接 Git import/push/repair 与 executable Web repair UI 仍保持 future/deferred；`check-source-control-baseline.sh` 已守住 Web read-only / CLI-only 边界，详见 `git-mirror-future-boundary-audit-2026-04-30.md`。
- P1 Search/settings current-boundary audit 已关闭：Search 当前固定为 `search` feature + non-low-spec 下的 repo-scoped baseline scan，Tantivy 常驻索引仍属 future；server route 补齐 stale search scope 回归，Settings 当前保持 `config.toml` + `deve config print/set`，server-backed Settings API/统一 GUI 持久化仍属 future，详见 `search-settings-boundary-audit-2026-04-29.md`。
- P1 Native AI Chat minimum boundary audit 已关闭：同步 `PluginResponse` completion、backend 产品名与 runtime plugin id 转换层、bounded multi-turn history、provider `tool_calls` 先 fail-closed 后不发送成功 finish、Trusted CLI 文案与 fail-closed 注释均已收口，详见 `native-ai-chat-boundary-audit-2026-04-30.md`。
- P0 AI effective config boundary 已关闭：`ai.mode` / `ai.native_enabled` 现在驱动 server provider/RPC、Trusted CLI policy、capabilities endpoint 与 Web fallback/disabled UI，详见 `ai-effective-config-boundary-status-2026-04-30.md`。
- P2 Browser UI prefs storage consolidation 已关闭：layout width、Outline visibility、locale preference、shortcut overrides 均已统一走 `storage/prefs.rs` fallback 层；除 prefs 实现与能力探测外，功能模块不再直接调用 browser storage，详见 `browser-ui-prefs-boundary-status-2026-04-30.md`。
- P2 Post-plan/code drift rescan 已关闭：修正 Source Control baseline 脚本文案漂移、收紧 `deve.ui.last_scope` 为 repo name alias-only、同步 `deve init` config 模板、硬化 Trusted CLI executable/exit-status fail-closed，并排出下一轮 P0/P1，详见 `post-p2-plan-code-drift-rescan-2026-04-30.md`。
- P3-10 Desktop/Mobile native adapter core contract 已关闭：`08_ui_design_02_desktop` 与 `08_ui_design_03_mobile` 已明确 minimal adapter contract，`deve_core::native_adapter` 已落地平台无关状态/事件/endpoint/session/readiness 合同与定向测试；Tauri desktop/mobile shell、embedded service launcher 与 Web bootstrap 消费仍属后续实现。
- P3-10 Web native bootstrap 消费已关闭：Web connection manager 可读取 `window.__DEVE_NATIVE_BOOTSTRAP`，复用 core native endpoint/session 校验，有效时只使用注入 endpoint，失效时 fail-closed 且不回退端口推断；浏览器默认路径保持不变，详见 `native-web-bootstrap-status-2026-04-29.md`。
- P3-10 Server native-safe launch surface 已关闭：新增 `ServerLaunchOptions` 与 hidden `serve --native-loopback` 路径，native 模式只绑定 `127.0.0.1`、占用端口 fail-closed、不进入 proxy fallback，`/api/node/role` 暴露 nullable `native_service` endpoint/session surface；普通 release/Docker `0.0.0.0` 行为保持不变，详见 `native-server-launch-status-2026-04-29.md`。
- P3-10 Desktop native shell skeleton 已关闭：新增 `apps/desktop` 无 Tauri 依赖骨架，固定受控 endpoint、session 绑定、Web bootstrap 注入、service offline 与 session invalid recovery 状态机；真实 Tauri packaging/菜单/托盘/安装包仍为 future，详见 `desktop-native-shell-status-2026-04-29.md`。
- P3-10 Mobile native shell skeleton 已关闭：新增 `apps/mobile` 无 Tauri Mobile 依赖骨架，固定受控 endpoint、session 绑定、Web bootstrap 注入、background/suspended/resumed/foreground reprobe、service offline 与 session invalid recovery 状态机；移动生命周期事件只作为 reprobe hint，不授予写权限，详见 `mobile-native-shell-status-2026-04-29.md`。
- P3-10 Native runtime readiness UI recovery polish 已关闭：Web 端新增 native bootstrap invalid/session pending/service offline/foreground reprobe 结构化状态，header/bottom bar/mobile footer/overlay/Source Control gate 已显示明确恢复语义；desktop/mobile skeleton 可输出不含 secret/reason 的 recovery bootstrap，详见 `native-web-recovery-status-2026-04-29.md`。
- P3-10 Native packaging dependency gate 已关闭：`apps/desktop` 与 `apps/mobile` 声明 `native-packaging` no-op future gate，默认构建保持 no-Tauri skeleton，`check-native-track-boundary.sh` 会阻止 packaging dependency/import 泄漏到 workspace root、core、cli、web 或未开启门禁的 native crates，详见 `native-packaging-dependency-gate-2026-04-29.md`。
- P3-10 Desktop packaging scaffold plan split 已关闭：`apps/desktop` 在 `native-packaging` feature 后新增 packaging scaffold，声明 planned `tauri`/`tauri-build` dependency batch、window/menu/tray/installer/auto-update acceptance 与 forbidden authorities；实际 Tauri dependency/import 仍未引入，详见 `desktop-packaging-scaffold-status-2026-04-29.md`。
- P3-10 Mobile packaging scaffold plan split 已关闭：`apps/mobile` 在 `native-packaging` feature 后新增 packaging scaffold，声明 planned `tauri`/`tauri-build` dependency batch、WebView/permission/share/deeplink/file-picker/push/store package acceptance、lifecycle reprobe invariant 与 forbidden authorities；实际 Tauri Mobile dependency/import 仍未引入，详见 `mobile-packaging-scaffold-status-2026-04-29.md`。
- P2 Mobile touch feedback consistency 已关闭：Sidebar、Outline、Search Result 现在共用 `interactive_item_state_class`，`selected/hover/active/disabled` 语义一致；旧 gap-web 中的 partial 记录已被当前代码和 `mobile-touch-feedback-status-2026-04-29.md` 取代。
- P3-10 Native service supervisor contract 已关闭：`deve_core::native_adapter::NativeServiceSupervisor` 固定 Starting/EndpointHealthy/SessionHandoffReady/Restarting/Offline 状态、health probe、retry budget 与 session handoff fail-closed；desktop/mobile shell 与 native loopback launch surface 已接入，详见 `native-service-supervisor-status-2026-04-29.md`。
- P3-10 Native process adapter decision 已关闭：真实 child-process runtime 当前不进入默认 no-Tauri skeleton；`CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` 固定为 `DeferredUntilPackagingGate`，desktop/mobile 默认构建不 spawn、不持有、不重启后端进程且不写 core authority，详见 `native-process-adapter-decision-2026-04-29.md`。
- P3-10 Native packaging dependency gate decision 已关闭：真实 `tauri` / `tauri-build` dependency 当前不进入 workspace；`CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY` 固定为 `DeferredUntilRuntimeBatch`，默认构建保持 no-Tauri，packaging scaffold 继续仅作为 planned future input，详见 `native-packaging-dependency-gate-decision-2026-04-29.md`。
- P3-10 Native plan post-gate wording split 已关闭：desktop/mobile plan 已把当前 no-Tauri skeleton/no-process gate 与 post-gate Tauri/embedded-service/offline-first target 分开，详见 `native-plan-post-gate-wording-split-2026-04-30.md`。
- P3-10 Native packaging gate recheck 已关闭：`check-native-track-boundary.sh` 现在同时守住 no-Tauri dependency/import、no process runtime leak、desktop/mobile shell tests 与 core policy；真实 packaging/process adapter 仍是 post-gate future，详见 `native-packaging-gate-recheck-2026-04-30.md`。
- P3-13 Graph blocked/degraded acceptance polish 已关闭：Graph summary panel 已显式区分 local-only、blocked、degraded、empty 与 error 状态，degraded projection 继续要求显式 `--allow-degraded-projection`，详见 `graph-blocked-degraded-acceptance-polish-2026-04-30.md`。
- P3-13 Graph structured degraded error 已关闭：HTTP graph degraded projection 现在返回 `GRAPH_DEGRADED_PROJECTION_REQUIRED`，Web 不再解析 CLI/user-facing 文案，详见 `graph-structured-degraded-error-2026-04-30.md`。
- P1 AgentBridge env alias plan sync 已关闭：`DEVE_AI_AGENT_BRIDGE_ENABLED` / `DEVE_AI_AGENT_BRIDGE_TRUSTED` 已在 AI plan 与 agent_bridge 历史设计注中显式绑定为 Trusted CLI policy 兼容输入，详见 `agent-bridge-env-alias-plan-sync-2026-04-30.md`。
- P2 AI Settings UI acceptance depth 已关闭：Settings AI backend button 状态已提取为纯 policy 并补代码级测试；class/disabled/title/click gate 同源于 `AiBackendButtonState`，可用 backend 不再显示 disabled fallback title，不可用 backend 才显示原因，详见 `ai-settings-ui-acceptance-depth-2026-04-30.md`。
- P2 Settings Sync UI acceptance depth 已关闭：Settings section button policy 已拆到纯 helper；Sync Mode Auto/Manual 视觉状态现在显式跟随 `sync_mode` signal，并由代码级测试覆盖，详见 `settings-sync-ui-acceptance-depth-2026-04-30.md`。
- P2 Settings Language UI acceptance depth 已关闭：Settings 语言按钮视觉状态现在由 `language_button_state(Locale)` 统一派生，并由代码级测试覆盖；locale 持久化与 signal 更新回调不变，详见 `settings-language-ui-acceptance-depth-2026-04-30.md`。
- P2 Settings Reserved UI acceptance depth 已关闭：Hybrid Editing 预留项现在由 `reserved_setting_state(Locale)` 提供 disabled marker、`aria-disabled`、title 与可见原因；SET-006 与 operation flows 已绑定该验收口径，旧 `Phase 6` 文案已替换为 current-release-unavailable 边界文案，详见 `settings-reserved-ui-acceptance-depth-2026-04-30.md`。
- Architecture registry operation ID sync 已关闭：overview lisp 已同步 i18n locale fallback、Native AI Chat mode/apply/tool-rejection 与 release Web WASM quality gate operation IDs，`check-architecture-registry.sh` 恢复通过，详见 `architecture-registry-operation-id-sync-2026-04-30.md`。
- P2 Mobile AI Chat keyboard regression 已关闭：`MobileChatSheet` 展开态在 `keyboard_offset > 0` 时保持可见并设置 bottom offset；折叠 chip 在键盘态隐藏，drawer/diff 层级继续优先，详见 `mobile-ai-chat-keyboard-regression-status-2026-04-30.md`。
- P2 Mobile AI Chat viewport smoke 已关闭：Chrome MCP 375x812 验证展开、输入聚焦、44x44 发送按钮、关闭返回与 drawer 隐藏 chat；同时修复 Web WASM 引用后端-only Git bridge DTO 的 build break，详见 `mobile-ai-chat-viewport-smoke-2026-04-30.md`。
- P2 Mobile Diff fixture viewport smoke 已关闭：Chrome MCP 375x812 验证 `.diff-view-mobile`、隐藏 AI Chat/移动辅助键盘栏、close/edit 视口内可用与关闭返回 editor；移动端 Diff header 改为两行布局，详见 `mobile-diff-fixture-viewport-smoke-2026-04-30.md`。
- P1/P2 Watcher external new-file debounce 已关闭：pending upsert 的语义变更信号现在传回 handler，重复外部新增/删除事件不再重复发 `FsChangeDetected` 刷新消息；`deve_core` 全包测试通过，详见 `watcher-external-new-file-debounce-status-2026-04-30.md`。
- P3-10 Desktop runtime readiness / foreground reprobe 已关闭：desktop shell snapshot 现包含 `NativeRuntimeReadiness`，`RuntimeReady` 要求 endpoint/auth/node-role/repo-handshake/writer-ready/current-scope 全部满足，`Foreground` / `Resumed` 会进入 `ForegroundReprobe` 且 stale `scope_nonce` 不恢复写态，详见 `desktop-runtime-readiness-status-2026-04-29.md`。
- P3-10 Native shell parity review 已关闭：mobile foreground/resume reprobe 现在也清空 `node_role_readable`，desktop/mobile/Web 对 native readiness、`foreground_reprobe` recovery bootstrap 与 write gate 的当前 no-Tauri contract 已对齐；native track 进入干净停靠点，详见 `native-shell-parity-review-2026-04-29.md`。
- P3-13 Graph visualization read-only CLI projection surface 已关闭：`deve_core::graph` 保持 authority-free projection helper，`deve graph` 只读导出 repo-scoped `GraphProjection` JSON，默认 fail-closed 于损坏 Structure Facts authority；Web Canvas/d3-force/Pixi renderer 仍属 future implementation。
- P1/P2 Post-Git-mirror priority reselection 已关闭：P0 没有重新打开的 blocker；native packaging gate 仍按计划关闭，因此下一实际实现批次转入 P3-13 graph 数据面。
- P3-13 Graph HTTP projection surface 已关闭：新增 CLI/HTTP 共享只读 adapter 与受保护 `GET /api/repo/graph` query，默认 fail-closed 于损坏 Structure Facts authority；Web graph renderer 仍属后续，详见 `graph-http-projection-status-2026-04-29.md`。
- P3-13 Graph Web projection panel scaffold 已关闭：Source Control Graph 区域新增只读 summary panel，读取 `/api/repo/graph` 并展示 nodes/edges/unresolved counts 与 loading/failed/empty/local-only fallback；不引入 d3/Pixi、不写 authority，详见 `graph-web-projection-panel-status-2026-04-29.md`。
- P3-13 Graph Web renderer gate decision 已关闭：当前批次不打开 Canvas/d3-force/Pixi renderer gate，不新增 Graph renderer dependency；Web 继续只保留 summary panel 和 HTTP projection 数据面，详见 `graph-renderer-gate-decision-2026-04-29.md`。
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
