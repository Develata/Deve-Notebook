## 验收用例索引

本目录内用例均为“可自动化执行”的测试步骤模板，服务于脚本化测试、集成测试与命令级回归。

它们 **不是** Chrome MCP 手工验收脚本；Chrome MCP 场景应记录在 `docs/features/` 中。

自动化验收的优先原则：

- 优先验证 application/control/CLI 层，而不是把显示层当作唯一操控入口
- 必须覆盖模块级、控件级与多端适配相关的自动化入口
- Web / Desktop / Android / Docker / CLI 的共享行为应尽量通过统一 control surface 验证

- `acceptance-cases/01_terminology.md`: 术语与规范性用语校验用例。
- `acceptance-cases/02_positioning.md`: Headless CLI 与核心边界用例。
- `acceptance-cases/03_rendering.md`: Markdown 渲染与交互用例。
- `acceptance-cases/04_diff.md`: Diff/Merge/Reconciliation 用例。
- `acceptance-cases/05_ui.md`: UI 通用 + Web/Desktop/Mobile 用例。
- `acceptance-cases/06_network.md`: P2P/WS/Handshake/Sync 用例。
- `acceptance-cases/07_storage_repo.md`: 存储/仓库/路径规范化用例。
- `acceptance-cases/08_auth.md`: 认证与安全用例。
- `acceptance-cases/09_i18n.md`: 国际化用例（对应第 11 章）。
- `acceptance-cases/10_plugins.md`: AI / External Agent / 插件运行时接口预留用例。
- `acceptance-cases/11_commands_settings.md`: CLI/Command Palette/Settings 用例。
- `acceptance-cases/12_tech_release.md`: 技术栈、性能预算、发布与运维用例。
- `acceptance-cases/13_ui_mobile_chat_regression.md`: 移动端 AI Chat 最小回归脚本。
