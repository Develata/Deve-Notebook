// apps\web\src\components\command_palette
//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 04_repository#repo-scope-runtime
//!
//! 命令面板的静态命令定义。

use super::types::Command;
use crate::components::main_layout::{ChatControl, SearchControl, SidebarControl};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::{BranchContext, SyncMergeContext};
use crate::i18n::{Locale, persist_locale_preference, t};
use leptos::prelude::*;

mod branch;
mod merge;
mod ngit;
mod remote_projection;
mod reserved;
#[cfg(test)]
mod tests;

use branch::establish_branch_command;
use merge::merge_peer_commands;
use ngit::{
    ngit_export_command, ngit_import_command, ngit_mirror_command, ngit_push_command,
    ngit_repair_command, ngit_status_command,
};
use remote_projection::remote_projection_commands;
use reserved::{ai_reserved_commands, source_control_reserved_commands};

/// 创建静态命令列表。
pub fn create_static_commands(
    locale: Locale,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
    locale_signal: RwSignal<Locale>,
    set_source_control_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Vec<Command> {
    // Try to get ChatControl from context at creation time
    let chat_control = use_context::<ChatControl>();
    let search_control = use_context::<SearchControl>();
    let branch_context = use_context::<BranchContext>();
    let sync_merge_context = use_context::<SyncMergeContext>();

    let mut commands = vec![
        // 打开文档命令 - 打开文档模态框
        Command::available(
            "open",
            (t::command_palette::open_document)(locale),
            move || {
                on_open.run(());
                // Do not close, as on_open re-purposes the search box
            },
        )
        .with_group((t::command_palette::group_navigation)(locale))
        .with_shortcut(t::command_palette::shortcut_ctrl_p())
        .with_enabled_when((t::command_palette::enabled_search_surface)(locale)),
        Command::available(
            "settings",
            (t::command_palette::open_settings)(locale),
            move || {
                on_settings.run(());
                set_show.set(false);
            },
        )
        .with_group((t::command_palette::group_settings)(locale))
        .with_enabled_when((t::command_palette::enabled_local_settings)(locale)),
        Command::available(
            "lang",
            (t::command_palette::toggle_language)(locale),
            move || {
                locale_signal.update(|locale| {
                    *locale = locale.toggle();
                    persist_locale_preference(*locale);
                });
                set_show.set(false);
            },
        )
        .with_group((t::command_palette::group_settings)(locale))
        .with_shortcut(t::command_palette::shortcut_ctrl_l())
        .with_enabled_when((t::command_palette::enabled_local_settings)(locale)),
        // P2P: Switch to Peer
        Command::available(
            "switch_peer",
            (t::command_palette::switch_peer)(locale),
            move || {
                if let Some(search_control) = search_control {
                    search_control.set_mode.set("@".to_string());
                    search_control.set_show.set(true);
                }
                set_show.set(false);
            },
        )
        .with_group((t::command_palette::group_peer)(locale))
        .with_shortcut(t::command_palette::shortcut_ctrl_shift_k())
        .with_enabled_when((t::command_palette::enabled_peer_surface)(locale)),
    ];

    if let Some(sidebar_control) = sidebar_control {
        commands.push(
            Command::available(
                "toggle_sidebar",
                (t::command_palette::toggle_sidebar)(locale),
                move || {
                    sidebar_control.toggle_visible();
                    set_show.set(false);
                },
            )
            .with_group((t::command_palette::group_layout)(locale))
            .with_shortcut(t::command_palette::shortcut_ctrl_b())
            .with_enabled_when((t::command_palette::enabled_local_settings)(locale)),
        );
    }

    commands.extend(source_control_reserved_commands(locale));
    commands.push(establish_branch_command(
        locale,
        set_show,
        set_source_control_notice,
        sidebar_control,
    ));
    commands.extend(merge_peer_commands(
        locale,
        set_show,
        branch_context,
        sync_merge_context,
    ));
    commands.extend(vec![
        ngit_status_command(locale, set_show, set_source_control_notice, sidebar_control),
        ngit_mirror_command(locale, set_show, set_source_control_notice, sidebar_control),
        ngit_export_command(locale, set_show, set_source_control_notice, sidebar_control),
        ngit_import_command(locale, set_show, set_source_control_notice, sidebar_control),
        ngit_push_command(locale, set_show, set_source_control_notice, sidebar_control),
        ngit_repair_command(locale, set_show, set_source_control_notice, sidebar_control),
    ]);
    commands.extend(remote_projection_commands(
        locale,
        set_show,
        sidebar_control,
    ));

    // Add AI Chat toggle command if ChatControl is available
    if let Some(chat_ctrl) = chat_control {
        commands.push(
            Command::available(
                "toggle_ai_chat",
                (t::command_palette::toggle_ai_chat)(locale),
                move || {
                    let current = chat_ctrl.chat_visible.get_untracked();
                    chat_ctrl.set_chat_visible.set(!current);
                    set_show.set(false);
                },
            )
            .with_group((t::command_palette::group_ai)(locale))
            .with_enabled_when((t::command_palette::enabled_local_ui)(locale)),
        );
    }
    commands.extend(ai_reserved_commands(locale));

    commands
}

/// 基于查询字符串筛选命令。
pub fn filter_commands(query: &str, commands: Vec<Command>, max_results: usize) -> Vec<Command> {
    let q = query.to_lowercase();
    let mut results = Vec::new();

    for cmd in commands {
        let shortcut = cmd.shortcut.as_deref().unwrap_or_default().to_lowercase();
        let detail = cmd.detail_text().to_lowercase();
        if q.is_empty()
            || cmd.title.to_lowercase().contains(&q)
            || cmd.id.contains(&q)
            || cmd.group.to_lowercase().contains(&q)
            || shortcut.contains(&q)
            || detail.contains(&q)
        {
            results.push(cmd);
        }
    }

    if results.len() > max_results {
        results.truncate(max_results);
    }

    results
}
