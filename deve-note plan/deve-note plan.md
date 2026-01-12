# 📑 Deve-Note Plan - Master Index

**版本**: 0.0.1
**核心理念**: Git-Flow P2P Architecture, Trinity Isolation, Remote Dashboard.

本文档已模块化，请参阅以下子文档获取详细规划：

## 📚 目录 (Table of Contents)

### 1. [Meta & Boundaries](./00_meta.md)
*   项目定位与术语定义 (Terminology)。
*   **核心边界 (Core Boundaries)**：MUST vs MAY。

### 2. [Architecture Philosophy](./01_architecture.md)
*   **Git-Flow P2P 架构**：P2P Triangle (Desktop/Mobile/Server) + Web Dashboard。
*   **Trinity Isolation**：Store A/B/C 三库隔离。
*   **Phase 0**: 核心验证原型。

### 3. [UI Design](./02_ui_design.md)
*   界面设计哲学：Cockpit 概念、键盘优先。
*   **Reactive Projection** & **Optimistic UI** (Web 限制)。

### 4. [UI Architecture](./03_ui_architecture.md)
*   组件系统：Leptos + Tailwind。
*   **Branch Switcher** & **Spectator Mode**。
*   编辑器内核与可视化系统。

### 5. [Backend Architecture](./04_backend.md)
*   **Repository Manager**: Local vs Shadow Repos.
*   **Gossip Protocol**: Sync Mode (Auto/Manual), Flow Control.
*   **Reconciliation**: Conflict Handling.

### 6. [Data Integrity & Flows](./05_data_flows.md)
*   数据恢复与导出。
*   **交互流程 (Flows)**：Alt-Tab 协同、Math 编辑、Git Sync、P2P Merge。

### 7. [Tech Stack](./06_stack.md)
*   技术选型清单。
*   **Performance Profiles**: Low-Spec (512MB) vs Standard.

### 8. [Runtime & Operations](./07_runtime_ops.md)
*   **Dual-Engine Plugins**: Wasm + Podman.
*   AI 扩展与安全性。
*   **Cross-Platform Delivery**: Web/Mobile/Desktop 适配策略。
*   Open Source Playbook.
