use deve_core::protocol::{ServerError, ServerErrorCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceControlNotice {
    pub code: ServerErrorCode,
    pub detail: Option<String>,
}

impl SourceControlNotice {
    pub fn from_server_error(error: &ServerError) -> Option<Self> {
        is_source_control_error(error.code).then(|| Self {
            code: error.code,
            detail: error.detail.clone(),
        })
    }
}

pub const fn is_source_control_error(code: ServerErrorCode) -> bool {
    matches!(
        code,
        ServerErrorCode::ScRepoNotSelected
            | ServerErrorCode::ScRemoteBranchReadonly
            | ServerErrorCode::ScRepoContextInvalid
            | ServerErrorCode::ScPendingNotFound
            | ServerErrorCode::ScStagedNotFound
            | ServerErrorCode::ScDocNotFound
            | ServerErrorCode::ScCommitNotFound
            | ServerErrorCode::ScNothingToCommit
            | ServerErrorCode::ScConflictTargetMissing
    )
}

#[cfg(test)]
mod tests {
    use super::{SourceControlNotice, is_source_control_error};
    use deve_core::protocol::{ServerError, ServerErrorCode};

    #[test]
    fn sc_codes_are_classified_as_source_control_errors() {
        assert!(is_source_control_error(ServerErrorCode::ScNothingToCommit));
        assert!(is_source_control_error(
            ServerErrorCode::ScRemoteBranchReadonly
        ));
        assert!(!is_source_control_error(ServerErrorCode::RequestFailed));
    }

    #[test]
    fn notice_only_builds_for_sc_errors() {
        let sc_error = ServerError::with_detail(ServerErrorCode::ScDocNotFound, "missing doc");
        let sc_notice = SourceControlNotice::from_server_error(&sc_error).unwrap();
        assert_eq!(sc_notice.code, ServerErrorCode::ScDocNotFound);
        assert_eq!(sc_notice.detail.as_deref(), Some("missing doc"));

        let generic_error = ServerError::new(ServerErrorCode::RequestFailed);
        assert!(SourceControlNotice::from_server_error(&generic_error).is_none());
    }
}
