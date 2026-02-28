// apps\web\src\components\command_palette
//! 命令面板的静态命令定义。

use super::types::Command;
use crate::components::main_layout::ChatControl;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

/// 创建静态命令列表。
pub fn create_static_commands(
    locale: Locale,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
    locale_signal: RwSignal<Locale>,
) -> Vec<Command> {
    // Try to get ChatControl from context at creation time
    let chat_control = use_context::<ChatControl>();

    let mut commands = vec![
        // 打开文档命令 - 打开文档模态框
        Command {
            id: "open".to_string(),
            title: (t::command_palette::open_document)(locale).to_string(),
            action: Callback::new(move |_| {
                on_open.run(());
                // Do not close, as on_open re-purposes the search box
            }),
            is_file: false,
        },
        Command {
            id: "settings".to_string(),
            title: (t::command_palette::open_settings)(locale).to_string(),
            action: Callback::new(move |_| {
                on_settings.run(());
                set_show.set(false);
            }),
            is_file: false,
        },
        Command {
            id: "lang".to_string(),
            title: (t::command_palette::toggle_language)(locale).to_string(),
            action: Callback::new(move |_| {
                locale_signal.update(|l| *l = l.toggle());
                set_show.set(false);
            }),
            is_file: false,
        },
        // P2P: Switch to Peer
        Command {
            id: "switch_peer".to_string(),
            title: (t::command_palette::switch_peer)(locale).to_string(),
            action: Callback::new(move |_| {
                let search_control = use_context::<crate::components::main_layout::SearchControl>()
                    .expect("search control");
                search_control.set_mode.set("@".to_string());
                search_control.set_show.set(true);
                set_show.set(false);
            }),
            is_file: false,
        },
        // P2P: Establish Branch (Placeholder)
        Command {
            id: "establish_branch".to_string(),
            title: (t::command_palette::establish_branch)(locale).to_string(),
            action: Callback::new(move |_| {
                let search_control = use_context::<crate::components::main_layout::SearchControl>()
                    .expect("search control");
                search_control.set_mode.set("@".to_string());
                search_control.set_show.set(true);
                set_show.set(false);
            }),
            is_file: false,
        },
        // P2P: Merge Peer
        Command {
            id: "merge_peer".to_string(),
            title: (t::command_palette::merge_peer)(locale).to_string(),
            action: Callback::new(move |_| {
                let branch = use_context::<crate::hooks::use_core::BranchContext>().expect("branch ctx");
                let sync = use_context::<crate::hooks::use_core::SyncMergeContext>().expect("sync ctx");
                if let Some(peer_id) = branch.active_branch.get_untracked() {
                    sync.on_merge_peer.run(peer_id.to_string());
                    set_show.set(false);
                } else {
                    leptos::logging::warn!("Cannot merge: No active peer selected.");
                    set_show.set(false);
                }
            }),
            is_file: false,
        },
    ];

    // Add AI Chat toggle command if ChatControl is available
    if let Some(chat_ctrl) = chat_control {
        commands.push(Command {
            id: "toggle_ai_chat".to_string(),
            title: (t::command_palette::toggle_ai_chat)(locale).to_string(),
            action: Callback::new(move |_| {
                let current = chat_ctrl.chat_visible.get_untracked();
                chat_ctrl.set_chat_visible.set(!current);
                set_show.set(false);
            }),
            is_file: false,
        });
    }

    commands
}

/// 基于查询字符串筛选命令。
pub fn filter_commands(query: &str, commands: Vec<Command>, max_results: usize) -> Vec<Command> {
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
