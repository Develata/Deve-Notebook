//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 08_auth#unauthorized-handling
//!
use deve_core::protocol::{ServerError, ServerErrorCode};

pub const DELETED_NO_DOC_ID_NOTICE_PREFIX: &str = "deleted-no-doc-id:";
pub const GIT_STATUS_CLI_NOTICE_DETAIL: &str = "git-status-cli-only";
pub const GIT_MIRROR_CLI_NOTICE_DETAIL: &str = "git-mirror-cli-only";
pub const GIT_EXPORT_CLI_NOTICE_DETAIL: &str = "git-export-cli-only";
pub const GIT_IMPORT_CLI_NOTICE_DETAIL: &str = "git-import-cli-only";
pub const GIT_PUSH_CLI_NOTICE_DETAIL: &str = "git-push-cli-only";
pub const GIT_REPAIR_CLI_NOTICE_DETAIL: &str = "git-repair-cli-only";
pub const ESTABLISH_BRANCH_UNAVAILABLE_DETAIL: &str = "establish-branch-unavailable";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceControlNotice {
    pub code: ServerErrorCode,
    pub detail: Option<String>,
}

impl SourceControlNotice {
    pub fn from_server_error(error: &ServerError) -> Option<Self> {
        is_source_control_error(error.code).then_some(Self {
            code: error.code,
            detail: None,
        })
    }

    pub fn git_import_cli_only() -> Self {
        Self {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: Some(GIT_IMPORT_CLI_NOTICE_DETAIL.to_string()),
        }
    }

    pub fn git_status_cli_only() -> Self {
        Self {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: Some(GIT_STATUS_CLI_NOTICE_DETAIL.to_string()),
        }
    }

    pub fn git_mirror_cli_only() -> Self {
        Self {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: Some(GIT_MIRROR_CLI_NOTICE_DETAIL.to_string()),
        }
    }

    pub fn git_export_cli_only() -> Self {
        Self {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: Some(GIT_EXPORT_CLI_NOTICE_DETAIL.to_string()),
        }
    }

    pub fn git_push_cli_only() -> Self {
        Self {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: Some(GIT_PUSH_CLI_NOTICE_DETAIL.to_string()),
        }
    }

    pub fn git_repair_cli_only() -> Self {
        Self {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: Some(GIT_REPAIR_CLI_NOTICE_DETAIL.to_string()),
        }
    }

    pub fn establish_branch_unavailable() -> Self {
        Self {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: Some(ESTABLISH_BRANCH_UNAVAILABLE_DETAIL.to_string()),
        }
    }
}

pub fn deleted_no_doc_id_path(notice: &SourceControlNotice) -> Option<&str> {
    notice
        .detail
        .as_deref()
        .and_then(|detail| detail.strip_prefix(DELETED_NO_DOC_ID_NOTICE_PREFIX))
}

pub fn is_deleted_no_doc_id_notice(notice: &SourceControlNotice) -> bool {
    deleted_no_doc_id_path(notice).is_some()
}

pub fn is_git_status_cli_notice(notice: &SourceControlNotice) -> bool {
    notice.detail.as_deref() == Some(GIT_STATUS_CLI_NOTICE_DETAIL)
}

pub fn is_git_mirror_cli_notice(notice: &SourceControlNotice) -> bool {
    notice.detail.as_deref() == Some(GIT_MIRROR_CLI_NOTICE_DETAIL)
}

pub fn is_git_export_cli_notice(notice: &SourceControlNotice) -> bool {
    notice.detail.as_deref() == Some(GIT_EXPORT_CLI_NOTICE_DETAIL)
}

pub fn is_git_import_cli_notice(notice: &SourceControlNotice) -> bool {
    notice.detail.as_deref() == Some(GIT_IMPORT_CLI_NOTICE_DETAIL)
}

pub fn is_git_push_cli_notice(notice: &SourceControlNotice) -> bool {
    notice.detail.as_deref() == Some(GIT_PUSH_CLI_NOTICE_DETAIL)
}

pub fn is_git_repair_cli_notice(notice: &SourceControlNotice) -> bool {
    notice.detail.as_deref() == Some(GIT_REPAIR_CLI_NOTICE_DETAIL)
}

pub fn is_establish_branch_unavailable_notice(notice: &SourceControlNotice) -> bool {
    notice.detail.as_deref() == Some(ESTABLISH_BRANCH_UNAVAILABLE_DETAIL)
}

pub fn is_local_command_notice(notice: &SourceControlNotice) -> bool {
    matches!(
        notice.detail.as_deref(),
        Some(GIT_STATUS_CLI_NOTICE_DETAIL)
            | Some(GIT_MIRROR_CLI_NOTICE_DETAIL)
            | Some(GIT_EXPORT_CLI_NOTICE_DETAIL)
            | Some(GIT_IMPORT_CLI_NOTICE_DETAIL)
            | Some(GIT_PUSH_CLI_NOTICE_DETAIL)
            | Some(GIT_REPAIR_CLI_NOTICE_DETAIL)
            | Some(ESTABLISH_BRANCH_UNAVAILABLE_DETAIL)
    )
}

pub const fn is_source_control_error(code: ServerErrorCode) -> bool {
    matches!(
        code,
        ServerErrorCode::ScRepoNotSelected
            | ServerErrorCode::ScRemoteBranchReadonly
            | ServerErrorCode::ScRepoContextInvalid
            | ServerErrorCode::ScStaleScope
            | ServerErrorCode::ScPendingNotFound
            | ServerErrorCode::ScStagedNotFound
            | ServerErrorCode::ScDocNotFound
            | ServerErrorCode::ScCommitNotFound
            | ServerErrorCode::ScCommitDiffUnprojectable
            | ServerErrorCode::ScNothingToCommit
            | ServerErrorCode::ScConflictTargetMissing
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DELETED_NO_DOC_ID_NOTICE_PREFIX, ESTABLISH_BRANCH_UNAVAILABLE_DETAIL,
        GIT_EXPORT_CLI_NOTICE_DETAIL, GIT_IMPORT_CLI_NOTICE_DETAIL, GIT_MIRROR_CLI_NOTICE_DETAIL,
        GIT_PUSH_CLI_NOTICE_DETAIL, GIT_REPAIR_CLI_NOTICE_DETAIL, GIT_STATUS_CLI_NOTICE_DETAIL,
        SourceControlNotice, deleted_no_doc_id_path, is_deleted_no_doc_id_notice,
        is_establish_branch_unavailable_notice, is_git_export_cli_notice, is_git_import_cli_notice,
        is_git_mirror_cli_notice, is_git_push_cli_notice, is_git_repair_cli_notice,
        is_git_status_cli_notice, is_local_command_notice, is_source_control_error,
    };
    use deve_core::protocol::{ServerError, ServerErrorCode};

    #[test]
    fn sc_codes_are_classified_as_source_control_errors() {
        for code in [
            ServerErrorCode::ScNothingToCommit,
            ServerErrorCode::ScRemoteBranchReadonly,
            ServerErrorCode::ScCommitDiffUnprojectable,
        ] {
            assert!(is_source_control_error(code));
        }
        assert!(!is_source_control_error(ServerErrorCode::RequestFailed));
    }

    #[test]
    fn notice_only_builds_for_sc_errors() {
        let sc_error = ServerError::with_detail(ServerErrorCode::ScDocNotFound, "missing doc");
        let sc_notice = SourceControlNotice::from_server_error(&sc_error).unwrap();
        assert_eq!(sc_notice.code, ServerErrorCode::ScDocNotFound);
        assert_eq!(sc_notice.detail, None);

        let generic_error = ServerError::new(ServerErrorCode::RequestFailed);
        assert!(SourceControlNotice::from_server_error(&generic_error).is_none());
    }

    #[test]
    fn deleted_docless_notice_is_detected() {
        let notice = SourceControlNotice {
            code: ServerErrorCode::ScDocNotFound,
            detail: Some(format!("{DELETED_NO_DOC_ID_NOTICE_PREFIX}deleted.md")),
        };
        assert!(is_deleted_no_doc_id_notice(&notice));
        assert_eq!(deleted_no_doc_id_path(&notice), Some("deleted.md"));
    }

    #[test]
    fn local_command_notices_are_detected() {
        for (notice, detail, classifier) in [
            (
                SourceControlNotice::git_status_cli_only(),
                GIT_STATUS_CLI_NOTICE_DETAIL,
                is_git_status_cli_notice as fn(&SourceControlNotice) -> bool,
            ),
            (
                SourceControlNotice::git_mirror_cli_only(),
                GIT_MIRROR_CLI_NOTICE_DETAIL,
                is_git_mirror_cli_notice,
            ),
            (
                SourceControlNotice::git_export_cli_only(),
                GIT_EXPORT_CLI_NOTICE_DETAIL,
                is_git_export_cli_notice,
            ),
            (
                SourceControlNotice::git_import_cli_only(),
                GIT_IMPORT_CLI_NOTICE_DETAIL,
                is_git_import_cli_notice,
            ),
            (
                SourceControlNotice::git_push_cli_only(),
                GIT_PUSH_CLI_NOTICE_DETAIL,
                is_git_push_cli_notice,
            ),
            (
                SourceControlNotice::git_repair_cli_only(),
                GIT_REPAIR_CLI_NOTICE_DETAIL,
                is_git_repair_cli_notice,
            ),
            (
                SourceControlNotice::establish_branch_unavailable(),
                ESTABLISH_BRANCH_UNAVAILABLE_DETAIL,
                is_establish_branch_unavailable_notice,
            ),
        ] {
            assert_eq!(notice.detail.as_deref(), Some(detail));
            assert!(classifier(&notice));
            assert!(is_local_command_notice(&notice));
        }

        let server_notice = SourceControlNotice {
            code: ServerErrorCode::ScDocNotFound,
            detail: None,
        };
        assert!(!is_local_command_notice(&server_notice));
    }
}
