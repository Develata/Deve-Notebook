use crate::hooks::use_core::source_control_notice::{
    SourceControlNotice, deleted_no_doc_id_path, is_deleted_no_doc_id_notice,
};
use crate::i18n::{Locale, server_error};

pub fn title(locale: Locale, notice: &SourceControlNotice) -> String {
    if is_deleted_no_doc_id_notice(notice) {
        return match locale {
            Locale::En => "Diff unavailable".to_string(),
            Locale::Zh => "无法显示差异".to_string(),
        };
    }
    server_error::message(locale, notice.code).to_string()
}

pub fn hint(locale: Locale, notice: &SourceControlNotice) -> String {
    match notice.code {
        deve_core::protocol::ServerErrorCode::ScDocNotFound
            if deleted_no_doc_id_path(notice).is_some() =>
        {
            let path = deleted_no_doc_id_path(notice).unwrap_or_default();
            match locale {
                Locale::En => format!(
                    "No diff is available for deleted change {path} because it has no document identity."
                ),
                Locale::Zh => {
                    format!("删除变更 {path} 没有文档身份，因此当前无法生成可显示的差异。")
                }
            }
        }
        deve_core::protocol::ServerErrorCode::ScCommitDiffUnprojectable => {
            let commit = notice
                .detail
                .as_deref()
                .map(|detail| detail.chars().take(7).collect::<String>());
            match (locale, commit.as_deref()) {
                (Locale::En, Some(commit)) => format!(
                    "Commit {commit} contains legacy content without structure projection, so Deve-Note cannot reconstruct a path-safe diff."
                ),
                (Locale::Zh, Some(commit)) => format!(
                    "提交 {commit} 包含缺少结构投影的旧内容，Deve-Note 无法安全重建带路径语义的差异。"
                ),
                (Locale::En, None) => {
                    "This legacy commit contains content without structure projection, so Deve-Note cannot reconstruct a path-safe diff.".to_string()
                }
                (Locale::Zh, None) => {
                    "该旧提交包含缺少结构投影的内容，Deve-Note 无法安全重建带路径语义的差异。".to_string()
                }
            }
        }
        _ if notice.detail.is_some() => notice.detail.clone().unwrap_or_default(),
        deve_core::protocol::ServerErrorCode::ScNothingToCommit => match locale {
            Locale::En => "Stage files before trying to commit.",
            Locale::Zh => "请先暂存文件，再执行提交。",
        }
        .to_string(),
        deve_core::protocol::ServerErrorCode::ScPendingNotFound
        | deve_core::protocol::ServerErrorCode::ScStagedNotFound
        | deve_core::protocol::ServerErrorCode::ScConflictTargetMissing => match locale {
            Locale::En => "Refresh the change list and try again.",
            Locale::Zh => "请刷新更改列表后再试。",
        }
        .to_string(),
        deve_core::protocol::ServerErrorCode::ScDocNotFound
        | deve_core::protocol::ServerErrorCode::ScCommitNotFound => match locale {
            Locale::En => "The selected Source Control item is no longer available.",
            Locale::Zh => "当前选中的源代码管理条目已不存在。",
        }
        .to_string(),
        _ => server_error::message(locale, notice.code).to_string(),
    }
}
