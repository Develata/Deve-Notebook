use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, bottom_bar, source_control as sc};
use leptos::prelude::*;

pub(crate) fn blocked_title(locale: Locale, block: RepoWriteBlock) -> String {
    match block {
        RepoWriteBlock::SessionExpired => bottom_bar::unauthorized(locale).to_string(),
        RepoWriteBlock::Offline => bottom_bar::offline(locale).to_string(),
        RepoWriteBlock::Reconnecting => bottom_bar::reconnecting(locale).to_string(),
        RepoWriteBlock::SnapshotLoading => bottom_bar::snapshot_loading(locale).to_string(),
        RepoWriteBlock::ReadOnly => bottom_bar::read_only(locale).to_string(),
        RepoWriteBlock::HandshakingRepo => bottom_bar::handshaking_repo(locale).to_string(),
        RepoWriteBlock::ScopeSwitching => match locale {
            Locale::En => "Switching scope...".to_string(),
            Locale::Zh => "切换作用域中...".to_string(),
        },
        RepoWriteBlock::NoRepo => sc::no_repo_selected(locale).to_string(),
    }
}

pub(crate) fn blocked_hint(locale: Locale, block: RepoWriteBlock) -> &'static str {
    match block {
        RepoWriteBlock::SessionExpired => match locale {
            Locale::En => "Sign in again before staging, discarding, or committing changes.",
            Locale::Zh => "请重新登录后再暂存、放弃或提交更改。",
        },
        RepoWriteBlock::Offline => match locale {
            Locale::En => {
                "Wait for the connection to recover before changing Source Control state."
            }
            Locale::Zh => "请等待连接恢复后再修改源代码管理状态。",
        },
        RepoWriteBlock::Reconnecting => match locale {
            Locale::En => {
                "The client is reconnecting. Source Control actions will resume automatically."
            }
            Locale::Zh => "客户端正在重连，源代码管理操作会在恢复后自动可用。",
        },
        RepoWriteBlock::SnapshotLoading => match locale {
            Locale::En => "Wait for the current repo snapshot to finish loading.",
            Locale::Zh => "请等待当前仓库快照加载完成。",
        },
        RepoWriteBlock::ReadOnly => sc::remote_branch_readonly_hint(locale),
        RepoWriteBlock::ScopeSwitching => match locale {
            Locale::En => "Wait for the repo or branch switch to finish before editing changes.",
            Locale::Zh => "请等待仓库或分支切换完成后再修改更改列表。",
        },
        RepoWriteBlock::NoRepo => match locale {
            Locale::En => "Select an active repo before using Source Control actions.",
            Locale::Zh => "请先选择激活仓库，再使用源代码管理操作。",
        },
        RepoWriteBlock::HandshakingRepo => match locale {
            Locale::En => "This repo is still negotiating writer access. Try again in a moment.",
            Locale::Zh => "当前仓库仍在协商写入权限，请稍后再试。",
        },
    }
}

#[component]
pub fn StatusNotice(block: Signal<Option<RepoWriteBlock>>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <Show when=move || block.get().is_some()>
            <div class="px-4 py-3 text-sm border-b border-default bg-panel">
                <p class="text-primary font-medium">
                    {move || block.get().map(|current| blocked_title(locale.get(), current)).unwrap_or_default()}
                </p>
                <p class="mt-1 text-xs text-muted">
                    {move || block.get().map(|current| blocked_hint(locale.get(), current)).unwrap_or_default()}
                </p>
            </div>
        </Show>
    }
}
