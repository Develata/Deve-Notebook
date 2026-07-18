# 19_plugins.md - 插件与运行时篇 (Plugins & Runtime)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Deferred`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-14`
- `Counterpart Feature`: `docs/features/17_plugins.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/10_plugins.md`
- `Primary Code Areas`: `crates/core/src/plugin/`, `docs/plan/plugins/`

> 本章不要求新增完整插件平台。AI Chat 归 `16_ai_agent`；Rhai/plugin-host 只能作为外围兼容运行时。未来扩展仅保留 Trusted External Agent Runtime 与 Calculation Runtime。MCP 不作为插件/运行时方向。

## 1. 章节状态

*   **要求**：定义边界、配置位、错误契约与安全前提。
*   **不要求**：
    - 插件安装器
    - 插件市场
    - 新增完整插件平台运行时代码
    - Calculation Runtime 的实际执行器
    - MCP runtime、MCP server 管理或 MCP tool loop

### MCP Retirement Boundary {#skills-cli-extension-boundary}

MCP 相关文字只作为历史决策保留。扩展路线是 Skills 调用受控 CLI 工具，或 `16_ai_agent` 定义的 Trusted CLI path；两者都必须显式启用、资源受限、默认只读并 fail-closed。MCP 企划不得复用 Rhai/plugin-host 边界，也不得绕过 Native AI Chat 的 read-first 默认策略。

## 2. Existing Rhai Plugin Host Boundary {#plugin-runtime-boundary}

允许保留轻量 Rhai runtime、manifest/capability 模型和 plugin-host / `PluginCall` 兼容边界，但这些能力属于外围系统。

### 兼容实现范围

*   兼容 plugin host **MAY** 保留 manifest、capability、Rhai runtime、host API、loader 与 `PluginCall` / `PluginResponse` 处理。
*   plugin-host 只能暴露外围调用入口，不得成为核心 notebook authority、repo scope 或 write pipeline 的替代入口。

### 必须保持的边界

*   **Capabilities Default Deny**：manifest capability 未声明时，host API 必须拒绝对应能力。
*   **Fail-Closed RPC**：非法消息、未知插件、运行时错误、不可序列化结果都必须返回结构化错误，不得静默成功或 fallback 到核心命令。
*   **Ledger-Managed Boundary**：托管笔记、`.notegit/` 与 ledger 对象不得通过裸文件写入绕过 authority；若需要写托管笔记，必须走 ledger-aware host functions。
*   **Managed Note Mutation Host**：plugin `note_write` 必须通过窄接口
    `ManagedNoteMutationHost` 注入 server adapter，并进入当前 repo 的
    `RepoMutationPublicationGate`。Core plugin runtime 不得反向依赖 CLI；Rhai 解析/执行不得持有
    repo permit；正文读取与 patch 准备在锁外完成，只有 exact-compare repo identity、ledger head 与 path
    binding 后的 authority transaction 可以进入临界区。
*   **Managed Source-Control Mutation Host**：local plugin `sc_commit` 必须通过独立窄接口
    `ManagedSourceControlMutationHost` 注入 server adapter，并进入同一个 repo mutation/publication gate；
    不得复用 HTTP facade 绕过 gate。Core plugin runtime 不得反向依赖 CLI。
*   **Source-Control Writer Gate**：plugin-host 暴露 source-control writer host functions 时，必须显式接入当前 repo/sync writer gate 与 NoteGit/ngit authority；不得接收 mirror/off 之类的 legacy bridge policy。缺少本地 managed host 时必须 fail-closed，除非调用目标是明确的 remote proxy delegated API。delegated API 必须以显式 authority（例如 `DelegatedRemoteProxy`）进入，不得把 `REMOTE_PROXY_SCOPE_NONCE = 1` 解释为普通 browser HTTP mutation grant。
*   未引入认证层的 plugin-host satellite 必须绑定 loopback，不得默认监听 `0.0.0.0`。
*   `agent-bridge` 的拦截属于 `16_ai_agent` 的 Trusted External Agent Bridge，不得被重新包装成通用插件平台能力。

## 3. Trusted External Agent Runtime

此能力是外部 CLI Agent 的未来接口位，不是默认产品能力。

### 基本原则

*   **Default Off**：默认关闭。
*   **Trusted Only**：仅面向受信任单用户部署。
*   **Fail-Closed**：安全壳不成立时，必须禁用，而不是降级为“直接放行子进程”。

### 预留边界

*   **Frontend**：统一 chat UI、上下文展示与结果流式渲染。
*   **Backend**：如未来启用，负责 CLI lifecycle、stdout streaming、资源约束与错误收敛。
*   **Trusted External Agent** **MUST NOT** 被视为通用插件市场能力；它是高级部署选项。

### 最低安全前提

*   固定路径或 allowlist，不接受任意 PATH 搜索。
*   环境变量白名单。
*   超时 / 输出上限 / 并发上限。
*   默认只读上下文，不得直接获得 Ledger 管理对象写权限。
*   无法满足这些条件时，对应 release **MAY** 完全不提供此能力。

## 4. Calculation Runtime

Calculation Runtime 仍然是长期能力，但本章**不要求代码实现**。

### 目标用途

*   执行需要完整 OS 环境的不可信代码块（如 Python / R / shell notebook）。

### 目标安全模型

*   **Podman (Rootless)** + **OCI Containers**
*   **No Root**
*   **No Net**（除非用户显式授权）
*   **Ephemeral**（用完即焚）

### 接口阶段要求

*   只预留接口与配置位。
*   不要求实际执行器、调度器、镜像管理或 UI 面板落地。

## 5. Common Boundary (通用边界)

若未来重新展开插件体系，仍遵循以下原则：

*   **Capabilities Default Deny**
*   **Ledger-Managed Boundary**：
    - `<projection_base>/<workspace_segment>/**/*.md`
    - `<projection_base>/<workspace_segment>/.notegit/**`
    - `ledger/**`
    这些对象不得通过原始文件写入直接修改。
*   若未来需要写托管笔记，**MUST** 走 ledger-aware host functions，而不是裸 `fs_write`。

## 6. Related Commands

*   无必须实现的命令。

## 7. Related Configuration

*   `AGENT_CLI_PATH`: 未来 Trusted External Agent 的可执行路径。
*   `plugin.podman.path`: 未来 Calculation Runtime 的 Podman 路径。
