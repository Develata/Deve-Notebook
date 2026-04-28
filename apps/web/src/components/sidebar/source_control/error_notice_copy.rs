use crate::hooks::use_core::source_control_notice::{
    SourceControlNotice, deleted_no_doc_id_path, is_deleted_no_doc_id_notice,
};
use crate::i18n::{Locale, server_error, source_control as sc};

pub fn title(locale: Locale, notice: &SourceControlNotice) -> String {
    if is_deleted_no_doc_id_notice(notice) {
        return sc::diff_unavailable(locale).to_string();
    }
    server_error::message(locale, notice.code).to_string()
}

pub fn hint(locale: Locale, notice: &SourceControlNotice) -> String {
    match notice.code {
        deve_core::protocol::ServerErrorCode::ScDocNotFound
            if deleted_no_doc_id_path(notice).is_some() =>
        {
            let path = deleted_no_doc_id_path(notice).unwrap_or_default();
            sc::deleted_change_no_doc_diff(locale, &path)
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
