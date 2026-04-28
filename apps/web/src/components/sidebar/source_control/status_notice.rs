//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
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
        RepoWriteBlock::ScopeSwitching => sc::scope_switching(locale).to_string(),
        RepoWriteBlock::NoRepo => sc::no_repo_selected(locale).to_string(),
    }
}

pub(crate) fn blocked_hint(locale: Locale, block: RepoWriteBlock) -> &'static str {
    match block {
        RepoWriteBlock::SessionExpired => sc::session_expired_hint(locale),
        RepoWriteBlock::Offline => sc::offline_hint(locale),
        RepoWriteBlock::Reconnecting => sc::reconnecting_hint(locale),
        RepoWriteBlock::SnapshotLoading => sc::snapshot_loading_hint(locale),
        RepoWriteBlock::ReadOnly => sc::remote_branch_readonly_hint(locale),
        RepoWriteBlock::ScopeSwitching => sc::scope_switching_hint(locale),
        RepoWriteBlock::NoRepo => sc::no_repo_hint(locale),
        RepoWriteBlock::HandshakingRepo => sc::handshaking_repo_hint(locale),
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
