# Deve-Note 开发进度与里程碑 (Development Schedule)

**当前状态**: 准备进入 Phase 1.5 - 网络与同步实现。
**校验说明**: 本表作为项目最终验收的**唯一标准 (Master Checklist)**，包含从底层内核到 UI 交互的所有功能点。
> **当前阶段**: Phase 2 收尾 / Phase 3 启动
## Phase 0: 钢铁核心 (Iron Core) - [SCAFFOLDED]

---

## 🗓️ Phase 1: Core Foundation (核心基石)
**目标**: 完成 "Trinity Isolation" 架构，实现存储、监听与基础状态机。
**周期**: 2025.11.20 - 2026.01.15 (已基本完成)
    - [x] 集成 Loro CRDT (Text/Map/List) 并封装为 `Doc` 对象.
- [x] **Project Scaffolding**
    - [x] Workspace setup (Core, CLI, Web).
    - [x] 实现文件变更 **防抖 (Debounce)** 逻辑 (>500ms).
    - [x] `error.rs`: 统一错误处理 (`AppError`).
    - [x] `Vault -> Ledger`: 实现 `Dissimilar` 差异计算 -> 生成 Ops.
    - [x] `Ledger -> Vault`: 实现 Ops 应用 -> 写入文件.
    - [x] `watcher.rs`: 基于 `notify` 的文件系统监听与防抖.
- [x] **Store B: Local Ledger (Redb)**
    - [x] `ledger/schema.rs`: 定义 Redb 表结构 (`DOCID_TO_PATH`, `LEDGER_OPS`).
    - [x] `ledger/ops.rs`: 原子化 Op 读写.
    - [x] **三库隔离**: 实现 `Vault` (用户区) / `LocalDB` (本机库) / `ShadowDB` (影子库) 目录结构.
- [x] **State Machine & Protocol**
- [x] **握手与认证 (Handshake & Auth)**:
    - [/] 实现基于 `AUTH_SECRET` 的 JWT Token 生成与校验.
    - [x] `state.rs`: CRDT 基础 (Myers Diff, DAG Reconstruct).
    - [ ] 实现 **Argon2** 管理员密码验证 (Admin Access).
- [x] **同步协议 (Gossip Protocol)**:
    - [x] **向量时钟 (Vector Clock)**: 实现逻辑时钟结构.
    - [x] **Gossip 逻辑**: 基于 Peer VC 计算缺失 Ops (Missing Ops).
    - [x] **影子写入 (Shadow Write)**: 将接收到的 Ops 写入对应的 `ShadowDB` (Store C).
- [x] **API 接口**:
    - [x] 实现前端专用的 WebSocket RPC (获取状态/Ops流/图谱数据).
## 🗓️ Phase 2: Basic Interaction (基础交互)
**目标**: 实现端到端编辑流，Web 端可读写本地文件。
**周期**: 2026.01.15 - 2026.02.01 (收尾中)

- [x] **CLI / Backend Service**
    - [x] `server/ws.rs`: WebSocket 网关与连接管理.
    - [x] `server/handlers/`: 消息路由 (Document, Sync).
    - [x] **标题栏**: 极简自定义 Header (无搜索框).
    - [x] `commands/scan.rs`: 启动时全量索引扫描.
- [x] **Web Frontend (Leptos)**
    - [x] **Markdown**: 支持 GFM 语法高亮 (Bold/Italic/List).
    - [x] **Math**: 实现 KaTeX **Inline** (`$E=mc^2$`) 与 **Block** 渲染.
    - [x] **行号**: 为后续同步滚动做准备.
    - [x] **File Tree**: 虚拟文件树渲染.
    - [x] **大纲栏 (Outline)**: 固定宽度 (260px)，实现右上角悬浮开关按钮 (Overlay Toggle).
    - [x] **主侧边栏**: 顶部水平 Activity Tabs (Explorer, Search, Git).
    - [ ] 📅 **Reconnection**: 完善断线重连机制与离线队列 (Offline Ops).

---

    - [x] **UI 组件**: 统一的模态框 (Icon + Input + List + Footer).
    - [x] **智能切换**: 同模式关闭 (Toggle Off)，异模式切换 (Switch Mode).
    - [x] **焦点管理**: 取消时还原光标精确位置，确认时聚焦新内容.
    - [x] **三大模式**:
        - [x] Command Palette (`>`)
        - [x] Quick Open (`无前缀`)
        - [x] Branch Switcher (`@` 或 `用户指定`)
- [x] **交互式状态栏**:
    - [x] **左侧**: 远程 Peer 状态, 当前分支, 同步 Spinner.
    - [x] **右侧**: 只读指示器 (Spectator), 光标位置 (Ln/Col).
    - [x] **Slider**: 历史版本回溯滑块 (History Slider).
**周期**: 2026.02.01 - 2026.02.20
    - [x] **三栏逻辑**: Old (Shadow) | New (Local) | Merge Result (Preview).
    - [x] **同步滚动**: 锁定左右编辑器滚动条，保持代码行对齐.
    - [x] **只读锁定**: 确保 Shadow 区域不可编辑 (Read-Only).

## Phase 4: 插件与扩展系统 (Extensions)
**目标**: 用户可扩展性与 AI 能力集成。
- [ ] **引擎 A (应用级运行时)**:
    - [ ] 集成 **Rhai** 脚本引擎 (Simple Hooks).
    - [ ] 集成 **Extism (WASM)** 插件加载器.
    - [ ] 实现 Capability Manifest 权限校验 UI.
- [ ] **引擎 B (计算运行时)**:
    - [ ] **Podman 集成**: 检测与调用宿主机 Podman (Rootless).
    - [ ] **代码执行**: WebSocket -> Server -> Podman -> Output.
- [ ] **AI 集成**:
    - [ ] **Chat Slot UI**: 实现右侧 AI 聊天面板 UI.
    - [ ] **Provider ABI**: 定义标准 AI 插件接口 (WASM Trait).
    - [ ] 📅 Hook System: 定义生命周期钩子 (on_save, on_load).
## Phase 5: 优化与安全 (Polish & Security)
**目标**: 生产环境就绪。
- [ ] **国际化 (I18n)**:
    - [ ] 集成 `leptos_i18n`.
    - [ ] 完成全量翻译文件 (`en-US`, `zh-CN`).
    - [ ] 后端错误码 (Error Codes) 映射 UI 提示.
- [ ] **安全加固**:
    - [ ] **路径遍历检查**: 校验所有 VFS 路径操作.
    - [ ] **Rootless 检查**: 确保 Podman 非 Root 运行.
    - [ ] **限流 (Rate Limiting)**: Axum 中间件配置.
- [ ] **移动端适配**:
    - [ ] iOS 触摸支持 (大号拖拽手柄).
    - [ ] SQLite 存储适配 (如需).
**周期**: 2026.02.21 - 2026.03.10

- [ ] **Distribution**
    - [ ] Dockerfile 编写.
    - [ ] CI/CD Pipeline (GitHub Actions).
- [ ] **Documentation**
    - [ ] User Manual (Usage).
    - [ ] Developer Guide (Plugin API).

---

## 🛑 Current Blockers (当前阻碍)

1.  **Merge Logic Missing**: `ledger/merge.rs` 目前为空或占位符。
    * **后果**: 无法处理多端并发修改，无法实现真正的 "Local-First" 数据合并。
    * **对策**: 下一步立即着手实现，采用拆分文件策略 (`algo.rs` + `mod.rs`)。

2.  **Plugin Runtime**: 仅有骨架，尚未打通。
    * **对策**: Phase 3 重点攻克。

## ⚠️ Development Constraints (开发约束)

为确保代码库的可维护性，所有后续提交必须遵守：
1.  **Size Limit**: 单文件行数目标 ~100 行，硬性上限 200 行。超过即拆分。
2.  **Docs**: 必须包含中文架构注释。
3.  **Isolation**: 严禁 Store A (Vault) 绕过 Store B (Ledger) 直接修改数据。