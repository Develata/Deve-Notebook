//! plan_ref:
//!   - 11_i18n#i18n-error-code-catalog
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
        (Locale::En, ServerErrorCode::SyncEditRejected) => "Edit rejected",
        (Locale::Zh, ServerErrorCode::SyncEditRejected) => "编辑被拒绝",
        (Locale::En, ServerErrorCode::SyncRepoUnbound) => "Session is not bound to this repository",
        (Locale::Zh, ServerErrorCode::SyncRepoUnbound) => "当前会话尚未绑定到该仓库",
        (Locale::En, ServerErrorCode::SyncPeerUnauthenticated) => {
            "Browser peer is not ready for writing"
        }
        (Locale::Zh, ServerErrorCode::SyncPeerUnauthenticated) => "浏览器写入身份尚未完成认证",
        (Locale::En, ServerErrorCode::SyncDecryptFailed) => "Decryption failed",
        (Locale::Zh, ServerErrorCode::SyncDecryptFailed) => "数据解密失败",
        (Locale::En, ServerErrorCode::ScRepoNotSelected) => "Repository not selected",
        (Locale::Zh, ServerErrorCode::ScRepoNotSelected) => "当前未选择激活仓库",
        (Locale::En, ServerErrorCode::ScRemoteBranchReadonly) => "Remote branch is read-only",
        (Locale::Zh, ServerErrorCode::ScRemoteBranchReadonly) => "远程分支为只读",
        (Locale::En, ServerErrorCode::ScRepoContextInvalid) => "Repository context is invalid",
        (Locale::Zh, ServerErrorCode::ScRepoContextInvalid) => "仓库上下文无效",
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
        (Locale::En, ServerErrorCode::PluginInvalidMessage) => "Invalid plugin host message",
        (Locale::Zh, ServerErrorCode::PluginInvalidMessage) => "插件宿主消息无效",
        (Locale::En, ServerErrorCode::PluginUnsupportedMessage) => {
            "Unsupported plugin host message"
        }
        (Locale::Zh, ServerErrorCode::PluginUnsupportedMessage) => "插件宿主不支持该消息",
    }
}
