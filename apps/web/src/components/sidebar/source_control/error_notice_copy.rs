//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 18_release#runtime-observability
//!
use crate::hooks::use_core::source_control_notice::{
    REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL, SourceControlNotice, deleted_no_doc_id_path,
    is_deleted_no_doc_id_notice, is_establish_branch_unavailable_notice, is_git_export_cli_notice,
    is_git_import_cli_notice, is_git_mirror_cli_notice, is_git_push_cli_notice,
    is_git_repair_cli_notice, is_git_status_cli_notice,
    is_remote_projection_provider_io_pending_notice,
    is_remote_projection_session_unavailable_notice,
};
use crate::i18n::{Locale, server_error, source_control as sc};

pub fn title(locale: Locale, notice: &SourceControlNotice) -> String {
    if is_deleted_no_doc_id_notice(notice) {
        return sc::diff_unavailable(locale).to_string();
    }
    if is_git_status_cli_notice(notice) {
        return sc::git_status_cli_only_title(locale).to_string();
    }
    if is_git_mirror_cli_notice(notice) {
        return sc::git_mirror_cli_only_title(locale).to_string();
    }
    if is_git_export_cli_notice(notice) {
        return sc::git_export_cli_only_title(locale).to_string();
    }
    if is_git_import_cli_notice(notice) {
        return sc::git_import_cli_only_title(locale).to_string();
    }
    if is_git_push_cli_notice(notice) {
        return sc::git_push_cli_only_title(locale).to_string();
    }
    if is_git_repair_cli_notice(notice) {
        return sc::git_repair_cli_only_title(locale).to_string();
    }
    if is_establish_branch_unavailable_notice(notice) {
        return sc::establish_branch_unavailable_title(locale).to_string();
    }
    if is_remote_projection_provider_io_pending_notice(notice) {
        return sc::remote_projection_provider_io_pending_title(locale).to_string();
    }
    if is_remote_projection_session_unavailable_notice(notice) {
        return sc::remote_projection_session_unavailable_title(locale).to_string();
    }
    server_error::message(locale, notice.code).to_string()
}

pub fn hint(locale: Locale, notice: &SourceControlNotice) -> String {
    if is_remote_projection_provider_io_pending_notice(notice) {
        return sc::remote_projection_provider_io_pending_hint(locale).to_string();
    }
    if is_remote_projection_session_unavailable_notice(notice) {
        return sc::remote_projection_session_unavailable_hint(locale).to_string();
    }
    if is_establish_branch_unavailable_notice(notice) {
        return sc::establish_branch_unavailable_hint(locale).to_string();
    }
    if is_git_status_cli_notice(notice) {
        return sc::git_status_cli_only_hint(locale).to_string();
    }
    if is_git_mirror_cli_notice(notice) {
        return sc::git_mirror_cli_only_hint(locale).to_string();
    }
    if is_git_export_cli_notice(notice) {
        return sc::git_export_cli_only_hint(locale).to_string();
    }
    if is_git_import_cli_notice(notice) {
        return sc::git_import_cli_only_hint(locale).to_string();
    }
    if is_git_push_cli_notice(notice) {
        return sc::git_push_cli_only_hint(locale).to_string();
    }
    if is_git_repair_cli_notice(notice) {
        return sc::git_repair_cli_only_hint(locale).to_string();
    }
    match notice.code {
        deve_core::protocol::ServerErrorCode::ScDocNotFound
            if deleted_no_doc_id_path(notice).is_some() =>
        {
            let path = deleted_no_doc_id_path(notice).unwrap_or_default();
            sc::deleted_change_no_doc_diff(locale, path)
        }
        deve_core::protocol::ServerErrorCode::ScCommitDiffUnprojectable => {
            sc::legacy_commit_unprojectable(locale, None)
        }
        deve_core::protocol::ServerErrorCode::ScNothingToCommit => {
            sc::stage_files_before_commit(locale).to_string()
        }
        deve_core::protocol::ServerErrorCode::ScPendingNotFound
        | deve_core::protocol::ServerErrorCode::ScStagedNotFound
        | deve_core::protocol::ServerErrorCode::ScConflictTargetMissing => {
            sc::refresh_change_list(locale).to_string()
        }
        deve_core::protocol::ServerErrorCode::ScDocNotFound
        | deve_core::protocol::ServerErrorCode::ScCommitNotFound => {
            sc::selected_item_unavailable(locale).to_string()
        }
        _ => server_error::message(locale, notice.code).to_string(),
    }
}

pub fn details(locale: Locale, notice: &SourceControlNotice) -> Vec<String> {
    if is_remote_projection_provider_io_pending_notice(notice) {
        return remote_projection_provider_io_detail(notice)
            .into_iter()
            .collect();
    }
    if is_git_push_cli_notice(notice) {
        return sc::git_push_cli_only_details(locale)
            .into_iter()
            .map(str::to_string)
            .collect();
    }
    if is_git_repair_cli_notice(notice) {
        return sc::git_repair_cli_only_details(locale)
            .into_iter()
            .map(str::to_string)
            .collect();
    }
    Vec::new()
}

fn remote_projection_provider_io_detail(notice: &SourceControlNotice) -> Option<String> {
    let detail = notice.detail.as_deref()?;
    let suffix = detail
        .strip_prefix(REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL)?
        .trim_start_matches([';', ' '])
        .trim();
    (!suffix.is_empty()).then(|| suffix.to_string())
}

#[cfg(test)]
mod tests {
    use super::{details, hint, title};
    use crate::hooks::use_core::source_control_notice::SourceControlNotice;
    use crate::i18n::{Locale, source_control as sc};

    #[test]
    fn local_git_status_mirror_export_notices_use_cli_copy() {
        let status = SourceControlNotice::git_status_cli_only();
        assert_eq!(
            title(Locale::En, &status),
            sc::git_status_cli_only_title(Locale::En)
        );
        assert!(hint(Locale::En, &status).contains("deve_cli ngit status"));

        let mirror = SourceControlNotice::git_mirror_cli_only();
        assert_eq!(
            title(Locale::Zh, &mirror),
            sc::git_mirror_cli_only_title(Locale::Zh)
        );
        assert!(hint(Locale::Zh, &mirror).contains("deve_cli ngit mirror"));

        let export = SourceControlNotice::git_export_cli_only();
        assert_eq!(
            title(Locale::En, &export),
            sc::git_export_cli_only_title(Locale::En)
        );
        assert!(hint(Locale::En, &export).contains("deve_cli ngit export"));
    }

    #[test]
    fn local_git_import_notice_uses_cli_copy() {
        let notice = SourceControlNotice::git_import_cli_only();

        assert_eq!(
            title(Locale::En, &notice),
            sc::git_import_cli_only_title(Locale::En)
        );
        assert!(hint(Locale::En, &notice).contains("deve_cli ngit import --apply"));
    }

    #[test]
    fn local_git_push_notice_uses_cli_copy() {
        let notice = SourceControlNotice::git_push_cli_only();

        assert_eq!(
            title(Locale::Zh, &notice),
            sc::git_push_cli_only_title(Locale::Zh)
        );
        assert!(hint(Locale::Zh, &notice).contains("--remote <remote> --branch <branch>"));
        assert!(
            details(Locale::Zh, &notice)
                .iter()
                .any(|line| line.contains("deve_cli ngit export --repo <repo>"))
        );
    }

    #[test]
    fn local_git_repair_notice_uses_cli_copy() {
        let notice = SourceControlNotice::git_repair_cli_only();

        assert_eq!(
            title(Locale::En, &notice),
            sc::git_repair_cli_only_title(Locale::En)
        );
        assert!(hint(Locale::En, &notice).contains("repair_action[...]"));
        assert!(
            details(Locale::En, &notice)
                .iter()
                .any(|line| line.contains("retry-out-of-sync"))
        );
    }

    #[test]
    fn establish_branch_notice_uses_unavailable_copy() {
        let notice = SourceControlNotice::establish_branch_unavailable();

        assert_eq!(
            title(Locale::En, &notice),
            sc::establish_branch_unavailable_title(Locale::En)
        );
        assert!(hint(Locale::Zh, &notice).contains("P2P: Merge Peer"));
    }

    #[test]
    fn remote_projection_notice_uses_provider_io_copy() {
        let notice = SourceControlNotice::remote_projection_provider_io_pending();

        assert_eq!(
            title(Locale::En, &notice),
            sc::remote_projection_provider_io_pending_title(Locale::En)
        );
        assert!(hint(Locale::En, &notice).contains("provider_io_ready=false"));
        assert!(hint(Locale::Zh, &notice).contains("External Changes"));
    }

    #[test]
    fn remote_projection_notice_details_surface_backend_reason() {
        let notice = SourceControlNotice {
            code: deve_core::protocol::ServerErrorCode::ScRepoContextInvalid,
            detail: Some(format!(
                "{}; current repo_url does not match selected provider",
                crate::hooks::use_core::source_control_notice::REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL
            )),
        };

        assert!(
            details(Locale::En, &notice)
                .iter()
                .any(|line| line.contains("repo_url does not match"))
        );
    }

    #[test]
    fn remote_projection_session_notice_uses_backend_session_copy() {
        let notice = SourceControlNotice::remote_projection_session_unavailable();

        assert_eq!(
            title(Locale::En, &notice),
            sc::remote_projection_session_unavailable_title(Locale::En)
        );
        assert!(hint(Locale::En, &notice).contains("no active backend session"));
        assert!(hint(Locale::Zh, &notice).contains("External Changes"));
    }
}
