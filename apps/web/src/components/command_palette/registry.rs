// apps\web\src\components\command_palette
//! plan_ref:
//!   - 12_commands#command-palette-shortcuts
//!   - 06_repository#repo-scope-runtime
//!
//! 命令面板的静态命令定义。

use super::types::Command;
use crate::components::main_layout::{ChatControl, SearchControl};
use crate::hooks::use_core::{BranchContext, CoreState, SyncMergeContext};
use crate::i18n::{Locale, persist_locale_preference, t};
use leptos::prelude::*;

mod git;
mod merge;

use git::{git_import_command, git_push_command, git_repair_command};
use merge::merge_peer_command;

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
    let search_control = use_context::<SearchControl>();
    let branch_context = use_context::<BranchContext>();
    let sync_merge_context = use_context::<SyncMergeContext>();
    let core_state = use_context::<CoreState>();

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
                locale_signal.update(|locale| {
                    *locale = locale.toggle();
                    persist_locale_preference(*locale);
                });
                set_show.set(false);
            }),
            is_file: false,
        },
        // P2P: Switch to Peer
        Command {
            id: "switch_peer".to_string(),
            title: (t::command_palette::switch_peer)(locale).to_string(),
            action: Callback::new(move |_| {
                if let Some(search_control) = search_control {
                    search_control.set_mode.set("@".to_string());
                    search_control.set_show.set(true);
                }
                set_show.set(false);
            }),
            is_file: false,
        },
        // P2P: Establish Branch (Placeholder)
        Command {
            id: "establish_branch".to_string(),
            title: (t::command_palette::establish_branch)(locale).to_string(),
            action: Callback::new(move |_| {
                if let Some(search_control) = search_control {
                    search_control.set_mode.set("@".to_string());
                    search_control.set_show.set(true);
                }
                set_show.set(false);
            }),
            is_file: false,
        },
        merge_peer_command(
            locale,
            set_show,
            branch_context,
            sync_merge_context,
            core_state,
        ),
        git_import_command(locale, set_show),
        git_push_command(locale, set_show),
        git_repair_command(locale, set_show),
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

#[cfg(test)]
mod tests {
    use super::create_static_commands;
    use crate::i18n::Locale;
    use leptos::prelude::*;

    #[test]
    fn static_commands_include_git_bridge_notices() {
        let owner = leptos::reactive::owner::Owner::new();

        owner.with(|| {
            let (_, set_show) = signal(false);
            let locale = RwSignal::new(Locale::En);
            let commands = create_static_commands(
                Locale::En,
                Callback::new(|_| {}),
                Callback::new(|_| {}),
                set_show,
                locale,
            );
            let ids = commands
                .iter()
                .map(|command| command.id.as_str())
                .collect::<Vec<_>>();

            assert!(ids.contains(&"git_import_changes"));
            assert!(ids.contains(&"git_push_mirror"));
            assert!(ids.contains(&"git_repair_mirror"));
        });
    }
}
