# 05_network.md - 网络架构篇 (Network Architecture)

## 拓扑定义：P2P 三角与 Web 面板 (The P2P Triangle & Web Dashboard)

### P2P Mesh (对等网络)
* **核心节点**：仅包含 **Desktop Native (PC/Mac)**、**Mobile Native (iOS/Android)** 和 **Server (Linux)**。
* **机制**：这三方拥有独立的 `PeerID` 和本地数据库 (Redb)，通过 Gossip 协议交换数据。
* **Server 角色**：Server 在 P2P 网络中充当 **Always-on Relay Peer** (全天候中继/备份节点)。

### Web Client (服务器面板)
* **定位**：Web 端**不是**一个独立的 P2P 节点，而是 **Server 节点的远程操作面板 (Remote Dashboard)**。
* **数据源**：Web 端直接通过 WebSocket 读写 Server 的内存/数据库。Web 端显示的 "Local" 即为 Server 的 `local.redb`。
* **存储**：**Stateless / RAM-Only (纯内存)**。Web 端严禁使用 IndexedDB/LocalStorage 存储业务数据。它只是 Server 状态的“易失性投影”。
* **连接约束**：Web 端必须保持与 Server 的 WebSocket 连接才能工作。**断连即锁屏**，严禁离线编辑。

## 连接与协议 (Connection & Protocol)

### 连接策略
* **Core (默认)**：基于 Relay 的连接。所有 Peer 默认连接 Server，通过 Server 转发数据包。
* **Extension (接口)**：预留 `Transport` 接口。允许未来通过插件实现 IPv4/IPv6 直连或 NAT 穿透。

### 同步协议 (Gossip Protocol)
* **Version Vector**：唯一真理。
* **Transport Layer**：
    *   **Relay (Phase 1)**：WebSocket via Server。
    *   **Direct (Phase 2)**：WebRTC/QUIC 预留。
*   **流控**：
    *   MUST 有背压。
    *   **Desktop MUST 支持离线**。
    *   **Web (Exception)**：**严禁离线编辑**。断网即锁屏。
    *   **失败语义**：重连后最终一致。

## 本章相关命令

* 无。

## 本章相关配置

*   `SYNC_MODE`: `auto` (Default, 后台自动拉取与合并) | `manual` (StrictMode, 仅交换 Vector，需显式 Fetch/Merge)。
