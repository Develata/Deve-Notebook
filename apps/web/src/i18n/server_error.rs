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
        (Locale::En, ServerErrorCode::RemoteProjectionLocatorInvalid) => {
            "Remote projection location is invalid"
        }
        (Locale::Zh, ServerErrorCode::RemoteProjectionLocatorInvalid) => "远程投影位置无效",
        (Locale::En, ServerErrorCode::RemoteProjectionProviderUnavailable) => {
            "Remote projection provider is unavailable"
        }
        (Locale::Zh, ServerErrorCode::RemoteProjectionProviderUnavailable) => {
            "远程投影服务暂不可用"
        }
        (Locale::En, ServerErrorCode::RemoteProjectionPushFailed) => {
            "Remote projection push failed"
        }
        (Locale::Zh, ServerErrorCode::RemoteProjectionPushFailed) => "远程投影推送失败",
        (Locale::En, ServerErrorCode::RemoteImportActiveSession) => {
            "A remote import is already active"
        }
        (Locale::Zh, ServerErrorCode::RemoteImportActiveSession) => "已存在活动的远程导入",
        (Locale::En, ServerErrorCode::RemoteImportNotFound) => "Remote import was not found",
        (Locale::Zh, ServerErrorCode::RemoteImportNotFound) => "未找到远程导入",
        (Locale::En, ServerErrorCode::RemoteImportStale) => "Remote import is stale",
        (Locale::Zh, ServerErrorCode::RemoteImportStale) => "远程导入已过期",
        (Locale::En, ServerErrorCode::RemoteImportBlocked) => "Remote import is blocked",
        (Locale::Zh, ServerErrorCode::RemoteImportBlocked) => "远程导入被阻塞",
        (Locale::En, ServerErrorCode::RemoteImportInvalidState) => {
            "Remote import state does not allow this action"
        }
        (Locale::Zh, ServerErrorCode::RemoteImportInvalidState) => "当前远程导入状态不允许此操作",
        (Locale::En, ServerErrorCode::RemoteImportLimitExceeded) => {
            "Remote import exceeds the supported limit"
        }
        (Locale::Zh, ServerErrorCode::RemoteImportLimitExceeded) => "远程导入超过支持限制",
        (Locale::En, ServerErrorCode::RemoteImportPrepareFailed) => {
            "Remote import preparation failed"
        }
        (Locale::Zh, ServerErrorCode::RemoteImportPrepareFailed) => "远程导入准备失败",
        (Locale::En, ServerErrorCode::RemoteImportApplyFailed) => "Remote import apply failed",
        (Locale::Zh, ServerErrorCode::RemoteImportApplyFailed) => "远程导入应用失败",
        (Locale::En, ServerErrorCode::RemoteImportCleanupRequired) => {
            "Remote import cleanup is required"
        }
        (Locale::Zh, ServerErrorCode::RemoteImportCleanupRequired) => "远程导入需要清理",
        (Locale::En, ServerErrorCode::RepoAliasInvalid) => "Repository alias is invalid",
        (Locale::Zh, ServerErrorCode::RepoAliasInvalid) => "仓库别名无效",
        (Locale::En, ServerErrorCode::RepoAliasStale) => {
            "Repository alias changed; refresh and retry"
        }
        (Locale::Zh, ServerErrorCode::RepoAliasStale) => "仓库别名已变化，请刷新后重试",
        (Locale::En, ServerErrorCode::RepoAliasStoreFailed) => "Failed to save repository alias",
        (Locale::Zh, ServerErrorCode::RepoAliasStoreFailed) => "仓库别名保存失败",
        (Locale::En, ServerErrorCode::RepoLifecycleBusy) => "Repository lifecycle is busy",
        (Locale::Zh, ServerErrorCode::RepoLifecycleBusy) => "仓库生命周期正忙",
        (Locale::En, ServerErrorCode::RepoLifecycleNotFound) => {
            "Repository lifecycle request was not found"
        }
        (Locale::Zh, ServerErrorCode::RepoLifecycleNotFound) => "未找到仓库生命周期请求",
        (Locale::En, ServerErrorCode::RepoLifecycleInvalidRequest) => {
            "Repository lifecycle request is invalid"
        }
        (Locale::Zh, ServerErrorCode::RepoLifecycleInvalidRequest) => "仓库生命周期请求无效",
        (Locale::En, ServerErrorCode::RepoLifecycleCommittedPartial) => {
            "Repository change committed with limited availability"
        }
        (Locale::Zh, ServerErrorCode::RepoLifecycleCommittedPartial) => {
            "仓库变更已提交，但当前可用性受限"
        }
        (Locale::En, ServerErrorCode::RepoLifecycleRepairRequired) => {
            "Repository lifecycle requires repair"
        }
        (Locale::Zh, ServerErrorCode::RepoLifecycleRepairRequired) => "仓库生命周期需要修复",
        (Locale::En, ServerErrorCode::RepoLifecyclePublicationPending) => {
            "Repository update publication is pending"
        }
        (Locale::Zh, ServerErrorCode::RepoLifecyclePublicationPending) => "仓库更新仍待发布",
        (Locale::En, ServerErrorCode::GraphDegradedProjectionRequired) => {
            "Graph projection requires explicit degraded export"
        }
        (Locale::Zh, ServerErrorCode::GraphDegradedProjectionRequired) => {
            "图谱投影需要显式降级导出"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Locale, ServerErrorCode, message};

    #[test]
    fn remote_import_messages_match_authoritative_bilingual_catalog() {
        let cases = [
            (
                ServerErrorCode::RemoteImportActiveSession,
                "A remote import is already active",
                "已存在活动的远程导入",
            ),
            (
                ServerErrorCode::RemoteImportNotFound,
                "Remote import was not found",
                "未找到远程导入",
            ),
            (
                ServerErrorCode::RemoteImportStale,
                "Remote import is stale",
                "远程导入已过期",
            ),
            (
                ServerErrorCode::RemoteImportBlocked,
                "Remote import is blocked",
                "远程导入被阻塞",
            ),
            (
                ServerErrorCode::RemoteImportInvalidState,
                "Remote import state does not allow this action",
                "当前远程导入状态不允许此操作",
            ),
            (
                ServerErrorCode::RemoteImportLimitExceeded,
                "Remote import exceeds the supported limit",
                "远程导入超过支持限制",
            ),
            (
                ServerErrorCode::RemoteImportPrepareFailed,
                "Remote import preparation failed",
                "远程导入准备失败",
            ),
            (
                ServerErrorCode::RemoteImportApplyFailed,
                "Remote import apply failed",
                "远程导入应用失败",
            ),
            (
                ServerErrorCode::RemoteImportCleanupRequired,
                "Remote import cleanup is required",
                "远程导入需要清理",
            ),
        ];

        for (code, english, chinese) in cases {
            assert_eq!(message(Locale::En, code), english, "English {code:?}");
            assert_eq!(message(Locale::Zh, code), chinese, "Chinese {code:?}");
        }
    }
}
