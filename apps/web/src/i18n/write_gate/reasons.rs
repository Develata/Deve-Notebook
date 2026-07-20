//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 09_web_thin_client_ledger#write-readiness

use crate::i18n::Locale;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteGateReason {
    SessionExpired,
    NativeBootstrapInvalid,
    NativeSessionPending,
    NativeServiceOffline,
    NativeReprobeRequired,
    Offline,
    Reconnecting,
    SnapshotLoading,
    ReadOnly,
    ScopeSwitching,
    NoRepo,
    WorkspaceIngestionUnavailable,
    HandshakingRepo,
    LocalRepoScopeUnstable,
    ScopeNonceExhausted,
    EmptyRepositoryName,
    RemoteBranchView,
    NoActiveDocument,
    DocumentTooLarge,
    WriterClientIdUnavailable,
    CurrentRepoIdUnavailable,
    FailedApplyCodeLocally,
    NoCurrentDocumentSelected,
    FileTooLarge,
    FileReaderUnavailable,
    FileReadFailed,
}

pub fn reason_label(locale: Locale, reason: WriteGateReason) -> &'static str {
    match (locale, reason) {
        (Locale::En, WriteGateReason::SessionExpired) => "session expired",
        (Locale::Zh, WriteGateReason::SessionExpired) => "会话已过期",
        (Locale::En, WriteGateReason::NativeBootstrapInvalid) => "native bootstrap invalid",
        (Locale::Zh, WriteGateReason::NativeBootstrapInvalid) => "原生启动信息无效",
        (Locale::En, WriteGateReason::NativeSessionPending) => "native session pending",
        (Locale::Zh, WriteGateReason::NativeSessionPending) => "原生会话仍在准备",
        (Locale::En, WriteGateReason::NativeServiceOffline) => "native service offline",
        (Locale::Zh, WriteGateReason::NativeServiceOffline) => "原生服务离线",
        (Locale::En, WriteGateReason::NativeReprobeRequired) => "native reprobe required",
        (Locale::Zh, WriteGateReason::NativeReprobeRequired) => "需要重新探测原生服务",
        (Locale::En, WriteGateReason::Offline) => "offline",
        (Locale::Zh, WriteGateReason::Offline) => "离线",
        (Locale::En, WriteGateReason::Reconnecting) => "reconnecting",
        (Locale::Zh, WriteGateReason::Reconnecting) => "正在重连",
        (Locale::En, WriteGateReason::SnapshotLoading) => "snapshot loading",
        (Locale::Zh, WriteGateReason::SnapshotLoading) => "正在加载快照",
        (Locale::En, WriteGateReason::ReadOnly) => "read-only",
        (Locale::Zh, WriteGateReason::ReadOnly) => "只读模式",
        (Locale::En, WriteGateReason::ScopeSwitching) => "scope switching",
        (Locale::Zh, WriteGateReason::ScopeSwitching) => "正在切换作用域",
        (Locale::En, WriteGateReason::NoRepo) => "no repo selected",
        (Locale::Zh, WriteGateReason::NoRepo) => "尚未选择仓库",
        (Locale::En, WriteGateReason::WorkspaceIngestionUnavailable) => {
            super::super::workspace_ingestion::unavailable(Locale::En)
        }
        (Locale::Zh, WriteGateReason::WorkspaceIngestionUnavailable) => {
            super::super::workspace_ingestion::unavailable(Locale::Zh)
        }
        (Locale::En, WriteGateReason::HandshakingRepo) => "repo handshaking",
        (Locale::Zh, WriteGateReason::HandshakingRepo) => "正在协商仓库写入权限",
        (Locale::En, WriteGateReason::LocalRepoScopeUnstable) => "local repo scope is not stable",
        (Locale::Zh, WriteGateReason::LocalRepoScopeUnstable) => "本地仓库作用域不稳定",
        (Locale::En, WriteGateReason::ScopeNonceExhausted) => "scope nonce exhausted",
        (Locale::Zh, WriteGateReason::ScopeNonceExhausted) => "作用域 nonce 已耗尽",
        (Locale::En, WriteGateReason::EmptyRepositoryName) => "empty repository name",
        (Locale::Zh, WriteGateReason::EmptyRepositoryName) => "仓库名称为空",
        (Locale::En, WriteGateReason::RemoteBranchView) => "remote branch view",
        (Locale::Zh, WriteGateReason::RemoteBranchView) => "当前是远程分支视图",
        (Locale::En, WriteGateReason::NoActiveDocument) => "no active document",
        (Locale::Zh, WriteGateReason::NoActiveDocument) => "当前没有打开的文档",
        (Locale::En, WriteGateReason::DocumentTooLarge) => "document is too large",
        (Locale::Zh, WriteGateReason::DocumentTooLarge) => "文档过大",
        (Locale::En, WriteGateReason::WriterClientIdUnavailable) => "writer client id unavailable",
        (Locale::Zh, WriteGateReason::WriterClientIdUnavailable) => "写入客户端 ID 不可用",
        (Locale::En, WriteGateReason::CurrentRepoIdUnavailable) => "current repo id unavailable",
        (Locale::Zh, WriteGateReason::CurrentRepoIdUnavailable) => "当前仓库 ID 不可用",
        (Locale::En, WriteGateReason::FailedApplyCodeLocally) => "failed to apply code locally",
        (Locale::Zh, WriteGateReason::FailedApplyCodeLocally) => "本地应用代码失败",
        (Locale::En, WriteGateReason::NoCurrentDocumentSelected) => "no current document selected",
        (Locale::Zh, WriteGateReason::NoCurrentDocumentSelected) => "当前未选择文档",
        (Locale::En, WriteGateReason::FileTooLarge) => "file is larger than 1 MiB",
        (Locale::Zh, WriteGateReason::FileTooLarge) => "文件超过 1 MiB",
        (Locale::En, WriteGateReason::FileReaderUnavailable) => "file reader is unavailable",
        (Locale::Zh, WriteGateReason::FileReaderUnavailable) => "文件读取器不可用",
        (Locale::En, WriteGateReason::FileReadFailed) => "file read failed",
        (Locale::Zh, WriteGateReason::FileReadFailed) => "文件读取失败",
    }
}
