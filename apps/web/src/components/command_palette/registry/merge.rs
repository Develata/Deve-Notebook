//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!
use crate::components::command_palette::types::Command;
use crate::hooks::use_core::{BranchContext, SyncMergeContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn merge_peer_commands(
    locale: Locale,
    set_show: WriteSignal<bool>,
    branch: Option<BranchContext>,
    sync: Option<SyncMergeContext>,
) -> Vec<Command> {
    let Some(branch) = branch else {
        return vec![merge_peer_unavailable_command(
            locale,
            set_show,
            t::command_palette::merge_peer_context_unavailable_reason(locale),
        )];
    };
    let Some(sync) = sync else {
        return vec![merge_peer_unavailable_command(
            locale,
            set_show,
            t::command_palette::merge_peer_context_unavailable_reason(locale),
        )];
    };
    if branch.active_branch.get().is_some() {
        return Vec::new();
    }

    let peers = branch.shadow_repos.get();
    if peers.is_empty() {
        return vec![merge_peer_unavailable_command(
            locale,
            set_show,
            t::command_palette::merge_peer_no_source_reason(locale),
        )];
    }

    peers
        .into_iter()
        .map(|peer_id| merge_peer_source_command(locale, set_show, sync.clone(), peer_id))
        .collect()
}

fn merge_peer_source_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    sync: SyncMergeContext,
    peer_id: String,
) -> Command {
    let title = format!("{}: {}", (t::command_palette::merge_peer)(locale), peer_id);
    let command_id = format!("merge_peer_{}", peer_id);
    Command::available(command_id, title, move || {
        sync.on_merge_peer.run(peer_id.clone());
        set_show.set(false);
    })
    .with_group((t::command_palette::group_peer)(locale))
    .with_enabled_when((t::command_palette::enabled_peer_merge_source)(locale))
}

fn merge_peer_unavailable_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    reason: &'static str,
) -> Command {
    Command::unavailable(
        "merge_peer",
        (t::command_palette::merge_peer)(locale),
        reason,
        move || {
            set_show.set(false);
        },
    )
    .with_group((t::command_palette::group_peer)(locale))
    .with_enabled_when(reason)
}

#[cfg(test)]
mod tests;
