# Deve-Note 开发计划表 (全功能详细版)

**当前状态**: 准备进入 Phase 1.5 - 网络与同步实现。
**校验说明**: 本表作为项目最终验收的**唯一标准 (Master Checklist)**，包含从底层内核到 UI 交互的所有功能点。

## Phase 0: 钢铁核心 (Iron Core) - [SCAFFOLDED]
**目标**: 验证双向同步闭环 (Ledger <-> Vault) 与核心数据结构。
- [x] **项目初始化 (Project Init)**:
    - [x] Rust Workspace 结构 (`core`, `server`, `desktop`, `web`).
    - [x] 基础依赖引入 (Loro, Redb, notify, tokio).
- [ ] **存储层 A (Ledger - Redb)**:
    - [ ] 实现 `OpSeq` 键值存储设计 (Zero-copy).
    - [ ] 集成 Loro CRDT (Text/Map/List) 并封装为 `Doc` 对象.
- [ ] **存储层 B (Vault - FS)**:
    - [ ] 实现 `notify` 文件监听器 (Watcher).
    - [ ] 实现文件变更 **防抖 (Debounce)** 逻辑 (>500ms).
- [ ] **和解循环 (Reconciliation Loop)**:
    - [ ] `Vault -> Ledger`: 实现 `Dissimilar` 差异计算 -> 生成 Ops.
    - [ ] `Ledger -> Vault`: 实现 Ops 应用 -> 写入文件.

## Phase 1: P2P 网络编织 (Network & P2P Fabric)
**目标**: 实现多端同步、三库隔离 (Trinity Isolation) 与身份认证。
- [ ] **仓库管理器 (Repository Manager)**:
    - [ ] **三库隔离**: 实现 `Vault` (用户区) / `LocalDB` (本机库) / `ShadowDB` (影子库) 目录结构.
    - [ ] **身份标识**: 强制执行 `RepoUUID` 和 `PeerUUID` 逻辑.
- [ ] **握手与认证 (Handshake & Auth)**:
    - [ ] 实现基于 `AUTH_SECRET` 的 JWT Token 生成与校验.
    - [ ] 实现 WebSocket 握手协议 (版本号/PeerName/公钥交换).
    - [ ] 实现 **Argon2** 管理员密码验证 (Admin Access).
- [ ] **同步协议 (Gossip Protocol)**:
    - [ ] **向量时钟 (Vector Clock)**: 实现逻辑时钟结构.
    - [ ] **Gossip 逻辑**: 基于 Peer VC 计算缺失 Ops (Missing Ops).
    - [ ] **影子写入 (Shadow Write)**: 将接收到的 Ops 写入对应的 `ShadowDB` (Store C).
- [ ] **API 接口**:
    - [ ] 实现前端专用的 WebSocket RPC (获取状态/Ops流/图谱数据).

## Phase 2: UI 基础 (UI Foundation - Leptos)
**目标**: 构建高性能、现代化外观的编辑器外壳 (Cursor-Style)。
- [ ] **应用外壳 (App Shell)**:
    - [ ] **5列网格布局**: Sidebar | Diff(Old) | Diff(New) | Outline | Chat.
    - [ ] **拖拽调整**: 实现各列宽度的拖拽调整 (Resizer).
    - [ ] **状态栏**: 独立的 Flex 布局 (底部).
    - [ ] **标题栏**: 极简自定义 Header (无搜索框).
- [ ] **编辑器内核 (Editor Core - CM6)**:
    - [ ] 集成 CodeMirror 6 (WASM/JS Adapter).
    - [ ] **Markdown**: 支持 GFM 语法高亮 (Bold/Italic/List).
    - [ ] **Math**: 实现 KaTeX **Inline** (`$E=mc^2$`) 与 **Block** 渲染.
    - [ ] **行号**: 为后续同步滚动做准备.
- [ ] **辅助面板**:
    - [ ] **大纲栏 (Outline)**: 固定宽度 (260px)，实现右上角悬浮开关按钮 (Overlay Toggle).
    - [ ] **主侧边栏**: 顶部水平 Activity Tabs (Explorer, Search, Git).

## Phase 3: UI 高级交互 (Advanced Operations)
**目标**: 复杂的模态框、Git 风格操作与视觉反馈。
- [ ] **统一搜索模态框 (Unified Search Modal)**:
    - [ ] **UI 组件**: 统一的模态框 (Icon + Input + List + Footer).
    - [ ] **智能切换**: 同模式关闭 (Toggle Off)，异模式切换 (Switch Mode).
    - [ ] **焦点管理**: 取消时还原光标精确位置，确认时聚焦新内容.
    - [ ] **三大模式**:
        - [ ] Command Palette (`>`)
        - [ ] Quick Open (`无前缀`)
        - [ ] Branch Switcher (`@` 或 `用户指定`)
- [ ] **交互式状态栏**:
    - [ ] **左侧**: 远程 Peer 状态, 当前分支, 同步 Spinner.
    - [ ] **右侧**: 只读指示器 (Spectator), 光标位置 (Ln/Col).
    - [ ] **Slider**: 历史版本回溯滑块 (History Slider).
- [ ] **差异视图系统 (Diff View UI)**:
    - [ ] **三栏逻辑**: Old (Shadow) | New (Local) | Merge Result (Preview).
    - [ ] **同步滚动**: 锁定左右编辑器滚动条，保持代码行对齐.
    - [ ] **只读锁定**: 确保 Shadow 区域不可编辑 (Read-Only).

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

## Phase 6: 发布与运维 (Release & DevOps)
**目标**: 全平台分发。
- [ ] **CI/CD 流水线**:
    - [ ]配置 `release.yml` (GitHub Actions).
    - [ ] 跨平台构建 (Win/Mac/Linux).
    - [ ] 签名与公证 (MacOS Notarization / Windows Sign).
- [ ] **Docker / OCI**:
    - [ ] 多架构镜像构建 (AMD64/ARM64).
    - [ ] 推送至 GHCR (`ghcr.io/Develata/...`).
    - [ ] 验证 Podman 兼容性.
- [ ] **最终验收 (Sanity Check)**:
    - [ ] 全流程测试 (Sync -> Conflict -> Merge).
    - [ ] 纯净安装测试 (Clean Install).