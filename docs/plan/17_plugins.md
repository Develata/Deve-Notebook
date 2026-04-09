# 17_plugins.md - 插件与运行时篇 (Plugins & Runtime)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Deferred`
- `Counterpart Feature`: `docs/features/17_plugins.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/10_plugins.md`
- `Primary Code Areas`: `crates/core/src/plugin/`, `docs/plan/plugins/`

> 本章当前**只做接口预留，不要求代码实现**。
> AI Chat 已提升为第 10 章的原生产品能力，不再视为插件主线。
> 本章当前只保留两类未来扩展：**Trusted External Agent Runtime** 与 **Calculation Runtime**。

## 1. 章节状态

*   **当前要求**：定义边界、配置位、错误契约与安全前提。
*   **当前不要求**：
    - 插件安装器
    - 插件市场
    - 完整运行时代码
    - Calculation Runtime 的实际执行器

## 2. Trusted External Agent Runtime（未来扩展）

此能力对应“外部 CLI Agent 接入”的未来接口位，而不是默认产品能力。

### 基本原则

*   **Default Off**：默认关闭。
*   **Trusted Only**：仅面向受信任单用户部署。
*   **Fail-Closed**：安全壳不成立时，必须禁用，而不是降级为“直接放行子进程”。

### 预留边界

*   **Frontend**：只负责统一 chat UI、上下文展示与结果流式渲染。
*   **Backend**：如未来启用，负责 CLI lifecycle、stdout streaming、资源约束与错误收敛。
*   **Trusted External Agent** **MUST NOT** 被视为通用插件市场能力；它是高级部署选项。

### 最低安全前提

*   固定路径或 allowlist，不接受任意 PATH 搜索。
*   环境变量白名单。
*   超时 / 输出上限 / 并发上限。
*   默认只读上下文，不得直接获得 Ledger 管理对象写权限。
*   无法满足这些条件时，当前 release **MAY** 完全不提供此能力。

## 3. Calculation Runtime（计算引擎）

Calculation Runtime 仍然是长期能力，但当前**不要求代码实现**。

### 目标用途

*   执行需要完整 OS 环境的不可信代码块（如 Python / R / shell notebook）。

### 目标安全模型

*   **Podman (Rootless)** + **OCI Containers**
*   **No Root**
*   **No Net**（除非用户显式授权）
*   **Ephemeral**（用完即焚）

### 当前阶段要求

*   只预留接口与配置位。
*   不要求实际执行器、调度器、镜像管理或 UI 面板落地。

## 4. 通用边界（接口预留）

若未来重新展开插件体系，仍遵循以下原则：

*   **Capabilities Default Deny**
*   **Ledger-Managed Boundary**：
    - `vault/<repo>/**/*.md`
    - `vault/<repo>/.notegit/**`
    - `ledger/**`
    这些对象不得通过原始文件写入直接修改。
*   若未来需要写托管笔记，**MUST** 走 ledger-aware host functions，而不是裸 `fs_write`。

## 本章相关命令

*   当前无必须实现的命令。

## 本章相关配置

*   `AGENT_CLI_PATH`: 未来 Trusted External Agent 的可执行路径。
*   `plugin.podman.path`: 未来 Calculation Runtime 的 Podman 路径。
