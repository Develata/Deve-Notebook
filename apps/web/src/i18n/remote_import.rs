//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Remote Import thin-client strings.

use super::Locale;
use deve_core::protocol::{
    RemoteImportBlocker, RemoteImportChangeKind, RemoteImportProjectionOutcome, RemoteImportState,
};

macro_rules! text {
    ($name:ident, $en:literal, $zh:literal) => {
        pub fn $name(locale: Locale) -> &'static str {
            match locale {
                Locale::En => $en,
                Locale::Zh => $zh,
            }
        }
    };
}

text!(title, "Remote Import", "远程导入");
text!(prepare_webdav, "Prepare WebDAV", "准备 WebDAV 导入");
text!(prepare_s3, "Prepare S3", "准备 S3 导入");
text!(refresh, "Refresh", "刷新");
text!(apply, "Apply session", "应用整个会话");
text!(discard, "Discard", "丢弃");
text!(no_sessions, "No import sessions", "暂无导入会话");
text!(no_entries, "No candidate entries", "暂无候选条目");
text!(offline, "Remote Import is offline", "远程导入当前离线");
text!(no_repo, "Select a repository first", "请先选择仓库");
text!(
    scope_transitioning,
    "Repository scope is changing",
    "仓库作用域正在切换"
);
text!(request_pending, "Request in progress…", "请求处理中…");
text!(
    workspace_ingestion_unavailable,
    "Apply is unavailable until workspace ingestion recovers",
    "工作区采集恢复前无法应用"
);
text!(load_more, "Load more", "加载更多");
text!(cleanup_pending, "Cleanup pending", "清理待处理");
text!(
    blocked,
    "Apply blocked by backend review",
    "后端审查已阻止应用"
);
text!(
    select_session,
    "Select a session to review",
    "选择一个会话进行审查"
);

pub fn state(locale: Locale, value: RemoteImportState) -> &'static str {
    match (locale, value) {
        (Locale::En, RemoteImportState::Preparing) => "Preparing",
        (Locale::En, RemoteImportState::Ready) => "Ready",
        (Locale::En, RemoteImportState::Stale) => "Stale",
        (Locale::En, RemoteImportState::Failed) => "Failed",
        (Locale::En, RemoteImportState::Applied) => "Applied",
        (Locale::En, RemoteImportState::Discarded) => "Discarded",
        (Locale::Zh, RemoteImportState::Preparing) => "准备中",
        (Locale::Zh, RemoteImportState::Ready) => "就绪",
        (Locale::Zh, RemoteImportState::Stale) => "已过期",
        (Locale::Zh, RemoteImportState::Failed) => "失败",
        (Locale::Zh, RemoteImportState::Applied) => "已应用",
        (Locale::Zh, RemoteImportState::Discarded) => "已丢弃",
    }
}

pub fn change_kind(locale: Locale, value: RemoteImportChangeKind) -> &'static str {
    match (locale, value) {
        (Locale::En, RemoteImportChangeKind::Added) => "Added",
        (Locale::En, RemoteImportChangeKind::Modified) => "Modified",
        (Locale::En, RemoteImportChangeKind::Unchanged) => "Unchanged",
        (Locale::Zh, RemoteImportChangeKind::Added) => "新增",
        (Locale::Zh, RemoteImportChangeKind::Modified) => "修改",
        (Locale::Zh, RemoteImportChangeKind::Unchanged) => "未变化",
    }
}

pub fn blocker(locale: Locale, value: RemoteImportBlocker) -> &'static str {
    match (locale, value) {
        (Locale::En, RemoteImportBlocker::LedgerHeadDrift) => "Ledger head changed",
        (Locale::En, RemoteImportBlocker::IgnoreSnapshotDrift) => "Ignore rules changed",
        (Locale::En, RemoteImportBlocker::LocatorBindingDrift) => "Remote binding changed",
        (Locale::En, RemoteImportBlocker::PendingOverlap) => "Pending edit overlap",
        (Locale::En, RemoteImportBlocker::StagedOverlap) => "Staged change overlap",
        (Locale::En, RemoteImportBlocker::ArtifactTamper) => "Candidate verification failed",
        (Locale::En, RemoteImportBlocker::RepoMembershipMismatch) => {
            "Repository membership changed"
        }
        (Locale::Zh, RemoteImportBlocker::LedgerHeadDrift) => "Ledger head 已变化",
        (Locale::Zh, RemoteImportBlocker::IgnoreSnapshotDrift) => "忽略规则已变化",
        (Locale::Zh, RemoteImportBlocker::LocatorBindingDrift) => "远端绑定已变化",
        (Locale::Zh, RemoteImportBlocker::PendingOverlap) => "与待处理编辑重叠",
        (Locale::Zh, RemoteImportBlocker::StagedOverlap) => "与已暂存修改重叠",
        (Locale::Zh, RemoteImportBlocker::ArtifactTamper) => "候选验证失败",
        (Locale::Zh, RemoteImportBlocker::RepoMembershipMismatch) => "仓库成员关系已变化",
    }
}

pub fn projection_outcome(locale: Locale, value: RemoteImportProjectionOutcome) -> &'static str {
    match (locale, value) {
        (Locale::En, RemoteImportProjectionOutcome::Pending) => {
            "Ledger committed; projection pending"
        }
        (Locale::En, RemoteImportProjectionOutcome::Written) => {
            "Ledger committed; projection written"
        }
        (Locale::En, RemoteImportProjectionOutcome::Degraded) => {
            "Ledger committed; projection degraded"
        }
        (Locale::Zh, RemoteImportProjectionOutcome::Pending) => "Ledger 已提交；Projection 待处理",
        (Locale::Zh, RemoteImportProjectionOutcome::Written) => "Ledger 已提交；Projection 已写入",
        (Locale::Zh, RemoteImportProjectionOutcome::Degraded) => "Ledger 已提交；Projection 已降级",
    }
}
