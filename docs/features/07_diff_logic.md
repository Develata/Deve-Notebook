# 07_diff_logic.md - Diff 与版本体验篇

本章描述工作区变更、stage、commit、diff、history 与 merge 的用户体验。

## 功能目标

- 用户应能看清当前有哪些工作区变化。
- 用户应能明确区分 working directory、staged、confirmed ledger dirty、committed。
- 用户应能明确区分外部投影文件夹修改与已经进入 ledger 的 Source Control 变更。
- 冲突与只读场景不能伪装成正常提交。

## Operation 示例

- Commit：`docs/features/operations/sc_commit.md`
- Stage / Unstage：`docs/features/operations/sc_stage_unstage.md`
- Discard File：`docs/features/operations/sc_discard_file.md`
- Discard Pending：`docs/features/operations/sc_discard_pending.md`
- Resolve Conflict：`docs/features/operations/sc_resolve_conflict.md`
- Merge Peer：`docs/features/operations/sc_merge_peer.md`
- Merge Runtime：`docs/features/operations/sc_merge_runtime.md`
- CommitAndPush：`docs/features/operations/sc_commit_and_push.md`
- Commit History / Commit Diff：`docs/features/operations/sc_history_commit_diff.md`
- External Changes：`docs/features/operations/external_changes.md`

## 功能项

### 1. External Changes / 外部修改

- 外部文件变化或本地投影工作区偏差应进入 `External Changes / 外部修改` 同级板块。
- 用户可以看到文件级变化状态。
- 用户可以 Open Diff、Stage / Unstage、Discard External Change，并用 `Apply to Ledger` / `确认外部修改` 写入 ledger facts。
- External Changes 不显示 history / graph，不创建 Source Control commit anchor。
- `Apply to Ledger` 不是普通 `Commit`，UI 不应只写 `Commit`。

### 2. External Stage / Unstage

- 用户可以把外部候选变化移入或移出 External Changes staged 区域。
- staged 与 unstaged 必须可见区分。
- External Changes working row 右侧应提供 VS Code-like inline actions：打开 diff、Discard External Change、Stage；桌面通过 hover/focus 显示，移动端保持可见并满足触控尺寸。
- External Changes staged row 右侧应提供 Unstage；Confirmed Ledger row 只提供打开 diff，不提供逐文件 Stage / Discard / Revert。
- 变更状态字母 `M` / `A` / `D` / `R` 必须保留，但应与 row action tray 分区显示，避免挤压文件名或覆盖按钮。
- External staged / external unstaged / confirmed ledger resource group header 应是可键盘触发的折叠按钮，并通过 `aria-expanded`
  与 `aria-controls` 暴露状态和受控内容；受控内容应以稳定 `id` 常驻 DOM，折叠时使用 `hidden`
  隐藏；section action 按钮不应顺带触发展开/收起。

### 2.1 External / Confirmed Ledger Overlap

- 如果同一文档同时存在外部修改与 `Confirmed Ledger Changes`，External Changes 行应显示
  `与已确认账本更改重叠` / `Overlaps confirmed ledger changes`。
- 重叠行禁用普通 Stage 和 `Apply to Ledger`，只允许打开 diff 或丢弃外部修改。
- 系统不得自动用外部文件覆盖 ledger，也不得自动把 ledger dirty 覆盖到投影文件；任何未来“用外部文件覆盖 ledger”都必须是单独明确操作。

### 2.2 Confirmed Ledger Changes

- 程序内编辑或 CLI 受控写入 ack 后，变化已经进入 ledger，不应出现在工作区 pending 列表。
- 若这些变化尚未被最新 Source Control commit anchor 覆盖，Source Control 应展示 `Confirmed Ledger Changes`。
- 该分组不提供 Stage / Discard / Revert；首版只提供打开 diff，commit 一次性覆盖全部 confirmed ledger changes。
- UI 应在该分组中说明它们已进入 ledger，不能逐文件暂存或放弃，只能打开 diff 或随本组整体 commit anchor 覆盖。
- confirmed-only commit 成功后，该分组应清空并刷新 history。
- Source Control 只负责 ledger/version-anchor 状态、Commit、history、graph；不显示 External Changes 的 staged/unstaged working groups。

### 3. Diff / History / Graph

- 用户可以打开 diff 查看变更内容。
- 用户可以查看 commit history / graph。
- 这些视图必须与当前 repo scope 一致。
- Repository / history / graph 这类 secondary panel header 应是可键盘触发的折叠按钮，并通过
  `aria-expanded` 与 `aria-controls` 暴露状态和受控内容；受控内容应以稳定 `id` 常驻 DOM，折叠时
  使用 `hidden` 隐藏。
- remote / spectator scope 下，diff / history / graph 仍作为只读视图可用；
  stage、unstage、discard、commit、resolve conflict 等写操作必须被隐藏或禁用。
- Graph 数据面是只读 projection，不写 ledger、workspace、search index 或 source-control state。
- Web 只验收 repo-scoped nodes / edges / unresolved counts，以及 loading / failed / empty / local-only fallback。
- 本功能篇不承诺高性能 Web graph renderer、force simulation、Canvas layout、d3-force/Pixi renderer 或 graph interaction state。

### 4. Merge / Conflict

- 冲突必须以显式方式显示。
- 只读或 spectator 场景下不能假装支持 commit/merge 写入。

### 5. NoteGit / Git Main Mirror Repair UI Boundary

Web 只提供 `ngit:import`、`ngit:push` 与 `ngit:repair`
的 backend/runtime intent 或 read-only notice。可点击 Git main mirror repair UI 只有满足以下边界后才能进入验收：

- UI 第一阶段只能展示 `repair_action[...]` / `repair_guidance[...]` 的只读解释与 copyable CLI command，不得直接执行 Git。
- 只读 review 只能读取 server-side mirror status 与 repair-action schema，不运行 Git，不写 `.git` / `.notegit`，Web 不解析 CLI 输出。
- loading、load failed、empty record fallback 只影响展示，不授予 Web repair 写权限。
- Git import/push/repair 写操作只允许通过显式 backend/runtime surface 触发。
- 若后续进入可执行 UI，必须有明确 manual confirmation，且确认内容包含 repo、repair action code、subject、retry command 与 `.notegit` authority 提醒。
- 任何可执行 repair flow 都必须 fail-closed 于 remote/spectator scope、未绑定 repo、writer not ready、dirty Deve Source Control、dirty Git worktree、`.notegit` Git tracking leak 与 stale scope nonce。
- 后台自动 Git writer 不是该 UI 的一部分；`.git` main 仍只是 projection mirror，`.notegit` / ledger source-control state 仍是 authority。

### 6. NoteGit-only Git Main Mirror

- Source Control 固定使用 NoteGit/ngit authority，不再暴露 `source_control.git_bridge` / mirror/off 设置。
- NoteGit/ngit commit 成功后始终尝试排队 Git main mirror record；排队或执行失败只形成诊断，不回滚 ledger commit。
- Git main mirror 只要求 Markdown Projection Workspace 的终态与 NoteGit/ngit 终态一致，不要求历史轨迹逐条一致。
- ngit status/import/mirror/export/push 在执行 import apply、mirror/export 或 push 前必须复用本地 Projection Workspace identity gate；
  `.notegit` identity marker 或 Projection Locator 破损时不得写 pending/import、`.git` mirror 或发布 mirror HEAD。
- Web Command Palette 与 Source Control UI 不展示 legacy bridge mode；Source Control header 应写成 NoteGit/ngit authority-first 文案，避免把 Git main mirror 误读成 Git authority 切换。
- Source Control header 的 section visibility menu 只用于切换 view-local section 显示；trigger 应暴露
  menu 展开状态，菜单项应暴露 checked 状态，并在选择后自动关闭。
- Web `Commit & Push` 入口只展示 Git push unsupported/read-only notice；旧 WS `CommitAndPush` frame 不得等价为普通 commit，服务端必须返回结构化 blocker 且无 source-control 写副作用。
- 插件 host 的 `sc_commit` 与 plugin-host HTTP commit 必须走同一个 NoteGit/ngit commit path；代理模式必须展示 delegated/unknown/readonly 状态，而不能硬编码为 mirror mode。
- 后端 commit writer API 不接收 legacy bridge policy；新增 CLI、HTTP、WS 或插件提交路径时必须复用 NoteGit/ngit source-control writer gate。

### 7. WebDAV/S3 Remote Projection Transport

- Command Palette 应提供 `webdav:push`、`webdav:pull`、`s3:push`、`s3:pull`。
- 未接线的 provider/direction 必须显示 `provider_io_ready=false` 并 fail-closed，不能伪装成
  已经 push/pull 成功。已接线的 provider/direction 只有在 backend/core runtime 完成
  workspace identity gate 与 provider adapter 调用后，才能显示 `provider_io_ready=true`。
- 当前 backend/CLI 已接线 `webdav:push`、`webdav:pull` 与 AWS `s3://` `s3:push`/`s3:pull`；
  `s3+https://` custom endpoint 在显式 credential binding 完成前必须继续 fail-closed。
- Web Command Palette 只发送 provider/direction intent；backend 从当前 local repo 的 `repo_url`
  解析 WebDAV/S3 locator。未配置 transport URL 或 provider 与 URL scheme 不匹配时显示
  `provider_io_ready=false`，不得在前端收集凭证、列举远端文件或覆盖本地文件。
- push 只上传当前 Markdown Projection Workspace 文件集合，不上传 ledger、`.notegit/`、`.git/` 或 runtime state。
- pull 只覆盖 Markdown Projection Workspace 文件；随后由 watcher/scan 进入 External Changes。
- pull adapter 必须对远端文件数、单文件字节数与总下载字节数设置硬预算；超过预算时必须在写 Projection Workspace 前 fail-closed。
- pull 覆盖 Projection Workspace 必须避免半写入可见：目标 parent/path 安全检查、staging 与 rollback 属于 backend/core runtime 职责。
- pull 不直接写 ledger、不创建 commit anchor、不自动 Apply to Ledger，也不直接写 Git main mirror queue。
- Web 只发送 typed intent；provider IO、覆盖策略、locator gate、identity gate 与 External Changes 触发均属于 backend/core runtime。

### 7. HTTP Source Control Write Grant

- Browser 通过 WebSocket 完成 `SyncHello + RegisterWriter` 后，HTTP stage/discard/unstage/commit 才能使用同一 session 的短生命周期 write grant。
- 普通 HTTP mutation 的 `scope_nonce` 必须匹配 server-side active grant；任意非零 nonce 或 remote proxy 固定 nonce 不能绕过 writer gate。
- anonymous localhost 模式下的“同一 session”由 dev session cookie 区分；不能把所有 localhost 请求视为同一个 dev-wide writer grant identity。
- 如果请求同时携带有效 JWT 与 dev session cookie，HTTP/WS Source Control grant 必须共同绑定 JWT session，不能由 dev cookie 覆盖。
- WS 断开、repo/branch 切换、repo scope recovery、sync guard 或 Browser `SyncHello` failure 清理当前 runtime binding、session 失效或 writer 重新注册后，旧 grant 必须失效。
- remote proxy delegated API 是独立 authority path；它不能被 Web Source Control UI 或普通主进程 HTTP mutation 复用。
- `/api/delegated/sc/*` 还必须要求显式 delegated capability；普通已登录浏览器或 anonymous localhost dev session
  不能直接调用 delegated 写入口绕过 browser write grant。
- plugin-host remote delegated Source Control API 必须由显式 delegated proxy 类型注册；普通本地
  SourceControlApi 不能被误登记为 delegated mode 并绕过本地写门。
- proxy 模式的只读 repo/source-control 查询必须走 delegated read capability；不能依赖 browser JWT、anonymous
  localhost dev session，或把只读代理查询升级成 writer grant。
- CLI `deve sc stage/commit`、plugin-host HTTP mutation 与 Rhai `sc_commit`
  也必须复用本地 Projection Workspace identity gate；`.notegit` identity marker
  或 Projection Locator 破损时不得写 pending/staging/commit。

## 非目标

- 当前阶段不支持跨 repo 自动 merge。
- 当前阶段不允许 remote spectator 直接提交远端写入。
- 当前阶段不实现 Web 后端直接 Git import/push/repair，也不实现后台自动 Git mirror repair。

## Chrome MCP 验收实例

### DIFF-FEAT-01: Changes -> Stage -> Unstage

前置条件：

- 当前 repo 有至少一个工作区变化。

步骤：

1. 打开 External Changes。
2. 观察 `External Changes` 列表。
3. 执行 `Stage`。
4. 再执行 `Unstage`。

期望结果：

- 条目能在 `External Changes` 与 `Staged External Changes` 之间移动。
- 不出现点击无效或状态错位。

### DIFF-FEAT-01B: External Changes -> Apply to Ledger

前置条件：

- 当前 repo 可写。
- Projection Workspace 中存在外部修改。

步骤：

1. 打开 External Changes。
2. Stage 一个外部修改。
3. 点击 `Apply to Ledger` / `确认外部修改`。
4. 打开 Source Control。

期望结果：

- 外部修改写入 ledger facts。
- External Changes staged/unstaged 列表刷新。
- Source Control 显示对应 `Confirmed Ledger Changes`。
- history / graph 不因 `Apply to Ledger` 创建新 commit anchor。

### DIFF-FEAT-02: 打开 Diff 与 History

前置条件：

- 当前 repo 存在变更和历史提交。

步骤：

1. 点击某个 change 打开 diff。
2. 切到 history / graph。
3. 选择一条提交查看详情。

期望结果：

- diff 正常显示。
- history / graph 与当前 repo 一致。

### DIFF-FEAT-03: 只读分支写入边界

前置条件：

- 切换到 remote / spectator 分支。

步骤：

1. 打开 Source Control。
2. 尝试执行 stage、commit 或其它写操作。

期望结果：

- 页面明确显示只读或不可写。
- diff / history / graph 仍可按当前 scope 只读打开。
- 不会假装提交成功。

### DIFF-FEAT-04: Confirmed Ledger Changes

前置条件：

- 当前 repo 可写，且存在一个通过编辑器或 CLI 受控写入确认的 ledger change。
- 该 change 尚未被最新 Source Control commit anchor 覆盖。

步骤：

1. 打开 Source Control。
2. 观察 `Confirmed Ledger Changes` 列表。
3. 点击 confirmed row 打开 diff。
4. 输入 commit message 并执行 commit。

期望结果：

- confirmed row 不显示 Stage / Discard / Revert，只显示打开 diff 的只读动作。
- diff 基于 latest commit anchor 与当前 ledger head。
- commit 成功后 `Confirmed Ledger Changes` 清空，history 增加新 commit。

### DIFF-FEAT-05: External 与 Confirmed Ledger 重叠 fail-closed

前置条件：

- 同一文档已有 `Confirmed Ledger Changes`。
- Projection Workspace 中同一路径或同一 `DocId` 又出现外部修改。

步骤：

1. 打开 External Changes。
2. 观察重叠行。
3. 尝试普通 Stage。
4. 打开 diff。
5. 丢弃外部修改。

期望结果：

- 行显示 `与已确认账本更改重叠` / `Overlaps confirmed ledger changes`。
- Stage 与 Apply to Ledger 禁用。
- Open Diff 可用。
- Discard External Change 可用，并恢复投影文件到当前 ledger projection。
