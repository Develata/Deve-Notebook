//! plan_ref:
//!   - 13_i18n#i18n-resource-management
//!   - 04_repository#local-repo-removal-contract

use super::Locale;
use deve_core::protocol::{
    LocalRepoRemovalBlocker, LocalRepoRemovalDeletedCategory, LocalRepoRemovalPreservedCategory,
    LocalRepoRemovalWarning,
};

pub fn remove_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remove local repository",
        Locale::Zh => "移除本地仓库",
    }
}

pub fn remove_subject(locale: Locale, alias: &str) -> String {
    match locale {
        Locale::En => format!("Review what happens to “{alias}” before continuing."),
        Locale::Zh => format!("继续前，请检查移除“{alias}”会产生的结果。"),
    }
}

pub fn irreversible_warning(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Ledger history and Deve runtime state will be permanently deleted. Workspace files and .git will be kept."
        }
        Locale::Zh => "Ledger 历史与 Deve 运行时状态将被永久删除；Workspace 文件与 .git 将保留。",
    }
}

pub fn deleted_heading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Deve-owned state to delete",
        Locale::Zh => "将删除的 Deve 自有状态",
    }
}

pub fn preserved_heading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Files and state to keep",
        Locale::Zh => "将保留的文件与状态",
    }
}

pub fn warnings_heading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Warnings",
        Locale::Zh => "注意事项",
    }
}

pub fn blockers_heading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Removal blockers",
        Locale::Zh => "移除阻断项",
    }
}

pub fn blocked(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Removal cannot continue until every blocker is resolved.",
        Locale::Zh => "仍有阻断项，解决前无法继续移除。",
    }
}

pub fn cancel(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Keep repository",
        Locale::Zh => "保留仓库",
    }
}

pub fn confirm(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Permanently remove Deve state",
        Locale::Zh => "永久移除 Deve 状态",
    }
}

pub fn deleted(locale: Locale, value: LocalRepoRemovalDeletedCategory) -> &'static str {
    match (locale, value) {
        (Locale::En, LocalRepoRemovalDeletedCategory::LocalLedgerAuthority) => {
            "Local Ledger authority"
        }
        (Locale::Zh, LocalRepoRemovalDeletedCategory::LocalLedgerAuthority) => {
            "本地 Ledger 权威数据"
        }
        (Locale::En, LocalRepoRemovalDeletedCategory::DeveRuntimeMetadata) => {
            "Deve runtime metadata"
        }
        (Locale::Zh, LocalRepoRemovalDeletedCategory::DeveRuntimeMetadata) => "Deve 运行时元数据",
        (Locale::En, LocalRepoRemovalDeletedCategory::ProjectionLocator) => "Projection locator",
        (Locale::Zh, LocalRepoRemovalDeletedCategory::ProjectionLocator) => "Projection 定位记录",
        (Locale::En, LocalRepoRemovalDeletedCategory::HostAlias) => "Host-local alias",
        (Locale::Zh, LocalRepoRemovalDeletedCategory::HostAlias) => "本机显示名称",
        (Locale::En, LocalRepoRemovalDeletedCategory::RemoteImportCaptures) => {
            "Remote Import captures"
        }
        (Locale::Zh, LocalRepoRemovalDeletedCategory::RemoteImportCaptures) => {
            "Remote Import 捕获内容"
        }
        (Locale::En, LocalRepoRemovalDeletedCategory::CatalogMembership) => {
            "Local catalog membership"
        }
        (Locale::Zh, LocalRepoRemovalDeletedCategory::CatalogMembership) => "本地目录成员关系",
    }
}

pub fn preserved(locale: Locale, value: LocalRepoRemovalPreservedCategory) -> &'static str {
    match (locale, value) {
        (Locale::En, LocalRepoRemovalPreservedCategory::WorkspaceContent) => {
            "Markdown, attachments, and other workspace files"
        }
        (Locale::Zh, LocalRepoRemovalPreservedCategory::WorkspaceContent) => {
            "Markdown、附件与其它 Workspace 文件"
        }
        (Locale::En, LocalRepoRemovalPreservedCategory::GitMetadata) => ".git metadata",
        (Locale::Zh, LocalRepoRemovalPreservedCategory::GitMetadata) => ".git 元数据",
        (Locale::En, LocalRepoRemovalPreservedCategory::RemoteShadows) => "Remote shadows",
        (Locale::Zh, LocalRepoRemovalPreservedCategory::RemoteShadows) => "远端影子数据",
        (Locale::En, LocalRepoRemovalPreservedCategory::HostIdentityAndConfiguration) => {
            "Host identity and configuration"
        }
        (Locale::Zh, LocalRepoRemovalPreservedCategory::HostIdentityAndConfiguration) => {
            "宿主身份与配置"
        }
        (Locale::En, LocalRepoRemovalPreservedCategory::OperatorRecoveryInputs) => {
            "Operator recovery inputs"
        }
        (Locale::Zh, LocalRepoRemovalPreservedCategory::OperatorRecoveryInputs) => "运维恢复输入",
        (Locale::En, LocalRepoRemovalPreservedCategory::AuthorityLockIdentity) => {
            "Persistent authority lock identity"
        }
        (Locale::Zh, LocalRepoRemovalPreservedCategory::AuthorityLockIdentity) => {
            "持久 authority lock 标识"
        }
    }
}

pub fn warning(locale: Locale, value: LocalRepoRemovalWarning) -> &'static str {
    match (locale, value) {
        (Locale::En, LocalRepoRemovalWarning::LedgerHistoryHasNoSupportedRestore) => {
            "Deleted Ledger history has no supported restore path."
        }
        (Locale::Zh, LocalRepoRemovalWarning::LedgerHistoryHasNoSupportedRestore) => {
            "删除后的 Ledger 历史目前没有受支持的恢复路径。"
        }
        (Locale::En, LocalRepoRemovalWarning::NoFallbackSelected) => {
            "If this repository is active, the session will enter NoScope."
        }
        (Locale::Zh, LocalRepoRemovalWarning::NoFallbackSelected) => {
            "如果这是当前仓库，会话将进入 NoScope。"
        }
        (Locale::En, LocalRepoRemovalWarning::SelectedFallbackUnavailable) => {
            "The selected fallback is unavailable; the session will enter NoScope."
        }
        (Locale::Zh, LocalRepoRemovalWarning::SelectedFallbackUnavailable) => {
            "选定的回退仓库不可用；会话将进入 NoScope。"
        }
        (Locale::En, LocalRepoRemovalWarning::RemoteImportCaptureWillBeDiscarded) => {
            "Unapplied Remote Import captures will be discarded."
        }
        (Locale::Zh, LocalRepoRemovalWarning::RemoteImportCaptureWillBeDiscarded) => {
            "尚未应用的 Remote Import 捕获内容将被丢弃。"
        }
    }
}

pub fn blocker(locale: Locale, value: LocalRepoRemovalBlocker) -> &'static str {
    match locale {
        Locale::En => match value {
            LocalRepoRemovalBlocker::ProjectionFault => "Projection recovery is required.",
            LocalRepoRemovalBlocker::WorkspaceIngestionUnavailable => {
                "Workspace ingestion is unavailable."
            }
            LocalRepoRemovalBlocker::AuthorityBusy => "Repository authority is busy.",
            LocalRepoRemovalBlocker::RepositoryIdentityAmbiguous => {
                "Repository identity cannot be verified uniquely."
            }
            LocalRepoRemovalBlocker::WorkspaceIdentityUnverified => {
                "Workspace ownership cannot be verified."
            }
            LocalRepoRemovalBlocker::RecoveryInputOverlap => {
                "A recovery input overlaps managed state."
            }
            LocalRepoRemovalBlocker::RemoteImportApplyInFlight => {
                "Remote Import Apply is still running."
            }
            LocalRepoRemovalBlocker::RemoteImportProjectionPending => {
                "A Remote Import projection is still pending."
            }
            LocalRepoRemovalBlocker::RemoteImportProjectionDegraded => {
                "A Remote Import projection is degraded."
            }
            LocalRepoRemovalBlocker::RepairRequired => "Repository repair is required.",
        },
        Locale::Zh => match value {
            LocalRepoRemovalBlocker::ProjectionFault => "需要先完成 Projection 恢复。",
            LocalRepoRemovalBlocker::WorkspaceIngestionUnavailable => {
                "Workspace ingestion 不可用。"
            }
            LocalRepoRemovalBlocker::AuthorityBusy => "仓库权威状态正忙。",
            LocalRepoRemovalBlocker::RepositoryIdentityAmbiguous => "无法唯一验证仓库身份。",
            LocalRepoRemovalBlocker::WorkspaceIdentityUnverified => "无法验证 Workspace 所有权。",
            LocalRepoRemovalBlocker::RecoveryInputOverlap => "恢复输入与受管状态重叠。",
            LocalRepoRemovalBlocker::RemoteImportApplyInFlight => "Remote Import Apply 仍在执行。",
            LocalRepoRemovalBlocker::RemoteImportProjectionPending => {
                "Remote Import Projection 仍待完成。"
            }
            LocalRepoRemovalBlocker::RemoteImportProjectionDegraded => {
                "Remote Import Projection 已降级。"
            }
            LocalRepoRemovalBlocker::RepairRequired => "仓库需要修复。",
        },
    }
}
