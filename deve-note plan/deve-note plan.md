# 📑 Deve-Note Plan - Master Index

**版本**: 0.0.1
**核心理念**: Git-Flow P2P Architecture, Trinity Isolation, Remote Dashboard.

本文档已模块化，请参阅以下子文档获取详细规划：

## 📚 目录 (Table of Contents)

### Phase 1: Context & Definitions
1.  **[Terminology & Definitions](./01_terminology.md)**: 核心术语 (Ledger, Snapshot, Peer) 与规范性用语.
2.  **[Project Positioning](./02_positioning.md)**: 项目定位、核心边界 (Core MUST).

### Phase 2: Architecture & Storage
3.  **[Rendering Engine](./03_rendering.md)**: 编辑器内核、LaTeX 公式与解析优先级.
4.  **[Data Storage](./04_storage.md)**: 三库隔离 (Trinity Isolation)、数据恢复与灾备.
5.  **[Network Architecture](./05_network.md)**: P2P 拓扑、Web 面板约束与同步协议.

### Phase 3: Version Control & Logic
6.  **[Repository & Branching](./06_repository.md)**: 仓库管理、严格分支策略与 Spectator Mode.
7.  **[Diff Logic](./07_diff_logic.md)**: 内部和解逻辑 (Reconciliation) 与合并流程.

### Phase 4: User Interface
8.  **[UI Design](./08_ui_design.md)**: Desktop/Web/Mobile 界面设计、组件组织与视觉规范.
9.  **[Authentication](./09_auth.md)**: 登录认证与 12-Factor Auth 策略.
10. **[Internationalization](./10_i18n.md)**: 多语言支持策略 (i18n).

### Phase 5: Extensions & Operations
11. **[Plugins & Runtime](./11_plugins.md)**: 双引擎运行时 (Wasm/Podman)、AI 隐私与 RPC 协议.
12. **[Commands Summary](./12_commands.md)**: CLI 与 Command Palette 指令汇总.
13. **[Settings Summary](./13_settings.md)**: 环境变量与配置文件汇总.
14. **[Technology Stack](./14_tech_stack.md)**: 技术选型、Markdown 兼容性与性能预算.
15. **[Release Strategy](./15_release.md)**: 开源发布、Docker 镜像与版本管理.
