//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
use crate::hooks::use_core::source_control_notice::{
    SourceControlNotice, deleted_no_doc_id_path, is_deleted_no_doc_id_notice,
    is_git_import_cli_notice, is_git_push_cli_notice, is_git_repair_cli_notice,
};
use crate::i18n::{Locale, server_error, source_control as sc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepairReviewCopy {
    pub title: String,
    pub rows: Vec<GitRepairReviewRow>,
    pub retry_command: String,
    pub authority_note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepairReviewRow {
    pub label: String,
    pub value: String,
}

pub fn title(locale: Locale, notice: &SourceControlNotice) -> String {
    if is_deleted_no_doc_id_notice(notice) {
        return sc::diff_unavailable(locale).to_string();
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
    server_error::message(locale, notice.code).to_string()
}

pub fn hint(locale: Locale, notice: &SourceControlNotice) -> String {
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
            let commit = notice
                .detail
                .as_deref()
                .map(|detail| detail.chars().take(7).collect::<String>());
            sc::legacy_commit_unprojectable(locale, commit.as_deref())
        }
        _ if notice.detail.is_some() => notice.detail.clone().unwrap_or_default(),
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

pub fn git_repair_review(
    locale: Locale,
    notice: &SourceControlNotice,
) -> Option<GitRepairReviewCopy> {
    is_git_repair_cli_notice(notice).then(|| GitRepairReviewCopy {
        title: sc::git_repair_review_title(locale).to_string(),
        rows: vec![
            GitRepairReviewRow {
                label: sc::git_repair_action_label(locale).to_string(),
                value: sc::git_repair_action_value(locale).to_string(),
            },
            GitRepairReviewRow {
                label: sc::git_repair_guidance_label(locale).to_string(),
                value: sc::git_repair_guidance_value(locale).to_string(),
            },
            GitRepairReviewRow {
                label: sc::git_repair_subject_label(locale).to_string(),
                value: sc::git_repair_subject_value(locale).to_string(),
            },
            GitRepairReviewRow {
                label: sc::git_repair_next_step_label(locale).to_string(),
                value: sc::git_repair_next_step_value(locale).to_string(),
            },
        ],
        retry_command: sc::git_repair_retry_command().to_string(),
        authority_note: sc::git_repair_authority_note(locale).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{details, git_repair_review, hint, title};
    use crate::hooks::use_core::source_control_notice::SourceControlNotice;
    use crate::i18n::{Locale, source_control as sc};

    #[test]
    fn local_git_import_notice_uses_cli_copy() {
        let notice = SourceControlNotice::git_import_cli_only();

        assert_eq!(
            title(Locale::En, &notice),
            sc::git_import_cli_only_title(Locale::En)
        );
        assert!(hint(Locale::En, &notice).contains("deve_cli git import --apply"));
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
                .any(|line| line.contains("deve_cli git export --repo <repo>"))
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

        let review = git_repair_review(Locale::En, &notice).expect("repair review");
        assert_eq!(review.title, sc::git_repair_review_title(Locale::En));
        assert!(
            review
                .rows
                .iter()
                .any(|row| row.value.contains("manual_only=yes"))
        );
        assert_eq!(
            review.retry_command,
            "deve_cli git export --repo <repo> --retry-out-of-sync"
        );
        assert!(review.authority_note.contains("read-only"));
    }
}
