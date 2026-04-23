use crate::components::command_palette::types::Command;
use crate::hooks::use_core::write_gate_banner::cannot_send;
use crate::hooks::use_core::{BranchContext, CoreState, SyncMergeContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn merge_peer_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    Command {
        id: "merge_peer".to_string(),
        title: (t::command_palette::merge_peer)(locale).to_string(),
        action: Callback::new(move |_| {
            let branch = use_context::<BranchContext>().expect("branch ctx");
            let sync = use_context::<SyncMergeContext>().expect("sync ctx");
            if let Some(peer_id) = branch.active_branch.get_untracked() {
                sync.on_merge_peer.run(peer_id.to_string());
            } else {
                let core = use_context::<CoreState>().expect("core ctx");
                let message = cannot_send("MergePeer", "no active peer selected");
                leptos::logging::warn!("{}", message);
                core.set_sync_banner.set(Some(message));
            }
            set_show.set(false);
        }),
        is_file: false,
    }
}
