# 06_repository.md - 仓库与分支篇 (Repository & Branching)

## 仓库管理 (Repository Manager)

*   **Repository Identification (仓库标识)**:
    *   **Basis**: 基于 URL 唯一标识 (e.g., `https://deve-note1.me`).
    *   **Storage**: 1 Repo = 1 Database File + 1 Local Folder.
*   **P2P Connection Strategy (连接策略)**:
    *   **Match**: URL 相同 -> 同一仓库协作 (显示 Shadow Branches)。
    *   **Mismatch**: URL 不同 -> **Multi-Root Workspace** (侧边栏分列显示 Local Repos + Peer Repos)。
    *   **Access Control**: Peer-only Repos (URL 不匹配) 强制为 **Read-Only** (仅允许 Copy/Diff)。

## 严格分支策略 (Strict Branching Policy)

*   **No Arbitrary Creation**: ❌ 禁止随意创建新分支 (No `git checkout -b`).
*   **Establishment Only (仅确立)**: ✅ 唯一创建 Local Branch 的方式是 **"Establish from Remote"** (即激活已存在的 Remote/Shadow 分支)。
*   **Deletion Rule**: 允许删除 Local Branch。
*   **Last Man Standing**: ⚠️ **禁止删除仓库的最后一个分支**。若要删除，必须执行 "Delete Repository" (删除整个库)。

## 分支切换与交互 (Branch Switching)

* **分支切换器**：UI 提供类似 VS Code 左下角的分支切换功能。
    * 切到 `Local`：读写 Store B + Store A。
    * 切到 `Peer Mobile`：**只读模式 (Spectator Mode)**。VFS 挂载点切换，直接从 `remotes/peer_mobile.redb` 读取并在内存中生成文件树。

## 本章相关命令

*   `P2P: Switch to Peer`: 切换到指定 Peer 的影子分支 (进入 Spectator Mode)。
*   `P2P: Establish Branch`: 从当前查看的 Peer 分支创建本地分支。

## 本章相关配置

* 无。
