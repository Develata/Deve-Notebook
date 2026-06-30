// apps/web/src/components/dashboard/actions_card.rs
//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! # Actions Card (快捷操作卡片)
//!
//! 提供 "New Doc" 和 "Sync Now" 按钮。

use crate::hooks::use_core::doc_name::next_untitled_doc_name;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_tracked};
use crate::hooks::use_core::write_gate_banner::cannot_create_document;
use crate::hooks::use_core::{DocContext, EditorContext, SyncMergeContext};
use crate::i18n::{Locale, t};
use crate::runtime::session_client::SessionClient;
use leptos::prelude::*;

pub(crate) fn dashboard_quick_action_touch_target_marker() -> &'static str {
    "dashboard_quick_actions"
}

fn dashboard_primary_action_button_class() -> &'static str {
    "flex h-11 flex-1 items-center justify-center rounded-md bg-accent px-3 py-2 text-xs font-medium text-on-accent transition-colors hover:bg-accent/90 disabled:cursor-not-allowed disabled:opacity-50"
}

fn dashboard_secondary_action_button_class() -> &'static str {
    "flex h-11 flex-1 items-center justify-center rounded-md border border-default px-3 py-2 text-xs font-medium text-primary transition-colors hover:bg-active"
}

#[component]
pub fn ActionsCard() -> impl IntoView {
    let document = expect_context::<DocContext>();
    let editor = expect_context::<EditorContext>();
    let session = expect_context::<SessionClient>();
    let sync = expect_context::<SyncMergeContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let document_for_create = document.clone();
    let editor_for_create = editor.clone();
    let session_for_create = session.clone();
    let editor_for_disabled = editor.clone();
    let session_for_disabled = session.clone();

    let on_new_doc = move |_| {
        if let Some(reason) = create_block_reason(&session_for_create, &editor_for_create) {
            let message = cannot_create_document(reason);
            warn_sync_banner(session_for_create.set_sync_banner, message);
            return;
        }
        let name = next_untitled_doc_name(
            document_for_create
                .docs
                .get_untracked()
                .iter()
                .map(|(_, path)| path.as_str()),
        );
        document_for_create.on_doc_create.run(name);
    };

    let on_sync = move |_| {
        sync.on_get_sync_mode.run(());
    };

    view! {
        <div
            class="bg-panel rounded-lg border border-default p-4"
            data-deve-dashboard-card="quick-actions"
        >
            <h3 class="text-sm font-semibold text-secondary mb-3">{move || t::dashboard::quick_actions(locale.get())}</h3>
            <div class="flex gap-2">
                <button
                    data-deve-mobile-touch-target=dashboard_quick_action_touch_target_marker()
                    class=dashboard_primary_action_button_class()
                    disabled=move || create_block_reason(&session_for_disabled, &editor_for_disabled).is_some()
                    on:click=on_new_doc
                >
                    {move || t::dashboard::new_doc(locale.get())}
                </button>
                <button
                    data-deve-mobile-touch-target=dashboard_quick_action_touch_target_marker()
                    class=dashboard_secondary_action_button_class()
                    on:click=on_sync
                >
                    {move || t::dashboard::sync_now(locale.get())}
                </button>
            </div>
        </div>
    }
}

fn create_block_reason(session: &SessionClient, editor: &EditorContext) -> Option<&'static str> {
    repo_write_block_tracked(
        &session.ws,
        RepoWriteSignals {
            load_state: editor.load_state,
            is_spectator: editor.is_spectator,
            handshake_ready: editor.handshake_ready,
            current_repo_id: editor.current_repo_id,
            current_scope_nonce: editor.current_scope_nonce,
            active_branch: editor.active_branch,
            pending_branch_switch: editor.pending_branch_switch,
            pending_repo_switch: editor.pending_repo_switch,
        },
    )
    .map(|block| block.label())
}

#[cfg(test)]
mod tests {
    use super::{
        dashboard_primary_action_button_class, dashboard_quick_action_touch_target_marker,
        dashboard_secondary_action_button_class,
    };

    #[test]
    fn mobile_touch_targets_dashboard_quick_actions_are_at_least_44px() {
        assert_eq!(
            dashboard_quick_action_touch_target_marker(),
            "dashboard_quick_actions"
        );
        assert!(dashboard_primary_action_button_class().contains("h-11"));
        assert!(dashboard_secondary_action_button_class().contains("h-11"));
        assert!(
            dashboard_primary_action_button_class().contains("items-center")
                && dashboard_primary_action_button_class().contains("justify-center")
        );
        assert!(
            dashboard_secondary_action_button_class().contains("items-center")
                && dashboard_secondary_action_button_class().contains("justify-center")
        );
    }
}
