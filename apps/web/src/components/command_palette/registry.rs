//! 命令面板的静态命令定义。

use leptos::prelude::*;
use crate::i18n::{Locale, t};
use super::types::Command;

/// 创建静态命令列表。
pub fn create_static_commands(
    locale: Locale,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
    locale_signal: RwSignal<Locale>,
) -> Vec<Command> {
    vec![
        // 打开文档命令 - 打开文档模态框
        Command {
            id: "open".to_string(), 
            title: if locale == Locale::Zh { "打开文档".to_string() } else { "Open Document".to_string() },
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    on_open.run(());
                    // Do not close, as on_open re-purposes the search box
                });
            }),
            is_file: false,
        },
        Command {
            id: "settings".to_string(), 
            title: (t::command_palette::open_settings)(locale).to_string(),
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    on_settings.run(());
                    set_show.set(false);
                });
            }),
            is_file: false,
        },
        Command {
            id: "lang".to_string(),
            title: (t::command_palette::toggle_language)(locale).to_string(),
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    locale_signal.update(|l| *l = l.toggle());
                    set_show.set(false);
                });
            }),
            is_file: false,
        },
        // P2P: Switch to Peer
        Command {
            id: "switch_peer".to_string(), 
            title: if locale == Locale::Zh { "P2P: 切换到节点 (Switch to Peer)".to_string() } else { "P2P: Switch to Peer".to_string() },
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    let search_control = use_context::<crate::app::SearchControl>().expect("search control");
                    search_control.set_mode.set("@".to_string());
                    search_control.set_show.set(true);
                    // Close this command palette (if it's a separate overlay, but Unified Search usually replaces it)
                    // Wait, if Unified Search IS the command palette, we just change mode.
                    // But here CommandPalette is a different component?
                    // Yes, `mod.rs` shows it's a separate `CommandPalette` component.
                    // So we close it.
                    set_show.set(false);
                });
            }),
            is_file: false,
        },
        // P2P: Establish Branch (Placeholder)
        Command {
            id: "establish_branch".to_string(), 
            title: if locale == Locale::Zh { "P2P: 建立分支 (Establish Branch)".to_string() } else { "P2P: Establish Branch".to_string() },
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                     // For now, reuse the same logic as Switch to Peer, as selecting a peer is the first step.
                     // Ideally this would open a dialog or automatically clone.
                    let search_control = use_context::<crate::app::SearchControl>().expect("search control");
                    search_control.set_mode.set("@".to_string());
                    search_control.set_show.set(true);
                    set_show.set(false);
                });
            }),
            is_file: false,
        },
        // P2P: Merge Peer
        Command {
            id: "merge_peer".to_string(),
            title: if locale == Locale::Zh { "P2P: 合并当前节点 (Merge Peer)".to_string() } else { "P2P: Merge Peer".to_string() },
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                     // Get CoreState to check active repo
                     let core = use_context::<crate::hooks::use_core::CoreState>().expect("core state");
                     if let Some(peer_id) = core.active_repo.get_untracked() {
                         core.on_merge_peer.run(peer_id.to_string());
                         // Notify user
                         // We don't have a toast system yet, but console log happens.
                         // Ideally we close the palette.
                         set_show.set(false);
                     } else {
                         // TODO: Show Toast "Please switch to a peer first"
                         leptos::logging::warn!("Cannot merge: No active peer selected.");
                         set_show.set(false);
                     }
                });
            }),
            is_file: false,
        }
    ]
}

/// 基于查询字符串筛选命令。
pub fn filter_commands(
    query: &str,
    commands: Vec<Command>,
    max_results: usize,
) -> Vec<Command> {
    let q = query.to_lowercase();
    let mut results = Vec::new();
    
    for cmd in commands {
        if q.is_empty() || cmd.title.to_lowercase().contains(&q) || cmd.id.contains(&q) {
            results.push(cmd);
        }
    }
    
    if results.len() > max_results {
        results.truncate(max_results);
    }
    
    results
}
