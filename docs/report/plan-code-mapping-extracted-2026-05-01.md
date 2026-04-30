# Plan Code Mapping Extraction - 2026-05-01

本报告记录一次文档分层清理：`docs/plan/` 是最终工程蓝图，不再保存当前代码路径、`Primary Code Areas`、`Code Mapping` 或 `Code Refs`。

## 迁出原则

- `docs/plan/` 只保留 authority、runtime、protocol、state machine、failure boundary 与 refactor target。
- 当前代码路径、实现覆盖、文件职责、测试路径属于非权威审计信息，应放在 `docs/report/`、`docs/overview/`，或由工具生成。
- 代码到 plan 的权威连接由 Rust 文件头 `plan_ref:` 与 `scripts/plan-coverage.sh` 维护。
- 后续不得把一次性实现扫描结果回写到 plan 正文。

## 迁出范围

本批次从 plan 章节中移除了以下类型信息：

- Metadata 中的 `Primary Code Areas`。
- 正文中的 `Code Refs`。
- 以当前源文件路径为主体的 `Module Boundary`。
- 独立 `Code Mapping` 章节。
- 报告式“当前实现分散在...”说明。

## 后续维护方式

- 需要查看代码覆盖：运行 `scripts/plan-coverage.sh`。
- 需要阶段性代码/plan 差距分析：新增 `docs/report/*-status-YYYY-MM-DD.md`。
- 需要架构视图：更新 `docs/overview/architecture-code.lisp` 与 `docs/overview/architecture-diff.md`。
- 需要稳定目标：只更新 `docs/plan/` 的抽象合同与稳定 anchor。
