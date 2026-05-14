//! plan_ref:
//!   - 12_commands#command-palette-shortcuts
//!
use crate::components::command_palette::types::Command;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate_banner::cannot_send;
use crate::hooks::use_core::{BranchContext, CoreState, SyncMergeContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn merge_peer_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    branch: Option<BranchContext>,
    sync: Option<SyncMergeContext>,
    core: Option<CoreState>,
) -> Command {
    Command::available(
        "merge_peer",
        (t::command_palette::merge_peer)(locale),
        Callback::new(move |_| {
            if let (Some(branch), Some(sync)) = (branch.as_ref(), sync.as_ref()) {
                if let Some(peer_id) = branch.active_branch.get_untracked() {
                    sync.on_merge_peer.run(peer_id.to_string());
                } else if let Some(core) = core.as_ref() {
                    let message = cannot_send("MergePeer", "no active peer selected");
                    warn_sync_banner(core.set_sync_banner, message);
                }
            } else {
                set_show.set(false);
                return;
            }
            set_show.set(false);
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::merge_peer_command;
    use crate::hooks::use_core::{BranchContext, SyncMergeContext};
    use crate::i18n::Locale;
    use deve_core::models::PeerId;
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;
    use std::sync::{Arc, Mutex};

    #[test]
    fn merge_peer_command_uses_captured_contexts() {
        let owner = Owner::new();
        owner.with(|| {
            let (show, set_show) = signal(true);
            let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
            let (current_repo, set_current_repo) = signal(Some("default".to_string()));
            let (current_repo_id, set_current_repo_id) = signal(Some("repo-1".to_string()));
            let (shadow_repos, _) = signal(vec!["peer-a".to_string()]);
            let (repo_list, _) = signal(vec!["default".to_string()]);
            let (sync_mode, _) = signal("auto".to_string());
            let (pending_ops_count, _) = signal(0u32);
            let (pending_ops_previews, _) = signal(Vec::<(String, String, String)>::new());
            let calls = Arc::new(Mutex::new(Vec::<String>::new()));
            let calls_for_callback = calls.clone();
            let command = merge_peer_command(
                Locale::En,
                set_show,
                Some(BranchContext {
                    active_branch,
                    set_active_branch: signal(None::<PeerId>).1,
                    on_switch_branch: Callback::new(|_: Option<String>| {}),
                    current_repo,
                    set_current_repo,
                    current_repo_id,
                    set_current_repo_id,
                    on_switch_repo: Callback::new(|_: String| {}),
                    shadow_repos,
                    on_list_shadows: Callback::new(|_| {}),
                    repo_list,
                }),
                Some(SyncMergeContext {
                    sync_mode,
                    pending_ops_count,
                    pending_ops_previews,
                    on_get_sync_mode: Callback::new(|_| {}),
                    on_set_sync_mode: Callback::new(|_: String| {}),
                    on_get_pending_ops: Callback::new(|_| {}),
                    on_confirm_merge: Callback::new(|_| {}),
                    on_discard_pending: Callback::new(|_| {}),
                    on_merge_peer: Callback::new(move |peer_id: String| {
                        calls_for_callback.lock().unwrap().push(peer_id);
                    }),
                }),
                None,
            );

            command.action.run(());

            assert_eq!(calls.lock().unwrap().as_slice(), ["peer-a"]);
            assert!(!show.get_untracked());
        });
    }
}
