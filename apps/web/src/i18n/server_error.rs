//! plan_ref:
//!   - 13_i18n#i18n-error-code-catalog
//!

use super::Locale;
use deve_core::protocol::ServerErrorCode;

pub fn message(locale: Locale, code: ServerErrorCode) -> &'static str {
    match (locale, code) {
        (Locale::En, ServerErrorCode::RequestFailed) => "Request failed",
        (Locale::Zh, ServerErrorCode::RequestFailed) => "请求失败",
        (Locale::En, ServerErrorCode::AuthTokenExpired) => "Token expired",
        (Locale::Zh, ServerErrorCode::AuthTokenExpired) => "凭证已过期",
        (Locale::En, ServerErrorCode::AuthTokenMissing) => "Authentication required",
        (Locale::Zh, ServerErrorCode::AuthTokenMissing) => "需要登录",
        (Locale::En, ServerErrorCode::AuthInvalidPassword) => "Invalid password",
        (Locale::Zh, ServerErrorCode::AuthInvalidPassword) => "密码错误",
        (Locale::En, ServerErrorCode::AuthRateLimited) => "Too many attempts",
        (Locale::Zh, ServerErrorCode::AuthRateLimited) => "请求频率超限",
        (Locale::En, ServerErrorCode::AuthCsrfMismatch) => "Request rejected",
        (Locale::Zh, ServerErrorCode::AuthCsrfMismatch) => "请求被拒绝",
        (Locale::En, ServerErrorCode::SyncEditRejected) => "Edit rejected",
        (Locale::Zh, ServerErrorCode::SyncEditRejected) => "编辑被拒绝",
        (Locale::En, ServerErrorCode::SyncRepoUnbound) => "Session is not bound to this repository",
        (Locale::Zh, ServerErrorCode::SyncRepoUnbound) => "当前会话尚未绑定到该仓库",
        (Locale::En, ServerErrorCode::SyncRepoRouteMismatch) => "Repository route mismatch",
        (Locale::Zh, ServerErrorCode::SyncRepoRouteMismatch) => "仓库路由不匹配",
        (Locale::En, ServerErrorCode::SyncSnapshotRequired) => "Snapshot refresh required",
        (Locale::Zh, ServerErrorCode::SyncSnapshotRequired) => "需要刷新快照",
        (Locale::En, ServerErrorCode::SyncInvalidPayload) => "Invalid sync payload",
        (Locale::Zh, ServerErrorCode::SyncInvalidPayload) => "同步载荷无效",
        (Locale::En, ServerErrorCode::SyncPeerUnauthenticated) => {
            "Browser peer is not ready for writing"
        }
        (Locale::Zh, ServerErrorCode::SyncPeerUnauthenticated) => "浏览器写入身份尚未完成认证",
        (Locale::En, ServerErrorCode::SyncPeerUnknown) => "Unknown peer",
        (Locale::Zh, ServerErrorCode::SyncPeerUnknown) => "未知节点",
        (Locale::En, ServerErrorCode::SyncVersionMismatch) => "Protocol mismatch",
        (Locale::Zh, ServerErrorCode::SyncVersionMismatch) => "协议版本不兼容",
        (Locale::En, ServerErrorCode::SyncDecryptFailed) => "Decryption failed",
        (Locale::Zh, ServerErrorCode::SyncDecryptFailed) => "数据解密失败",
        (Locale::En, ServerErrorCode::SyncDisconnected) => "Connection lost",
        (Locale::Zh, ServerErrorCode::SyncDisconnected) => "连接已断开",
        (Locale::En, ServerErrorCode::ScRepoNotSelected) => "Repository not selected",
        (Locale::Zh, ServerErrorCode::ScRepoNotSelected) => "当前未选择激活仓库",
        (Locale::En, ServerErrorCode::ScRemoteBranchReadonly) => "Remote branch is read-only",
        (Locale::Zh, ServerErrorCode::ScRemoteBranchReadonly) => "远程分支为只读",
        (Locale::En, ServerErrorCode::ScRepoContextInvalid) => "Repository context is invalid",
        (Locale::Zh, ServerErrorCode::ScRepoContextInvalid) => "仓库上下文无效",
        (Locale::En, ServerErrorCode::ScStaleScope) => "Repository scope is stale",
        (Locale::Zh, ServerErrorCode::ScStaleScope) => "仓库作用域已过期",
        (Locale::En, ServerErrorCode::ScPendingNotFound) => "Pending change not found",
        (Locale::Zh, ServerErrorCode::ScPendingNotFound) => "待处理变更不存在",
        (Locale::En, ServerErrorCode::ScStagedNotFound) => "Staged change not found",
        (Locale::Zh, ServerErrorCode::ScStagedNotFound) => "暂存变更不存在",
        (Locale::En, ServerErrorCode::ScDocNotFound) => "Document not found",
        (Locale::Zh, ServerErrorCode::ScDocNotFound) => "文档不存在",
        (Locale::En, ServerErrorCode::ScCommitNotFound) => "Commit not found",
        (Locale::Zh, ServerErrorCode::ScCommitNotFound) => "提交不存在",
        (Locale::En, ServerErrorCode::ScCommitDiffUnprojectable) => "Commit diff unavailable",
        (Locale::Zh, ServerErrorCode::ScCommitDiffUnprojectable) => "提交差异不可用",
        (Locale::En, ServerErrorCode::ScNothingToCommit) => "Nothing to commit",
        (Locale::Zh, ServerErrorCode::ScNothingToCommit) => "没有可提交内容",
        (Locale::En, ServerErrorCode::ScConflictTargetMissing) => "Conflict target missing",
        (Locale::Zh, ServerErrorCode::ScConflictTargetMissing) => "冲突目标已失效",
        (Locale::En, ServerErrorCode::StorageDbLocked) => "Database is locked",
        (Locale::Zh, ServerErrorCode::StorageDbLocked) => "数据库被锁定",
        (Locale::En, ServerErrorCode::StorageNotFound) => "Document not found",
        (Locale::Zh, ServerErrorCode::StorageNotFound) => "文档不存在",
        (Locale::En, ServerErrorCode::StorageConflict) => "Write conflict",
        (Locale::Zh, ServerErrorCode::StorageConflict) => "写入冲突",
        (Locale::En, ServerErrorCode::StoragePersistFailed) => "Failed to persist change",
        (Locale::Zh, ServerErrorCode::StoragePersistFailed) => "变更持久化失败",
        (locale, ServerErrorCode::StorageWorkspaceIngestionUnavailable) => {
            super::workspace_ingestion::unavailable(locale)
        }
        (Locale::En, ServerErrorCode::DocNotFound) => "Document not found",
        (Locale::Zh, ServerErrorCode::DocNotFound) => "文档不存在",
        (Locale::En, ServerErrorCode::DocContextInvalid) => "Document context is invalid",
        (Locale::Zh, ServerErrorCode::DocContextInvalid) => "文档上下文无效",
        (Locale::En, ServerErrorCode::PluginInvalidMessage) => "Invalid plugin host message",
        (Locale::Zh, ServerErrorCode::PluginInvalidMessage) => "插件宿主消息无效",
        (Locale::En, ServerErrorCode::PluginUnsupportedMessage) => {
            "Unsupported plugin host message"
        }
        (Locale::Zh, ServerErrorCode::PluginUnsupportedMessage) => "插件宿主不支持该消息",
        (Locale::En, ServerErrorCode::PluginUnknownPlugin) => "Plugin not found",
        (Locale::Zh, ServerErrorCode::PluginUnknownPlugin) => "插件不存在",
        (Locale::En, ServerErrorCode::PluginCapabilityDenied) => "Plugin capability denied",
        (Locale::Zh, ServerErrorCode::PluginCapabilityDenied) => "插件能力未授权",
        (Locale::En, ServerErrorCode::PluginRuntimeError) => "Plugin runtime error",
        (Locale::Zh, ServerErrorCode::PluginRuntimeError) => "插件运行时错误",
        (Locale::En, ServerErrorCode::PluginSerializationError) => {
            "Plugin result is not serializable"
        }
        (Locale::Zh, ServerErrorCode::PluginSerializationError) => "插件结果不可序列化",
        (Locale::En, ServerErrorCode::DiffResourceLimit) => {
            "Diff exceeds the supported resource limit"
        }
        (Locale::Zh, ServerErrorCode::DiffResourceLimit) => "差异超过支持的资源限制",
        (Locale::En, ServerErrorCode::DiffComputeFailed) => "Diff projection could not be computed",
        (Locale::Zh, ServerErrorCode::DiffComputeFailed) => "无法计算差异投影",
        (Locale::En, ServerErrorCode::GraphDegradedProjectionRequired) => {
            "Graph projection requires explicit degraded export"
        }
        (Locale::Zh, ServerErrorCode::GraphDegradedProjectionRequired) => {
            "图谱投影需要显式降级导出"
        }
    }
}
