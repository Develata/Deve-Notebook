# 08_ui_design_01_web - Web UI

## Server Dashboard (服务器面板)
* **定位**：Web 端作为 Server 节点的远程操作面板 (Remote Dashboard)。
* **限制**：
    *   **RAM-Only**: 严禁使用 IndexedDB 持久化数据。
    *   **断连锁屏**: 检测到 WebSocket 心跳丢失时，界面 **MUST** 立即进入锁定/只读状态，提示“连接断开”，严禁离线编辑。
    *   **乐观 UI**: 仅在连接存活时有效。
    *   **External Edit Flow (外部协同)**: 
        1.  VS Code 修改 -> 保存。
        2.  后端检测 -> 推送 Ops。
        3.  前端平滑更新 -> 弹出非侵入式 Toast 提示：“已合并外部修改”。
