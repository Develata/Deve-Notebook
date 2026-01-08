# Deve-Note 开发计划表

**预计总时长**: 8-12 周
**开始日期**: 待定

## 阶段 0: 钢铁核心 (Headless Prototype)
**时长**: 第 1-2 周 (关键路径)
**目标**: 在没有任何 UI 的情况下，验证 "Ledger -> Vault" 和 "Vault -> Ledger" 的双向同步闭环。**必须**在开始 UI 工作前完成。

- [ ] **核心逻辑**: 搭建 Rust Workspace, 实现 `Ledger` 结构体 (Redb + CRDT/Loro)。
- [ ] **VFS 层**: 实现 `DocId` 分配与路径映射 (Path Mapping) 逻辑。
- [ ] **和解引擎 (Reconciliation Engine)**:
    - [ ] 实现 `notify` 文件监听器。
    - [ ] **关键**: 实现 Inode 追踪与防抖 (Debounce) 逻辑。
    - [ ] **关键**: 实现 "Diff-to-Ops" 逻辑 (Dissimilar)。
- [ ] **CLI 工具**: 构建 `deve-note init`, `deve-note watch`, `deve-note append` 命令。
- [ ] **验证及其验收**:
    - [ ] 测试: `VS Code` 修改文件 -> `Ledger` 正确记录 Op。
    - [ ] 测试: `Ledger` 接收 Op -> `Vault` 文件更新 (且不破坏用户光标/不造成冲突)。
    - [ ] 测试: 在 OS 中重命名文件 -> `Ledger` 保持 `DocId` 不变 (不误判为删除+新建)。
    - [ ] **增强**: 实现 UUID/Hash Fallback 机制，确保 Inode 失效时仍能找回 DocId。
    - [ ] **安全**: 验证 12-Factor Auth，确保无 `init` UI，仅通过 Env 启动。

## 阶段 1: 最小可行驾驶舱 (MVC)
**时长**: 第 3-5 周
**目标**: 一个可用的本地 Markdown 编辑器，具备基本导航功能。

- [ ] **前端基础设施**:
    - [ ] 初始化 Leptos v0.7 + Tailwind CSS 项目。
    - [ ] 集成 `leptos_i18n` 实现多语言支持 (En/Zh)。
    - [ ] 实现 "Resizable Slots" (可缩放插槽) 布局引擎。
- [ ] **编辑器集成**:
    - [ ] 将 CodeMirror 6 封装为 Leptos 组件。
    - [ ] 将 CodeMirror 变更绑定到 Loro CRDT (Wasm)。
    - [ ] 实现基础 Markdown 样式渲染与数学公式 (KaTeX/MathJax) 支持。
- [ ] **服务端通信**:
    - [ ] 实现具备断线重连逻辑的 WebSocket 客户端。
    - [ ] 实现 "文件树" 侧边栏 (虚拟列表)。

## 阶段 2: 鲁棒性强化 ("512MB 挑战")
**时长**: 第 6-7 周
**目标**: 针对低配服务器进行优化，并确保数据安全。

- [ ] **性能优化**:
    - [ ] 实现 `DEVE_PROFILE` 配置加载逻辑。
    - [ ] 为 `low-spec` 模式实现仅 CSR (客户端渲染) 模式。
    - [ ] 内存分析与优化 (设置缓存上限)。
- [ ] **数据安全**:
    - [ ] 实现定期快照 (Snapshotting) 与裁剪 (Pruning) 策略 (低配模式保留 10 个快照)。
    - [ ] 实现 `deve-note export-ledger` (导出 JSONL) 灾备功能。
    - [ ] 压力测试: 测试网络断开和服务器强制杀进程场景下的数据完整性。

## 阶段 3: 插件与高级能力
**时长**: 第 8-10 周
**目标**: 启用扩展性与丰富功能 (搜索, 图谱)。

- [ ] **插件系统**:
    - [ ] 集成 `Rhai` 或 `Extism` 运行时。
    - [ ] 实现 `Capability` (能力清单) 解析器，并强制执行 "Manifest Enforcement" (未声明即 Panic)。
- [ ] **宿主函数 (Host Functions)**:
    - [ ] 暴露 `read_note`, `write_note` (受控 API)。
    - [ ] 实现 `Plugin RPC Bridge` (前端 <-> 后端通信桥梁)。
- [ ] **高级特性** (仅 Standard Profile):
    - [ ] 集成 `Tantivy` 实现全文检索。
    - [ ] 实现后台图谱分析与可视化功能。

## 阶段 4: 打磨与发布
**时长**: 第 11-12 周
**目标**: 准备公开发布。

- [ ] **CI/CD**:
    - [ ] 配置 GitHub Actions 进行交叉编译 (生成 Windows/Linux/macOS 二进制)。
    - [ ] 构建 Docker 镜像 (支持 Standard & Low-Spec profiles)。
- [ ] **移动端**:
    - [ ] 为移动端外壳构建简化的 "阅读模式 (Reader Mode)" UI。
- [ ] **文档**:
    - [ ] 编写 `README.md` 和用户指南 (User Guide)。
    - [ ] 编写插件 API 文档。
- [ ] **发布**: 发布 v0.1.0 Beta 版本。