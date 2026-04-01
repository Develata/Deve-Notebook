use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, server_error};
use leptos::prelude::*;

const DELETED_NO_DOC_ID_NOTICE_PREFIX: &str = "deleted-no-doc-id:";

fn deleted_no_doc_id_path(notice: &SourceControlNotice) -> Option<&str> {
    notice
        .detail
        .as_deref()
        .and_then(|detail| detail.strip_prefix(DELETED_NO_DOC_ID_NOTICE_PREFIX))
}

fn title(locale: Locale, notice: &SourceControlNotice) -> String {
    if deleted_no_doc_id_path(notice).is_some() {
        return match locale {
            Locale::En => "Diff unavailable".to_string(),
            Locale::Zh => "无法显示差异".to_string(),
        };
    }
    server_error::message(locale, notice.code).to_string()
}

fn hint(locale: Locale, notice: &SourceControlNotice) -> String {
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

#[component]
pub fn ErrorNotice(
    notice: ReadSignal<Option<SourceControlNotice>>,
    block: Signal<Option<RepoWriteBlock>>,
    clear_notice: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <Show when=move || block.get().is_none() && notice.get().is_some()>
            <div class="px-4 py-3 text-sm border-b border-default bg-warning/5">
                <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                        <p class="text-primary font-medium">
                            {move || {
                                notice
                                    .get()
                                    .map(|current| title(locale.get(), &current))
                                    .unwrap_or_default()
                            }}
                        </p>
                        <p class="mt-1 text-xs text-muted">
                            {move || notice.get().map(|current| hint(locale.get(), &current)).unwrap_or_default()}
                        </p>
                    </div>
                    <button
                        class="text-xs text-secondary hover:text-primary"
                        on:click=move |_| clear_notice.run(())
                    >
                        {"×"}
                    </button>
                </div>
            </div>
        </Show>
    }
}
