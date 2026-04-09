# Deve-Note 详细进度表 (Detailed Schedule)

**上次更新**: 2026.02.28
**当前阶段**: Phase 4 (Extensions) 进行中。Apps Audit 修复完成，Auth 全链路已就绪。

本目录包含根据 `deve-note plan/` 拆分的详细功能验收清单。

## 📚 进度索引

| 模块 | 进度文件 | 对应 Plan | 状态 |
| :--- | :--- | :--- | :--- |
| **核心架构** | [01_core.md](schedules/01_core.md) | 04, 05, 06, 07, 09 | 🟢 **98%** |
| **前端交互** | [02_ui.md](schedules/02_ui.md) | 03, 08, 10, 12, 13 | 🟢 **93%** |
| **扩展系统** | [03_extensions.md](schedules/03_extensions.md) | 11 | 🔴 **0%** |
| **发布运维** | [04_release.md](schedules/04_release.md) | 15 | ⚪ **0%** |

## 🚀 下一步计划 (Next Steps)

1.  **启动插件系统 (Phase 4)**:
    *   集成 **Rhai** 脚本引擎，实现简单的 Hook 系统。
    *   集成 **WASM** 运行时，为 AI 插件做准备。

2.  **完善 UI 细节**:
    *   实现 **Code Block Toolbar** (Copy 按钮)。
    *   完成 **Argon2** 密码哈希验证。

## ⚠️ 验收说明

请对照 `schedules/` 下的子文件逐项验收。打钩 (`[x]`) 表示代码已实现并经过初步验证。
