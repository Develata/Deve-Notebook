# Learnings - WebLightPeer Audit Unification

记录执行过程中发现的模式、约定和最佳实践。

---

## [2026-03-06 T1] WebLightPeer 术语定义

**关键发现**:
- 原文档将 Web 描述为 "不是 P2P 节点"，但实际代码中浏览器持久化 IdentityKeyPair 并发送 SyncHello（peer 行为）
- 新定义明确 WebLightPeer 为"受限同步端点"，避免"纯 dashboard"与"完全 peer"的模糊地带
- 核心约束：repo-scoped isolation, online dependency, storage separation, auth layering

**术语表**:
- WebLightPeer, DashboardSession, PeerIdentity, RepoScopedVector, OfflineCache, DegradedSyncMode

**不变量**:
- INV-1: Repo Scope Isolation
- INV-2: Online Dependency
- INV-3: Storage Separation
- INV-4: Auth Layering
